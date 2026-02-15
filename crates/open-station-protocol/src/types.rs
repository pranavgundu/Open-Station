use std::fmt;

// ---------------------------------------------------------------------------
// 1. Mode
// ---------------------------------------------------------------------------

/// The three operating modes of an FRC robot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Teleop,
    Test,
    Autonomous,
}

impl Mode {
    /// Encode the mode into a 2-bit value (bits 0-1).
    pub fn to_bits(self) -> u8 {
        match self {
            Mode::Teleop => 0b00,
            Mode::Test => 0b01,
            Mode::Autonomous => 0b10,
        }
    }

    /// Decode a 2-bit value into a `Mode`, returning `None` for invalid values.
    pub fn from_bits(bits: u8) -> Option<Mode> {
        match bits & 0b11 {
            0b00 => Some(Mode::Teleop),
            0b01 => Some(Mode::Test),
            0b10 => Some(Mode::Autonomous),
            _ => None,
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Teleop
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

// ---------------------------------------------------------------------------
// 2. AllianceColor
// ---------------------------------------------------------------------------

/// Red or Blue alliance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllianceColor {
    Red,
    Blue,
}

// ---------------------------------------------------------------------------
// 3. Alliance
// ---------------------------------------------------------------------------

/// An alliance position: color + station number (1-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alliance {
    pub color: AllianceColor,
    pub station: u8,
}

impl Alliance {
    /// Create a new `Alliance`. Panics if `station` is not 1, 2, or 3.
    pub fn new(color: AllianceColor, station: u8) -> Self {
        assert!(
            (1..=3).contains(&station),
            "Station must be 1, 2, or 3, got {station}"
        );
        Alliance { color, station }
    }

    /// Encode as a single byte: Red1=0, Red2=1, Red3=2, Blue1=3, Blue2=4, Blue3=5.
    pub fn to_byte(self) -> u8 {
        let base = match self.color {
            AllianceColor::Red => 0,
            AllianceColor::Blue => 3,
        };
        base + (self.station - 1)
    }

    /// Decode from a byte. Returns `None` for values >= 6.
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

// ---------------------------------------------------------------------------
// 4. ControlFlags
// ---------------------------------------------------------------------------

/// Flags sent from the Driver Station to the robot in each control packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlFlags {
    /// Emergency stop — bit 7.
    pub estop: bool,
    /// FMS is connected — bit 3.
    pub fms_connected: bool,
    /// Robot is enabled — bit 2.
    pub enabled: bool,
    /// Current operating mode — bits 0-1.
    pub mode: Mode,
}

impl Default for ControlFlags {
    fn default() -> Self {
        ControlFlags {
            estop: false,
            fms_connected: false,
            enabled: false,
            mode: Mode::default(),
        }
    }
}

impl ControlFlags {
    /// Encode to a single byte.
    ///
    /// Layout: `[estop:1][0:3][fms:1][enabled:1][mode:2]`
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

    /// Decode from a single byte.
    pub fn from_byte(byte: u8) -> ControlFlags {
        ControlFlags {
            estop: (byte >> 7) & 1 != 0,
            fms_connected: (byte >> 3) & 1 != 0,
            enabled: (byte >> 2) & 1 != 0,
            mode: Mode::from_bits(byte & 0b11).unwrap_or(Mode::Teleop),
        }
    }
}

// ---------------------------------------------------------------------------
// 5. RequestFlags
// ---------------------------------------------------------------------------

/// Request flags sent from the Driver Station to the robot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestFlags {
    /// Request a RoboRIO reboot — bit 3.
    pub reboot_roborio: bool,
    /// Request a robot code restart — bit 2.
    pub restart_code: bool,
}

impl Default for RequestFlags {
    fn default() -> Self {
        RequestFlags {
            reboot_roborio: false,
            restart_code: false,
        }
    }
}

