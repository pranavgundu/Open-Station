use crate::types::TcpMessage;

pub struct TcpFrameReader {
    buffer: Vec<u8>,
}

impl TcpFrameReader {
    pub fn new() -> Self {
        TcpFrameReader { buffer: Vec::new() }
    }

    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn next_frame(&mut self) -> Option<(u8, Vec<u8>)> {
        if self.buffer.len() < 3 {
            return None;
        }

        let size_hi = self.buffer[0];
        let size_lo = self.buffer[1];
        let size = u16::from_be_bytes([size_hi, size_lo]) as usize;

        if self.buffer.len() < 2 + size {
            return None;
        }

        let tag = self.buffer[2];

        let payload_len = size - 1;
        let payload = self.buffer[3..3 + payload_len].to_vec();

        self.buffer.drain(0..2 + size);

        Some((tag, payload))
    }
}

impl Default for TcpFrameReader {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_tcp_message(tag: u8, payload: &[u8]) -> Option<TcpMessage> {
    match tag {
        0x00 => {
            let message = String::from_utf8(payload.to_vec()).ok()?;
            Some(TcpMessage::Message(message))
        }
        0x0a => {
            if payload.len() < 4 {
                return None;
            }

            let device_type = payload[0];
            let device_id = payload[1];
            let name_len = payload[2] as usize;

            if payload.len() < 3 + name_len + 1 {
                return None;
            }

            let name = String::from_utf8(payload[3..3 + name_len].to_vec()).ok()?;
            let version_len = payload[3 + name_len] as usize;

            if payload.len() < 3 + name_len + 1 + version_len {
                return None;
            }

            let version =
                String::from_utf8(payload[4 + name_len..4 + name_len + version_len].to_vec())
                    .ok()?;

            Some(TcpMessage::VersionInfo {
                device_type,
                device_id,
                name,
                version,
            })
        }
        0x0b => {
            if payload.len() < 8 + 2 + 4 + 2 + 2 {
                return None;
            }

            let timestamp_bytes: [u8; 8] = payload[0..8].try_into().ok()?;
            let timestamp = f64::from_be_bytes(timestamp_bytes);

            let sequence = u16::from_be_bytes([payload[8], payload[9]]);
            let error_code =
                i32::from_be_bytes([payload[10], payload[11], payload[12], payload[13]]);
            let flags = u16::from_be_bytes([payload[14], payload[15]]);
            let is_error = (flags & 1) != 0;

            let details_len = u16::from_be_bytes([payload[16], payload[17]]) as usize;
            if payload.len() < 18 + details_len + 2 {
                return None;
            }

            let details = String::from_utf8(payload[18..18 + details_len].to_vec()).ok()?;

            let location_len_offset = 18 + details_len;
            let location_len = u16::from_be_bytes([
                payload[location_len_offset],
                payload[location_len_offset + 1],
            ]) as usize;

            if payload.len() < location_len_offset + 2 + location_len + 2 {
                return None;
            }

            let location = String::from_utf8(
                payload[location_len_offset + 2..location_len_offset + 2 + location_len].to_vec(),
            )
            .ok()?;

            let call_stack_len_offset = location_len_offset + 2 + location_len;
            let call_stack_len = u16::from_be_bytes([
                payload[call_stack_len_offset],
                payload[call_stack_len_offset + 1],
            ]) as usize;

            if payload.len() < call_stack_len_offset + 2 + call_stack_len {
                return None;
            }

            let call_stack = String::from_utf8(
                payload[call_stack_len_offset + 2..call_stack_len_offset + 2 + call_stack_len]
                    .to_vec(),
            )
            .ok()?;

            Some(TcpMessage::ErrorReport {
                timestamp,
                sequence,
                error_code,
                is_error,
                details,
                location,
                call_stack,
            })
        }
        0x0c => {
            let stdout = String::from_utf8(payload.to_vec()).ok()?;
            Some(TcpMessage::Stdout(stdout))
        }
        _ => None,
    }
}

pub fn encode_tcp_frame(tag: u8, payload: &[u8]) -> Vec<u8> {
    let size = 1 + payload.len();
    let size_bytes = (size as u16).to_be_bytes();

    let mut frame = Vec::with_capacity(2 + size);
    frame.extend_from_slice(&size_bytes);
    frame.push(tag);
    frame.extend_from_slice(payload);
    frame
}

pub fn build_game_data_frame(data: &str) -> Vec<u8> {
    encode_tcp_frame(0x0e, data.as_bytes())
}

pub fn build_joystick_descriptor_frame(
    slot: u8,
    name: &str,
    axis_count: u8,
    button_count: u8,
    pov_count: u8,
) -> Vec<u8> {
    let mut payload = vec![slot, 0, 0, name.len() as u8];
    payload.extend_from_slice(name.as_bytes());
    payload.push(axis_count);
    payload.push(button_count);
    payload.push(pov_count);

    encode_tcp_frame(0x02, &payload)
}

pub fn build_match_info_frame(match_name: &str, match_type: u8) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(match_name.len() as u8);
    payload.extend_from_slice(match_name.as_bytes());
    payload.push(match_type);

