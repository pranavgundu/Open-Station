use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Teleop,
    Test,
    Autonomous,
}

impl Mode {
    pub fn to_bits(self) -> u8 {
        match self {
            Mode::Teleop => 0b00,
            Mode::Test => 0b01,
            Mode::Autonomous => 0b10,
        }
    }

    pub fn from_bits(bits: u8) -> Option<Mode> {
        match bits & 0b11 {
            0b00 => Some(Mode::Teleop),
            0b01 => Some(Mode::Test),
            0b10 => Some(Mode::Autonomous),
            _ => None,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Teleop => write!(f, "Teleoperated"),
            Mode::Test => write!(f, "Test"),
            Mode::Autonomous => write!(f, "Autonomous"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllianceColor {
    Red,
    Blue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alliance {
    pub color: AllianceColor,
    pub station: u8,
}

impl Alliance {
    pub fn new(color: AllianceColor, station: u8) -> Self {
        assert!(
            (1..=3).contains(&station),
            "Station must be 1, 2, or 3, got {station}"
        );
        Alliance { color, station }
    }

    pub fn to_byte(self) -> u8 {
        let base = match self.color {
            AllianceColor::Red => 0,
            AllianceColor::Blue => 3,
        };
        base + (self.station - 1)
    }

    pub fn from_byte(byte: u8) -> Option<Alliance> {
        match byte {
            0 => Some(Alliance::new(AllianceColor::Red, 1)),
            1 => Some(Alliance::new(AllianceColor::Red, 2)),
            2 => Some(Alliance::new(AllianceColor::Red, 3)),
            3 => Some(Alliance::new(AllianceColor::Blue, 1)),
            4 => Some(Alliance::new(AllianceColor::Blue, 2)),
            5 => Some(Alliance::new(AllianceColor::Blue, 3)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlFlags {
    pub estop: bool,
    pub fms_connected: bool,
    pub enabled: bool,
    pub mode: Mode,
}

impl ControlFlags {
    pub fn to_byte(self) -> u8 {
        let mut byte = 0u8;
        if self.estop {
            byte |= 1 << 7;
        }
        if self.fms_connected {
            byte |= 1 << 3;
        }
        if self.enabled {
            byte |= 1 << 2;
        }
        byte |= self.mode.to_bits();
        byte
    }

    pub fn from_byte(byte: u8) -> ControlFlags {
        ControlFlags {
            estop: (byte >> 7) & 1 != 0,
            fms_connected: (byte >> 3) & 1 != 0,
            enabled: (byte >> 2) & 1 != 0,
            mode: Mode::from_bits(byte & 0b11).unwrap_or(Mode::Teleop),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequestFlags {
    pub reboot_roborio: bool,
    pub restart_code: bool,
}

impl RequestFlags {
    pub fn to_byte(self) -> u8 {
        let mut byte = 0u8;
        if self.reboot_roborio {
            byte |= 1 << 3;
        }
        if self.restart_code {
            byte |= 1 << 2;
        }
        byte
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusFlags {
    pub estop: bool,
    pub code_initializing: bool,
    pub brownout: bool,
    pub enabled: bool,
    pub mode: Mode,
}

impl StatusFlags {
    pub fn from_byte(byte: u8) -> StatusFlags {
        StatusFlags {
            estop: (byte >> 7) & 1 != 0,
            code_initializing: (byte >> 4) & 1 != 0,
            brownout: (byte >> 3) & 1 != 0,
            enabled: (byte >> 2) & 1 != 0,
            mode: Mode::from_bits(byte & 0b11).unwrap_or(Mode::Teleop),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryVoltage {
    pub volts: f32,
}

impl BatteryVoltage {
    pub fn from_bytes(high: u8, low: u8) -> BatteryVoltage {
        BatteryVoltage {
            volts: high as f32 + low as f32 / 256.0,
        }
    }

    pub fn to_bytes(self) -> (u8, u8) {
        let high = self.volts.floor() as u8;
        let frac = (self.volts - high as f32) * 256.0;
        (high, frac.round() as u8)
    }
}

#[derive(Debug, Clone, Default)]
pub struct JoystickData {
    pub axes: Vec<i8>,
    pub buttons: Vec<bool>,
    pub povs: Vec<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RumbleOutput {
    pub left: f32,
    pub right: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanMetrics {
    pub utilization: f32,
    pub bus_off_count: u32,
    pub tx_full_count: u32,
    pub rx_error_count: u8,
    pub tx_error_count: u8,
}

impl Default for CanMetrics {
    fn default() -> Self {
        CanMetrics {
            utilization: 0.0,
            bus_off_count: 0,
            tx_full_count: 0,
            rx_error_count: 0,
            tx_error_count: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TelemetryData {
    pub can: CanMetrics,
    pub pdp_currents: Vec<f32>,
    pub cpu_usage: Vec<f32>,
    pub ram_usage: u32,
    pub disk_free: u32,
}

#[derive(Debug, Clone)]
pub struct RobotState {
    pub connected: bool,
    pub code_running: bool,
    pub voltage: BatteryVoltage,
    pub status: StatusFlags,
    pub telemetry: TelemetryData,
    pub sequence: u16,
    pub trip_time_ms: f64,
    pub lost_packets: u32,
}

#[derive(Debug, Clone)]
pub enum TcpMessage {
    Stdout(String),
    ErrorReport {
        timestamp: f64,
        sequence: u16,
        error_code: i32,
        is_error: bool,
        details: String,
        location: String,
        call_stack: String,
    },
    VersionInfo {
        device_type: u8,
        device_id: u8,
        name: String,
        version: String,
    },
    Message(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_round_trip() {
        for mode in [Mode::Teleop, Mode::Test, Mode::Autonomous] {
            let bits = mode.to_bits();
            let decoded = Mode::from_bits(bits).expect("valid bits should decode");
            assert_eq!(mode, decoded, "round-trip failed for {mode}");
        }
        // Invalid bit pattern 0b11 must return None
        assert_eq!(Mode::from_bits(0b11), None);
    }

    #[test]
    fn test_alliance_round_trip() {
        let combinations = [
            (AllianceColor::Red, 1, 0u8),
            (AllianceColor::Red, 2, 1),
            (AllianceColor::Red, 3, 2),
            (AllianceColor::Blue, 1, 3),
            (AllianceColor::Blue, 2, 4),
            (AllianceColor::Blue, 3, 5),
        ];

        for (color, station, expected_byte) in combinations {
            let alliance = Alliance::new(color, station);
            let byte = alliance.to_byte();
            assert_eq!(byte, expected_byte, "encode failed for {color:?} {station}");

            let decoded = Alliance::from_byte(byte).expect("valid byte should decode");
            assert_eq!(decoded, alliance, "round-trip failed for byte {byte}");
        }
    }

    #[test]
    fn test_control_flags_encoding() {
        let flags = ControlFlags {
            estop: true,
            fms_connected: false,
            enabled: true,
            mode: Mode::Autonomous,
        };

        let byte = flags.to_byte();
        assert_eq!(byte, 0b1000_0110);

        let decoded = ControlFlags::from_byte(byte);
        assert_eq!(decoded, flags);
    }

    #[test]
    fn test_battery_voltage_encoding() {
        let voltage = BatteryVoltage { volts: 12.5 };
        let (high, low) = voltage.to_bytes();
        let decoded = BatteryVoltage::from_bytes(high, low);

        assert!(
            (decoded.volts - 12.5).abs() < 0.01,
            "expected ~12.5, got {}",
            decoded.volts
        );
    }

    #[test]
    fn test_status_flags_brownout() {
        let flags = StatusFlags::from_byte(0b0000_1000);
        assert!(!flags.estop);
        assert!(!flags.code_initializing);
        assert!(flags.brownout);
        assert!(!flags.enabled);
        assert_eq!(flags.mode, Mode::Teleop);
    }

    #[test]
    fn test_request_flags_encoding() {
        let flags = RequestFlags {
            reboot_roborio: true,
            restart_code: true,
        };
        assert_eq!(flags.to_byte(), 0b0000_1100);
    }

    #[test]
    fn test_alliance_invalid_byte() {
        assert_eq!(Alliance::from_byte(6), None);
        assert_eq!(Alliance::from_byte(255), None);
    }
}
