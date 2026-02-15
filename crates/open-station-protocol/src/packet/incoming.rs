use crate::types::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PacketError {
    #[error("packet too short: expected at least {expected} bytes, got {actual}")]
    TooShort { expected: usize, actual: usize },
    #[error("invalid comm version: {0}")]
    InvalidVersion(u8),
    #[error("unknown tag: 0x{0:02x}")]
    UnknownTag(u8),
}

/// Parsed roboRIO -> DS UDP packet.
///
/// Packet format:
/// ```text
/// [seq_hi][seq_lo][0x01][status][trace][voltage_hi][voltage_lo][request_date][...tags]
/// ```
#[derive(Debug, Clone)]
pub struct RioPacket {
    pub sequence: u16,
    pub status: StatusFlags,
    pub trace: u8,
    pub voltage: BatteryVoltage,
    pub request_date: bool,
    pub tags: Vec<RioTag>,
}

/// Parsed telemetry tag from roboRIO.
#[derive(Debug, Clone)]
pub enum RioTag {
    JoystickOutput {
        outputs: u32,
        left_rumble: u16,
        right_rumble: u16,
    },
    DiskUsage(u32),
    CpuUsage(Vec<f32>),
    RamUsage(u32),
    PdpData(Vec<f32>),
    CanMetrics(CanMetrics),
    Unknown(u8, Vec<u8>),
}

/// Parse a complete roboRIO -> DS UDP packet.
pub fn parse_rio_packet(data: &[u8]) -> Result<RioPacket, PacketError> {
    if data.len() < 8 {
        return Err(PacketError::TooShort {
            expected: 8,
            actual: data.len(),
        });
    }

    let sequence = u16::from_be_bytes([data[0], data[1]]);

    let comm_version = data[2];
    if comm_version != 0x01 {
        return Err(PacketError::InvalidVersion(comm_version));
    }

    let status = StatusFlags::from_byte(data[3]);
    let trace = data[4];
    let voltage = BatteryVoltage::from_bytes(data[5], data[6]);
    let request_date = data[7] != 0;

    let tags = parse_tags(&data[8..]);

    Ok(RioPacket {
        sequence,
        status,
        trace,
        voltage,
        request_date,
        tags,
    })
}

/// Parse tagged telemetry data from the remaining bytes after the 8-byte header.
///
/// Each tag: `[size][tag_id][payload...]` where size includes the tag_id byte.
/// Unknown tags are stored as `RioTag::Unknown`.
fn parse_tags(mut data: &[u8]) -> Vec<RioTag> {
    let mut tags = Vec::new();

    while !data.is_empty() {
        // Need at least 2 bytes: size + tag_id
        if data.len() < 2 {
            break;
        }

        let size = data[0] as usize;
        let tag_id = data[1];

        // size includes the tag_id byte but not the size byte itself.
        // So total bytes consumed = 1 (size byte) + size (tag_id + payload).
        if data.len() < 1 + size {
            break;
        }

        let payload = &data[2..1 + size];

        let tag = match tag_id {
            0x01 => parse_joystick_output(payload),
            0x04 => parse_disk_usage(payload),
            0x05 => parse_cpu_usage(payload),
            0x06 => parse_ram_usage(payload),
            0x08 => parse_pdp_data(payload),
            0x0e => parse_can_metrics(payload),
            _ => RioTag::Unknown(tag_id, payload.to_vec()),
        };

        tags.push(tag);
        data = &data[1 + size..];
    }

    tags
}

/// Parse a joystick output tag (0x01).
///
/// Format: `[outputs(4)][left_rumble(2)][right_rumble(2)]`
fn parse_joystick_output(payload: &[u8]) -> RioTag {
    if payload.len() < 8 {
        return RioTag::JoystickOutput {
            outputs: 0,
            left_rumble: 0,
            right_rumble: 0,
        };
    }
    let outputs = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let left_rumble = u16::from_be_bytes([payload[4], payload[5]]);
    let right_rumble = u16::from_be_bytes([payload[6], payload[7]]);
    RioTag::JoystickOutput {
        outputs,
        left_rumble,
        right_rumble,
    }
}

/// Parse a disk usage tag (0x04).
///
/// Format: `[free_bytes(4)]` -- u32 big-endian
fn parse_disk_usage(payload: &[u8]) -> RioTag {
    if payload.len() < 4 {
        return RioTag::DiskUsage(0);
    }
    let free = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    RioTag::DiskUsage(free)
}

