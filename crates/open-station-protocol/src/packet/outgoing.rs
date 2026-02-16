use crate::types::*;

pub fn build_ds_packet(
    sequence: u16,
    control: &ControlFlags,
    request: &RequestFlags,
    alliance: &Alliance,
    joysticks: &[JoystickData],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);

    buf.push((sequence >> 8) as u8);
    buf.push((sequence & 0xFF) as u8);
    buf.push(0x01);
    buf.push(control.to_byte());
    buf.push(request.to_byte());
    buf.push(alliance.to_byte());

    for js in joysticks.iter().take(6) {
        append_joystick_tag(&mut buf, js);
    }

    buf
}

pub fn append_joystick_tag(buf: &mut Vec<u8>, joystick: &JoystickData) {
    let axis_count = joystick.axes.len() as u8;
    let button_count = joystick.buttons.len() as u8;
    let button_byte_count = (button_count as usize).div_ceil(8);
    let pov_count = joystick.povs.len() as u8;

    let size: u8 = 1 + 1 + axis_count + 1 + button_byte_count as u8 + 1 + pov_count * 2;

    buf.push(size);
    buf.push(0x0c);

    buf.push(axis_count);
    for &axis in &joystick.axes {
        buf.push(axis as u8);
    }

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

    buf.push(pov_count);
    for &pov in &joystick.povs {
        buf.push((pov >> 8) as u8);
        buf.push((pov & 0xFF) as u8);
    }
}

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
    let month = (now.month0()) as u8;
    let year = (now.year() - 1900) as u8;

    buf.push(0x0b);
    buf.push(0x0f);

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

pub fn append_timezone_tag(buf: &mut Vec<u8>, tz: &str) {
    let size = (1 + tz.len()) as u8;
    buf.push(size);
    buf.push(0x10);
    buf.extend_from_slice(tz.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_encoding() {
        let packet = build_ds_packet(
            0x1234,
            &ControlFlags::default(),
            &RequestFlags::default(),
            &Alliance::new(AllianceColor::Red, 1),
            &[],
        );
        assert_eq!(packet[0], 0x12);
        assert_eq!(packet[1], 0x34);
        assert_eq!(packet[2], 0x01);
        assert_eq!(packet[3], 0x00);
        assert_eq!(packet[4], 0x00);
        assert_eq!(packet[5], 0x00);
    }

    #[test]
    fn test_joystick_tag_empty() {
        let mut buf = Vec::new();
        let js = JoystickData::default();
        append_joystick_tag(&mut buf, &js);
        assert_eq!(buf.len(), 5);
        assert_eq!(buf[1], 0x0c);
    }

    #[test]
    fn test_joystick_tag_full() {
        let js = JoystickData {
            axes: vec![0, 127, -128, 64, -64, 0],
            buttons: vec![
                true, false, true, false, false, false, false, false, true, false, false, true,
            ],
            povs: vec![90],
        };
        let mut buf = Vec::new();
        append_joystick_tag(&mut buf, &js);
        assert_eq!(buf[1], 0x0c);
        assert_eq!(buf[2], 6);
        assert_eq!(buf[3], 0i8 as u8);
        assert_eq!(buf[4], 127i8 as u8);
        assert_eq!(buf[5], (-128i8) as u8);
        assert_eq!(buf[9], 12);
        assert_eq!(buf[10], 0x05);
        assert_eq!(buf[11], 0x09);
        assert_eq!(buf[12], 1);
        assert_eq!(buf[13], 0x00);
        assert_eq!(buf[14], 0x5A);
    }

    #[test]
    fn test_button_packing() {
        let js = JoystickData {
            axes: vec![],
            buttons: vec![true, false, true],
            povs: vec![],
        };
        let mut buf = Vec::new();
        append_joystick_tag(&mut buf, &js);
        assert_eq!(buf[2], 0);
        assert_eq!(buf[3], 3);
        assert_eq!(buf[4], 0b00000101);
    }

    #[test]
    fn test_timezone_tag() {
        let mut buf = Vec::new();
        append_timezone_tag(&mut buf, "America/New_York");
        assert_eq!(buf[0], 17);
        assert_eq!(buf[1], 0x10);
        assert_eq!(&buf[2..], b"America/New_York");
    }
}
