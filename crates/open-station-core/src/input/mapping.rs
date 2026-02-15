use gilrs::{Axis, Button, Gamepad};

/// Map a gilrs axis to FRC axis index (0-5)
/// LeftStickX → 0, LeftStickY → 1, LeftZ (left trigger) → 2,
/// RightZ (right trigger) → 3, RightStickX → 4, RightStickY → 5
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

/// Map a gilrs button to FRC button index (0-based, FRC is 1-based so add 1 for display)
/// South(A)→0, East(B)→1, North(Y)→2, West(X)→3,
/// LeftTrigger(LB)→4, RightTrigger(RB)→5,
/// Select→6, Start→7, LeftThumb(LS)→8, RightThumb(RS)→9
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

/// Read D-pad state from gamepad and return POV angle
/// Up=0, Right=90, Down=180, Left=270, not pressed=-1
/// Also handles diagonals: UpRight=45, DownRight=135, DownLeft=225, UpLeft=315
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

    // Note: D-pad tests need a Gamepad instance which requires hardware.
    // The mapping logic is simple enough to verify by inspection.
}