/// Parse a CPU usage tag (0x05).
///
/// Format: `[count][hi_1][lo_1][hi_2][lo_2]...`
/// Each pair is a fixed-point percentage: integer part + fractional/256.
fn parse_cpu_usage(payload: &[u8]) -> RioTag {
    if payload.is_empty() {
        return RioTag::CpuUsage(Vec::new());
    }
    let count = payload[0] as usize;
    let mut values = Vec::with_capacity(count);
    let pairs = &payload[1..];

    for i in 0..count {
        let offset = i * 2;
        if offset + 1 >= pairs.len() {
            break;
        }
        let hi = pairs[offset] as f32;
        let lo = pairs[offset + 1] as f32;
        values.push(hi + lo / 256.0);
    }

    RioTag::CpuUsage(values)
}

/// Parse a RAM usage tag (0x06).
///
/// Format: `[ram_bytes(4)]` -- u32 big-endian
fn parse_ram_usage(payload: &[u8]) -> RioTag {
    if payload.len() < 4 {
        return RioTag::RamUsage(0);
    }
    let ram = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    RioTag::RamUsage(ram)
}

/// Parse a PDP data tag (0x08).
///
/// 21 bytes encode 16 channels of 10-bit current values.
/// Every 5 bytes encodes 4 channels. Convert 10-bit value to amps: value * 0.125.
fn parse_pdp_data(payload: &[u8]) -> RioTag {
    let mut currents = Vec::with_capacity(16);

    // 4 groups of 5 bytes = 20 bytes for 16 channels
    for group in 0..4 {
        let offset = group * 5;
        if offset + 4 >= payload.len() {
            break;
        }
        let b = &payload[offset..offset + 5];

        let ch_a = ((b[0] as u16) << 2) | ((b[1] as u16) >> 6);
        let ch_b = (((b[1] as u16) & 0x3F) << 4) | ((b[2] as u16) >> 4);
        let ch_c = (((b[2] as u16) & 0x0F) << 6) | ((b[3] as u16) >> 2);
        let ch_d = (((b[3] as u16) & 0x03) << 8) | (b[4] as u16);

        currents.push(ch_a as f32 * 0.125);
        currents.push(ch_b as f32 * 0.125);
        currents.push(ch_c as f32 * 0.125);
        currents.push(ch_d as f32 * 0.125);
    }

    RioTag::PdpData(currents)
}

