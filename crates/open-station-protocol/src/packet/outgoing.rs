use crate::types::*;

/// Build a complete DS->roboRIO UDP packet.
///
/// Packet format:
/// ```text
/// [seq_hi][seq_lo][0x01][control][request][alliance][...tags]
/// ```
///
/// The 6-byte header is followed by zero or more tagged data sections
/// (joystick tags, datetime tag, timezone tag, etc.).
pub fn build_ds_packet(
    sequence: u16,
    control: &ControlFlags,
    request: &RequestFlags,
    alliance: &Alliance,
    joysticks: &[JoystickData],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);

    // 6-byte header
    buf.push((sequence >> 8) as u8); // seq_hi
    buf.push((sequence & 0xFF) as u8); // seq_lo
    buf.push(0x01); // comm version
    buf.push(control.to_byte());
    buf.push(request.to_byte());
    buf.push(alliance.to_byte());

    // Append joystick tags (up to 6)
    for js in joysticks.iter().take(6) {
        append_joystick_tag(&mut buf, js);
    }

    buf
}

/// Append a joystick data tag (0x0c) to the buffer.
///
/// Format:
/// ```text
/// [size][0x0c][axis_count][axis_values...][button_count][button_bytes...][pov_count][pov_hi][pov_lo]...
/// ```
///
/// `size` is the total size of the tag data INCLUDING the tag byte itself.
pub fn append_joystick_tag(buf: &mut Vec<u8>, joystick: &JoystickData) {
    let axis_count = joystick.axes.len() as u8;
    let button_count = joystick.buttons.len() as u8;
    let button_byte_count = (button_count as usize + 7) / 8;
    let pov_count = joystick.povs.len() as u8;

    // size = tag(1) + axis_count(1) + axes(N) + button_count(1) + button_bytes(M) + pov_count(1) + povs(P*2)
    let size: u8 = 1 + 1 + axis_count + 1 + button_byte_count as u8 + 1 + pov_count * 2;

    buf.push(size);
    buf.push(0x0c); // joystick tag

    // Axes
    buf.push(axis_count);
    for &axis in &joystick.axes {
        buf.push(axis as u8);
    }

    // Buttons - packed bits, LSB first
    buf.push(button_count);
    for byte_idx in 0..button_byte_count {
        let mut byte = 0u8;
        for bit in 0..8 {
            let button_idx = byte_idx * 8 + bit;
            if button_idx < joystick.buttons.len() && joystick.buttons[button_idx] {
                byte |= 1 << bit;
            }
        }
        buf.push(byte);
    }

    // POVs
    buf.push(pov_count);
    for &pov in &joystick.povs {
        buf.push((pov >> 8) as u8);
        buf.push((pov & 0xFF) as u8);
    }
}

/// Append a datetime tag (0x0f) with current UTC time.
///
/// Format:
/// ```text
/// [0x0b][0x0f][us3][us2][us1][us0][sec][min][hr][day][month][year]
/// ```
///
/// - `0x0b`: fixed size (11 bytes)
/// - microseconds: u32 big-endian
/// - day: 1-31
/// - month: 0-11 (January = 0)
/// - year: year - 1900
pub fn append_datetime_tag(buf: &mut Vec<u8>) {
    use chrono::Datelike;
    use chrono::Timelike;
    use chrono::Utc;

    let now = Utc::now();

    let microseconds = now.nanosecond() / 1000;
    let seconds = now.second() as u8;
    let minutes = now.minute() as u8;
    let hours = now.hour() as u8;
    let day = now.day() as u8;
    let month = (now.month0()) as u8; // month0 returns 0-11
    let year = (now.year() - 1900) as u8;

    buf.push(0x0b); // size: 11 bytes
    buf.push(0x0f); // datetime tag

    // Microseconds as u32 big-endian
    buf.push((microseconds >> 24) as u8);
    buf.push((microseconds >> 16) as u8);
    buf.push((microseconds >> 8) as u8);
    buf.push((microseconds & 0xFF) as u8);

    buf.push(seconds);
    buf.push(minutes);
    buf.push(hours);
    buf.push(day);
    buf.push(month);
    buf.push(year);
}