impl RequestFlags {
    /// Encode to a single byte.
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

// ---------------------------------------------------------------------------
// 6. StatusFlags
// ---------------------------------------------------------------------------

/// Status flags received from the robot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusFlags {
    /// Emergency stop active — bit 7.
    pub estop: bool,
    /// Robot code is still initializing — bit 4.
    pub code_initializing: bool,
    /// Brownout detected — bit 3.
    pub brownout: bool,
    /// Robot is enabled — bit 2.
    pub enabled: bool,
    /// Current operating mode — bits 0-1.
    pub mode: Mode,
}

impl StatusFlags {
    /// Decode from a single byte.
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

// ---------------------------------------------------------------------------
// 7. BatteryVoltage
// ---------------------------------------------------------------------------

/// Robot battery voltage represented as a high byte (integer volts) and a low
/// byte (fractional volts as value/256).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryVoltage {
    pub volts: f32,
}

impl BatteryVoltage {
    /// Decode from the two-byte wire format.
    pub fn from_bytes(high: u8, low: u8) -> BatteryVoltage {
        BatteryVoltage {
            volts: high as f32 + low as f32 / 256.0,
        }
    }

    /// Encode to the two-byte wire format.
    pub fn to_bytes(self) -> (u8, u8) {
        let high = self.volts.floor() as u8;
        let frac = (self.volts - high as f32) * 256.0;
        (high, frac.round() as u8)
    }
}

// ---------------------------------------------------------------------------
// 8. JoystickData
// ---------------------------------------------------------------------------

/// Joystick input state for a single controller.
#[derive(Debug, Clone, Default)]
pub struct JoystickData {
    /// Axis values (–128..127).
    pub axes: Vec<i8>,
    /// Button pressed states.
    pub buttons: Vec<bool>,
    /// POV hat switch values (–1 when centered).
    pub povs: Vec<i16>,
}

// ---------------------------------------------------------------------------
// 9. RumbleOutput
// ---------------------------------------------------------------------------

/// Haptic rumble output for a controller (values clamped 0.0-1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RumbleOutput {
    pub left: f32,
    pub right: f32,
}

// ---------------------------------------------------------------------------
// 10. CanMetrics
// ---------------------------------------------------------------------------

/// CAN bus health metrics reported by the robot.
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

// ---------------------------------------------------------------------------
// 11. TelemetryData
// ---------------------------------------------------------------------------

/// Aggregate telemetry payload received from the robot.
#[derive(Debug, Clone, Default)]
pub struct TelemetryData {
    pub can: CanMetrics,
    pub pdp_currents: Vec<f32>,
    pub cpu_usage: Vec<f32>,
    pub ram_usage: u32,
    pub disk_free: u32,
}

// ---------------------------------------------------------------------------
// 12. RobotState
// ---------------------------------------------------------------------------

/// Complete snapshot of the robot's state as seen by the Driver Station.
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

// ---------------------------------------------------------------------------
// 13. TcpMessage
// ---------------------------------------------------------------------------

/// Messages received from the robot over the TCP channel.
#[derive(Debug, Clone)]
pub enum TcpMessage {
    /// Standard output text from robot code.
    Stdout(String),
    /// An error or warning report.
    ErrorReport {
        timestamp: f64,
        sequence: u16,
        error_code: i32,
        is_error: bool,
        details: String,
        location: String,
        call_stack: String,
    },
    /// Device version information.
    VersionInfo {
        device_type: u8,
        device_id: u8,
        name: String,
        version: String,
    },
    /// Generic message.
    Message(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

        // bit 7 = estop (1), bit 3 = fms (0), bit 2 = enabled (1), bits 0-1 = autonomous (10)
        // 1000_0110 = 0x86 = 134
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
        // 0b0000_1000 => brownout=true, everything else false/Teleop
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
        // bit 3 = reboot (1), bit 2 = restart (1) => 0b0000_1100 = 12
        assert_eq!(flags.to_byte(), 0b0000_1100);
    }

    #[test]
    fn test_alliance_invalid_byte() {
        assert_eq!(Alliance::from_byte(6), None);
        assert_eq!(Alliance::from_byte(255), None);
    }
}