/// Parse a CAN metrics tag (0x0e).
///
/// Format: `[utilization_pct][bus_off(2)][tx_full(2)][rx_err][tx_err]`
fn parse_can_metrics(payload: &[u8]) -> RioTag {
    if payload.len() < 7 {
        return RioTag::CanMetrics(CanMetrics::default());
    }
    let utilization = payload[0] as f32;
    let bus_off = u16::from_be_bytes([payload[1], payload[2]]) as u32;
    let tx_full = u16::from_be_bytes([payload[3], payload[4]]) as u32;
    let rx_err = payload[5];
    let tx_err = payload[6];

    RioTag::CanMetrics(CanMetrics {
        utilization,
        bus_off_count: bus_off,
        tx_full_count: tx_full,
        rx_error_count: rx_err,
        tx_error_count: tx_err,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_packet() {
        // 8 bytes: seq=1, status=0, trace=0, voltage=12.5V, no date request
        let data = [0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        let packet = parse_rio_packet(&data).unwrap();
        assert_eq!(packet.sequence, 1);
        assert!(!packet.status.estop);
        assert!(!packet.status.enabled);
        assert!((packet.voltage.volts - 12.5).abs() < 0.01);
        assert!(!packet.request_date);
        assert!(packet.tags.is_empty());
    }

    #[test]
    fn test_parse_voltage() {
        let data = [0x00, 0x01, 0x01, 0x00, 0x00, 0x0D, 0x40, 0x00];
        let packet = parse_rio_packet(&data).unwrap();
        // 13 + 64/256 = 13.25V
        assert!((packet.voltage.volts - 13.25).abs() < 0.01);
    }

    #[test]
    fn test_parse_can_tag() {
        let mut data = vec![0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        // CAN tag: size=8, tag=0x0e, util=50, bus_off=0x0001, tx_full=0x0002, rx_err=3, tx_err=4
        data.extend_from_slice(&[0x08, 0x0e, 50, 0x00, 0x01, 0x00, 0x02, 3, 4]);
        let packet = parse_rio_packet(&data).unwrap();
        assert_eq!(packet.tags.len(), 1);
        match &packet.tags[0] {
            RioTag::CanMetrics(can) => {
                assert_eq!(can.utilization as u8, 50);
                assert_eq!(can.bus_off_count, 1);
                assert_eq!(can.tx_full_count, 2);
                assert_eq!(can.rx_error_count, 3);
                assert_eq!(can.tx_error_count, 4);
            }
            _ => panic!("expected CanMetrics tag"),
        }
    }

    #[test]
    fn test_parse_packet_too_short() {
        let data = [0x00, 0x01, 0x01];
        assert!(parse_rio_packet(&data).is_err());
    }

    #[test]
    fn test_parse_disk_tag() {
        let mut data = vec![0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        // Disk tag: size=5, tag=0x04, free=1048576 (0x00100000)
        data.extend_from_slice(&[0x05, 0x04, 0x00, 0x10, 0x00, 0x00]);
        let packet = parse_rio_packet(&data).unwrap();
        match &packet.tags[0] {
            RioTag::DiskUsage(free) => assert_eq!(*free, 1048576),
            _ => panic!("expected DiskUsage tag"),
        }
    }

    #[test]
    fn test_parse_invalid_version() {
        let data = [0x00, 0x01, 0x02, 0x00, 0x00, 0x0C, 0x80, 0x00];
        let err = parse_rio_packet(&data).unwrap_err();
        assert!(err.to_string().contains("invalid comm version"));
    }

    #[test]
    fn test_parse_status_flags() {
        // status byte: estop=1 (bit7), code_init=1 (bit4), brownout=1 (bit3),
        //              enabled=1 (bit2), mode=autonomous (0b10)
        let status_byte = 0b1001_1110;
        let data = [0x00, 0x01, 0x01, status_byte, 0x00, 0x0C, 0x80, 0x00];
        let packet = parse_rio_packet(&data).unwrap();
        assert!(packet.status.estop);
        assert!(packet.status.code_initializing);
        assert!(packet.status.brownout);
        assert!(packet.status.enabled);
        assert_eq!(packet.status.mode, Mode::Autonomous);
    }

    #[test]
    fn test_parse_request_date() {
        let data = [0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x01];
        let packet = parse_rio_packet(&data).unwrap();
        assert!(packet.request_date);
    }

    #[test]
    fn test_parse_joystick_output_tag() {
        let mut data = vec![0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        // JoystickOutput tag: size=9, tag=0x01, outputs=0x000000FF,
        // left_rumble=0x8000, right_rumble=0x4000
        data.extend_from_slice(&[
            0x09, 0x01, 0x00, 0x00, 0x00, 0xFF, 0x80, 0x00, 0x40, 0x00,
        ]);
        let packet = parse_rio_packet(&data).unwrap();
        assert_eq!(packet.tags.len(), 1);
        match &packet.tags[0] {
            RioTag::JoystickOutput {
                outputs,
                left_rumble,
                right_rumble,
            } => {
                assert_eq!(*outputs, 0xFF);
                assert_eq!(*left_rumble, 0x8000);
                assert_eq!(*right_rumble, 0x4000);
            }
            _ => panic!("expected JoystickOutput tag"),
        }
    }

    #[test]
    fn test_parse_cpu_usage_tag() {
        let mut data = vec![0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        // CPU tag: size=6, tag=0x05, count=2, cpu1=50.0 (0x32, 0x00), cpu2=75.5 (0x4B, 0x80)
        data.extend_from_slice(&[0x06, 0x05, 0x02, 0x32, 0x00, 0x4B, 0x80]);
        let packet = parse_rio_packet(&data).unwrap();
        match &packet.tags[0] {
            RioTag::CpuUsage(values) => {
                assert_eq!(values.len(), 2);
                assert!((values[0] - 50.0).abs() < 0.01);
                assert!((values[1] - 75.5).abs() < 0.01);
            }
            _ => panic!("expected CpuUsage tag"),
        }
    }

    #[test]
    fn test_parse_ram_usage_tag() {
        let mut data = vec![0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        // RAM tag: size=5, tag=0x06, ram=0x01000000 (16 MB)
        data.extend_from_slice(&[0x05, 0x06, 0x01, 0x00, 0x00, 0x00]);
        let packet = parse_rio_packet(&data).unwrap();
        match &packet.tags[0] {
            RioTag::RamUsage(ram) => assert_eq!(*ram, 0x01000000),
            _ => panic!("expected RamUsage tag"),
        }
    }

    #[test]
    fn test_parse_pdp_data_tag() {
        let mut data = vec![0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        // PDP tag: size=22, tag=0x08, followed by 21 bytes of packed data.
        // For simplicity, encode 4 channels in the first 5 bytes with known values:
        //   Channel 0 = 80 (raw 10-bit) -> 80 * 0.125 = 10.0 amps
        //   Channel 1 = 160 -> 160 * 0.125 = 20.0 amps
        //   Channel 2 = 0 -> 0.0 amps
        //   Channel 3 = 0 -> 0.0 amps
        //
        // Encoding channel 0 = 80 = 0b00_0101_0000:
        //   byte0 = 80 >> 2 = 20 = 0x14
        //   byte1 hi 2 bits = 80 & 3 = 0 -> (0 << 6)
        // Encoding channel 1 = 160 = 0b00_1010_0000:
        //   byte1 lo 6 bits = 160 >> 4 = 10 -> byte1 = (0 << 6) | 10 = 0x0A
        //   byte2 hi 4 bits = 160 & 0x0F = 0 -> (0 << 4)
        // Encoding channel 2 = 0:
        //   byte2 lo 4 bits = 0 >> 6 = 0 -> byte2 = (0 << 4) | 0 = 0x00
        //   byte3 hi 6 bits = 0 >> 2 = 0
        // Encoding channel 3 = 0:
        //   byte3 lo 2 bits = 0 & 3 = 0 -> byte3 = 0x00
        //   byte4 = 0 & 0xFF = 0x00
        let pdp_bytes: [u8; 21] = [
            0x14, 0x0A, 0x00, 0x00, 0x00, // group 0: channels 0-3
            0x00, 0x00, 0x00, 0x00, 0x00, // group 1: channels 4-7
            0x00, 0x00, 0x00, 0x00, 0x00, // group 2: channels 8-11
            0x00, 0x00, 0x00, 0x00, 0x00, // group 3: channels 12-15
            0x00, // padding/extra byte
        ];

        data.push(22); // size = 1 (tag) + 21 (payload)
        data.push(0x08); // tag
        data.extend_from_slice(&pdp_bytes);

        let packet = parse_rio_packet(&data).unwrap();
        match &packet.tags[0] {
            RioTag::PdpData(currents) => {
                assert_eq!(currents.len(), 16);
                assert!((currents[0] - 10.0).abs() < 0.01, "ch0: {}", currents[0]);
                assert!((currents[1] - 20.0).abs() < 0.01, "ch1: {}", currents[1]);
                assert!((currents[2] - 0.0).abs() < 0.01, "ch2: {}", currents[2]);
                assert!((currents[3] - 0.0).abs() < 0.01, "ch3: {}", currents[3]);
            }
            _ => panic!("expected PdpData tag"),
        }
    }

    #[test]
    fn test_parse_unknown_tag() {
        let mut data = vec![0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        // Unknown tag: size=3, tag=0xFF, payload=[0xAA, 0xBB]
        data.extend_from_slice(&[0x03, 0xFF, 0xAA, 0xBB]);
        let packet = parse_rio_packet(&data).unwrap();
        assert_eq!(packet.tags.len(), 1);
        match &packet.tags[0] {
            RioTag::Unknown(tag_id, payload) => {
                assert_eq!(*tag_id, 0xFF);
                assert_eq!(payload, &[0xAA, 0xBB]);
            }
            _ => panic!("expected Unknown tag"),
        }
    }

    #[test]
    fn test_parse_multiple_tags() {
        let mut data = vec![0x00, 0x01, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        // Disk tag
        data.extend_from_slice(&[0x05, 0x04, 0x00, 0x10, 0x00, 0x00]);
        // RAM tag
        data.extend_from_slice(&[0x05, 0x06, 0x00, 0x20, 0x00, 0x00]);
        let packet = parse_rio_packet(&data).unwrap();
        assert_eq!(packet.tags.len(), 2);
        match &packet.tags[0] {
            RioTag::DiskUsage(free) => assert_eq!(*free, 1048576),
            _ => panic!("expected DiskUsage tag"),
        }
        match &packet.tags[1] {
            RioTag::RamUsage(ram) => assert_eq!(*ram, 0x00200000),
            _ => panic!("expected RamUsage tag"),
        }
    }

    #[test]
    fn test_parse_sequence_number() {
        let data = [0x12, 0x34, 0x01, 0x00, 0x00, 0x0C, 0x80, 0x00];
        let packet = parse_rio_packet(&data).unwrap();
        assert_eq!(packet.sequence, 0x1234);
    }

    #[test]
    fn test_parse_trace_byte() {
        let data = [0x00, 0x01, 0x01, 0x00, 0xAB, 0x0C, 0x80, 0x00];
        let packet = parse_rio_packet(&data).unwrap();
        assert_eq!(packet.trace, 0xAB);
    }
}