/// Append a timezone tag (0x10) with the given timezone string.
///
/// Format:
/// ```text
/// [size][0x10][timezone_string_bytes...]
/// ```
///
/// `size` = 1 + timezone_string.len()
pub fn append_timezone_tag(buf: &mut Vec<u8>, tz: &str) {
    let size = (1 + tz.len()) as u8;
    buf.push(size);
    buf.push(0x10); // timezone tag
    buf.extend_from_slice(tz.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_encoding() {
        // Sequence 0x1234, teleop disabled, Red1
        let packet = build_ds_packet(
            0x1234,
            &ControlFlags::default(),
            &RequestFlags::default(),
            &Alliance::new(AllianceColor::Red, 1),
            &[],
        );
        assert_eq!(packet[0], 0x12); // seq hi
        assert_eq!(packet[1], 0x34); // seq lo
        assert_eq!(packet[2], 0x01); // comm version
        assert_eq!(packet[3], 0x00); // control: teleop, disabled, no estop
        assert_eq!(packet[4], 0x00); // request: nothing
        assert_eq!(packet[5], 0x00); // alliance: Red1
    }

    #[test]
    fn test_joystick_tag_empty() {
        // Joystick with 0 axes, 0 buttons, 0 POVs
        let mut buf = Vec::new();
        let js = JoystickData::default();
        append_joystick_tag(&mut buf, &js);
        // size byte + 0x0c + axis_count(0) + button_count(0) + pov_count(0)
        assert_eq!(buf.len(), 5);
        assert_eq!(buf[1], 0x0c); // tag
    }

    #[test]
    fn test_joystick_tag_full() {
        // 6 axes, 12 buttons, 1 POV
        let js = JoystickData {
            axes: vec![0, 127, -128, 64, -64, 0],
            buttons: vec![
                true, false, true, false, false, false, false, false, true, false, false, true,
            ],
            povs: vec![90],
        };
        let mut buf = Vec::new();
        append_joystick_tag(&mut buf, &js);
        // Verify structure is correct
        assert_eq!(buf[1], 0x0c);
        assert_eq!(buf[2], 6); // 6 axes
                               // axes at indices 3-8
        assert_eq!(buf[3], 0i8 as u8);
        assert_eq!(buf[4], 127i8 as u8);
        assert_eq!(buf[5], (-128i8) as u8);
        assert_eq!(buf[9], 12); // 12 buttons
                                // 12 buttons = 2 bytes
                                // buttons: [true, false, true, false, false, false, false, false, true, false, false, true]
                                // byte 0: bits 0-7 = 1,0,1,0,0,0,0,0 = 0b00000101 = 0x05
                                // byte 1: bits 0-3 = 1,0,0,1 = 0b00001001 = 0x09
        assert_eq!(buf[10], 0x05);
        assert_eq!(buf[11], 0x09);
        assert_eq!(buf[12], 1); // 1 POV
                                // POV 90 = 0x005A big-endian
        assert_eq!(buf[13], 0x00);
        assert_eq!(buf[14], 0x5A);
    }

    #[test]
    fn test_button_packing() {
        // Only buttons 0 and 2 set (1-indexed: buttons 1 and 3)
        let js = JoystickData {
            axes: vec![],
            buttons: vec![true, false, true],
            povs: vec![],
        };
        let mut buf = Vec::new();
        append_joystick_tag(&mut buf, &js);
        assert_eq!(buf[2], 0); // 0 axes
        assert_eq!(buf[3], 3); // 3 buttons
        assert_eq!(buf[4], 0b00000101); // buttons 0 and 2 set
    }

    #[test]
    fn test_timezone_tag() {
        let mut buf = Vec::new();
        append_timezone_tag(&mut buf, "America/New_York");
        assert_eq!(buf[0], 17); // size: 1 + 16
        assert_eq!(buf[1], 0x10); // tag
        assert_eq!(&buf[2..], b"America/New_York");
    }
}