    encode_tcp_frame(0x07, &payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_frame() {
        let frame = encode_tcp_frame(0x0c, b"Hello Robot");
        assert_eq!(frame[0], 0x00);
        assert_eq!(frame[1], 0x0C);
        assert_eq!(frame[2], 0x0c);
        assert_eq!(&frame[3..], b"Hello Robot");
    }

    #[test]
    fn test_frame_reader_complete() {
        let mut reader = TcpFrameReader::new();
        let frame = encode_tcp_frame(0x0c, b"test");
        reader.feed(&frame);
        let (tag, payload) = reader.next_frame().unwrap();
        assert_eq!(tag, 0x0c);
        assert_eq!(payload, b"test");
        assert!(reader.next_frame().is_none());
    }

    #[test]
    fn test_frame_reader_partial() {
        let mut reader = TcpFrameReader::new();
        let frame = encode_tcp_frame(0x0c, b"test");
        for &byte in &frame {
            reader.feed(&[byte]);
        }
        let (tag, payload) = reader.next_frame().unwrap();
        assert_eq!(tag, 0x0c);
        assert_eq!(payload, b"test");
    }

    #[test]
    fn test_frame_reader_multiple() {
        let mut reader = TcpFrameReader::new();
        let frame1 = encode_tcp_frame(0x0c, b"first");
        let frame2 = encode_tcp_frame(0x00, b"second");
        let mut combined = frame1;
        combined.extend_from_slice(&frame2);
        reader.feed(&combined);

        let (tag1, p1) = reader.next_frame().unwrap();
        assert_eq!(tag1, 0x0c);
        assert_eq!(p1, b"first");

        let (tag2, p2) = reader.next_frame().unwrap();
        assert_eq!(tag2, 0x00);
        assert_eq!(p2, b"second");
    }

    #[test]
    fn test_parse_stdout() {
        let msg = parse_tcp_message(0x0c, b"Robot output").unwrap();
        match msg {
            TcpMessage::Stdout(s) => assert_eq!(s, "Robot output"),
            _ => panic!("expected Stdout"),
        }
    }

    #[test]
    fn test_parse_message() {
        let msg = parse_tcp_message(0x00, b"DS message").unwrap();
        match msg {
            TcpMessage::Message(s) => assert_eq!(s, "DS message"),
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn test_game_data_frame() {
        let frame = build_game_data_frame("LRL");
        assert_eq!(frame[2], 0x0e);
        assert_eq!(&frame[3..], b"LRL");
    }

    #[test]
    fn test_joystick_descriptor_frame() {
        let frame = build_joystick_descriptor_frame(0, "Gamepad", 6, 12, 1);
        assert_eq!(frame[2], 0x02);
        assert_eq!(frame[3], 0);
    }
}
