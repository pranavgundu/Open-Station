use gilrs::{Axis, Button, Gamepad};

pub fn map_axis(axis: Axis) -> Option<usize> {
    match axis {
        Axis::LeftStickX => Some(0),
        Axis::LeftStickY => Some(1),
        Axis::LeftZ => Some(2),
        Axis::RightZ => Some(3),
        Axis::RightStickX => Some(4),
        Axis::RightStickY => Some(5),
        _ => None,
    }
}

pub fn map_button(button: Button) -> Option<usize> {
    match button {
        Button::South => Some(0),
        Button::East => Some(1),
        Button::North => Some(2),
        Button::West => Some(3),
        Button::LeftTrigger => Some(4),
        Button::RightTrigger => Some(5),
        Button::Select => Some(6),
        Button::Start => Some(7),
        Button::LeftThumb => Some(8),
        Button::RightThumb => Some(9),
        _ => None,
    }
}

pub fn read_dpad_pov(gamepad: &Gamepad) -> i16 {
    let up = gamepad.is_pressed(Button::DPadUp);
    let down = gamepad.is_pressed(Button::DPadDown);
    let left = gamepad.is_pressed(Button::DPadLeft);
    let right = gamepad.is_pressed(Button::DPadRight);

    match (up, right, down, left) {
        (true, false, false, false) => 0,
        (true, true, false, false) => 45,
        (false, true, false, false) => 90,
        (false, true, true, false) => 135,
        (false, false, true, false) => 180,
        (false, false, true, true) => 225,
        (false, false, false, true) => 270,
        (true, false, false, true) => 315,
        _ => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axis_mapping() {
        assert_eq!(map_axis(Axis::LeftStickX), Some(0));
        assert_eq!(map_axis(Axis::LeftStickY), Some(1));
        assert_eq!(map_axis(Axis::LeftZ), Some(2));
        assert_eq!(map_axis(Axis::RightZ), Some(3));
        assert_eq!(map_axis(Axis::RightStickX), Some(4));
        assert_eq!(map_axis(Axis::RightStickY), Some(5));
    }

    #[test]
    fn test_button_mapping() {
        assert_eq!(map_button(Button::South), Some(0));
        assert_eq!(map_button(Button::East), Some(1));
        assert_eq!(map_button(Button::North), Some(2));
        assert_eq!(map_button(Button::West), Some(3));
        assert_eq!(map_button(Button::LeftTrigger), Some(4));
        assert_eq!(map_button(Button::RightTrigger), Some(5));
        assert_eq!(map_button(Button::Select), Some(6));
        assert_eq!(map_button(Button::Start), Some(7));
        assert_eq!(map_button(Button::LeftThumb), Some(8));
        assert_eq!(map_button(Button::RightThumb), Some(9));
    }
}
