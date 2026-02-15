# Open Station Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a fully cross-platform FRC Driver Station with Tauri v2 (Rust + React/TypeScript) that replicates all NI Driver Station features except FMS.

**Architecture:** Three-layer Cargo workspace: `open-station-protocol` (FRC protocol), `open-station-core` (app logic, input, hotkeys), `open-station` (Tauri app with React frontend). Protocol is from scratch — no `ds` crate.

**Tech Stack:** Rust, TypeScript, Tauri v2, React, tokio, gilrs, Tailwind CSS

---

## Phase 1: Project Scaffolding

### Task 1: Initialize Cargo workspace and Tauri app

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/open-station-protocol/Cargo.toml`
- Create: `crates/open-station-protocol/src/lib.rs`
- Create: `crates/open-station-core/Cargo.toml`
- Create: `crates/open-station-core/src/lib.rs`

**Step 1: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/open-station-protocol",
    "crates/open-station-core",
    "src-tauri",
]
```

**Step 2: Create protocol crate**

```bash
mkdir -p crates/open-station-protocol/src
```

`crates/open-station-protocol/Cargo.toml`:
```toml
[package]
name = "open-station-protocol"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["net", "rt-multi-thread", "sync", "time", "macros"] }
mdns-sd = "0.11"
log = "0.4"
thiserror = "2"
chrono = "0.4"

[dev-dependencies]
tokio = { version = "1", features = ["test-util"] }
```

`crates/open-station-protocol/src/lib.rs`:
```rust
pub mod types;
pub mod packet;
pub mod connection;
pub mod driver_station;
```

**Step 3: Create core crate**

```bash
mkdir -p crates/open-station-core/src
```

`crates/open-station-core/Cargo.toml`:
```toml
[package]
name = "open-station-core"
version = "0.1.0"
edition = "2021"

[dependencies]
open-station-protocol = { path = "../open-station-protocol" }
gilrs = "0.11"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "6"
log = "0.4"
tokio = { version = "1", features = ["sync", "time"] }
```

**Step 4: Initialize Tauri v2 app**

```bash
cargo install create-tauri-app
# Use npm, React, TypeScript template
npx create-tauri-app@latest . --template react-ts --manager npm --force
```

Then update `src-tauri/Cargo.toml` to add workspace dependencies:
```toml
[dependencies]
open-station-core = { path = "../crates/open-station-core" }
open-station-protocol = { path = "../crates/open-station-protocol" }
```

And add `src-tauri` to the workspace members.

**Step 5: Install frontend dependencies**

```bash
npm install
npm install -D tailwindcss @tailwindcss/vite
```

**Step 6: Verify everything compiles**

```bash
cargo build
npm run tauri dev
```

Expected: Tauri window opens with default React template.

**Step 7: Commit**

```
feat: initialize project scaffolding with Cargo workspace and Tauri v2
```

---

## Phase 2: Protocol Types

### Task 2: Define core protocol types

**Files:**
- Create: `crates/open-station-protocol/src/types.rs`
- Test: `crates/open-station-protocol/src/types.rs` (inline tests)

**Step 1: Write types with inline tests**

```rust
use std::fmt;

/// Robot operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Teleop,
    Test,
    Autonomous,
}

impl Mode {
    /// Encode mode as 2-bit value for control byte
    pub fn to_bits(self) -> u8 {
        match self {
            Mode::Teleop => 0b00,
            Mode::Test => 0b01,
            Mode::Autonomous => 0b10,
        }
    }

    /// Decode mode from 2-bit value
    pub fn from_bits(bits: u8) -> Option<Self> {
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

/// Alliance color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllianceColor {
    Red,
    Blue,
}

/// Alliance station (color + position 1-3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alliance {
    pub color: AllianceColor,
    pub station: u8, // 1, 2, or 3
}

impl Alliance {
    pub fn new(color: AllianceColor, station: u8) -> Self {
        assert!(station >= 1 && station <= 3, "Station must be 1-3");
        Self { color, station }
    }

    /// Encode as single byte for UDP packet
    pub fn to_byte(self) -> u8 {
        let base = match self.color {
            AllianceColor::Red => 0,
            AllianceColor::Blue => 3,
        };
        base + (self.station - 1)
    }

    /// Decode from single byte
    pub fn from_byte(b: u8) -> Option<Self> {
        let color = if b < 3 {
            AllianceColor::Red
        } else if b < 6 {
            AllianceColor::Blue
        } else {
            return None;
        };
        let station = (b % 3) + 1;
        Some(Self { color, station })
    }
}

/// Control byte flags (DS → roboRIO)
#[derive(Debug, Clone, Copy, Default)]
pub struct ControlFlags {
    pub estop: bool,
    pub fms_connected: bool,
    pub enabled: bool,
    pub mode: Mode,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Teleop
    }
}

impl ControlFlags {
    pub fn to_byte(self) -> u8 {
        let mut b: u8 = 0;
        if self.estop { b |= 1 << 7; }
        if self.fms_connected { b |= 1 << 3; }
        if self.enabled { b |= 1 << 2; }
        b |= self.mode.to_bits();
        b
    }

    pub fn from_byte(b: u8) -> Self {
        Self {
            estop: b & (1 << 7) != 0,
            fms_connected: b & (1 << 3) != 0,
            enabled: b & (1 << 2) != 0,
            mode: Mode::from_bits(b).unwrap_or_default(),
        }
    }
}

/// Request byte flags (DS → roboRIO)
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestFlags {
    pub reboot_roborio: bool,
    pub restart_code: bool,
}

impl RequestFlags {
    pub fn to_byte(self) -> u8 {
        let mut b: u8 = 0;
        if self.reboot_roborio { b |= 1 << 3; }
        if self.restart_code { b |= 1 << 2; }
        b
    }
}

/// Status byte flags (roboRIO → DS)
#[derive(Debug, Clone, Copy, Default)]
pub struct StatusFlags {
    pub estop: bool,
    pub code_initializing: bool,
    pub brownout: bool,
    pub enabled: bool,
    pub mode: Mode,
}

impl StatusFlags {
    pub fn from_byte(b: u8) -> Self {
        Self {
            estop: b & (1 << 7) != 0,
            code_initializing: b & (1 << 4) != 0,
            brownout: b & (1 << 3) != 0,
            enabled: b & (1 << 2) != 0,
            mode: Mode::from_bits(b).unwrap_or_default(),
        }
    }
}

/// Decoded battery voltage
#[derive(Debug, Clone, Copy, Default)]
pub struct BatteryVoltage {
    pub volts: f32,
}

impl BatteryVoltage {
    pub fn from_bytes(high: u8, low: u8) -> Self {
        Self {
            volts: high as f32 + low as f32 / 256.0,
        }
    }

    pub fn to_bytes(self) -> (u8, u8) {
        let high = self.volts.floor() as u8;
        let low = ((self.volts - high as f32) * 256.0) as u8;
        (high, low)
    }
}

/// Joystick data for one controller
#[derive(Debug, Clone, Default)]
pub struct JoystickData {
    pub axes: Vec<i8>,
    pub buttons: Vec<bool>,
    pub povs: Vec<i16>,
}

/// Rumble output for XInput devices
#[derive(Debug, Clone, Copy, Default)]
pub struct RumbleOutput {
    pub left: f32,  // 0.0 to 1.0
    pub right: f32, // 0.0 to 1.0
}

/// CAN bus metrics
#[derive(Debug, Clone, Copy, Default)]
pub struct CanMetrics {
    pub utilization: f32,
    pub bus_off_count: u32,
    pub tx_full_count: u32,
    pub rx_error_count: u8,
    pub tx_error_count: u8,
}

/// Robot telemetry data
#[derive(Debug, Clone, Default)]
pub struct TelemetryData {
    pub can: CanMetrics,
    pub pdp_currents: Vec<f32>, // up to 16 channels
    pub cpu_usage: Vec<f32>,    // per-core percentages
    pub ram_usage: u32,         // bytes
    pub disk_free: u32,         // bytes
}

/// Aggregated robot state
#[derive(Debug, Clone, Default)]
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

/// TCP message types received from roboRIO
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
    fn mode_round_trip() {
        for mode in [Mode::Teleop, Mode::Test, Mode::Autonomous] {
            assert_eq!(Mode::from_bits(mode.to_bits()), Some(mode));
        }
    }

    #[test]
    fn alliance_round_trip() {
        for color in [AllianceColor::Red, AllianceColor::Blue] {
            for station in 1..=3 {
                let a = Alliance::new(color, station);
                let decoded = Alliance::from_byte(a.to_byte()).unwrap();
                assert_eq!(decoded.color, color);
                assert_eq!(decoded.station, station);
            }
        }
    }

    #[test]
    fn control_flags_encoding() {
        let flags = ControlFlags {
            estop: true,
            fms_connected: false,
            enabled: true,
            mode: Mode::Autonomous,
        };
        let byte = flags.to_byte();
        assert_eq!(byte & (1 << 7), 1 << 7); // estop set
        assert_eq!(byte & (1 << 2), 1 << 2); // enabled set
        assert_eq!(byte & 0b11, 0b10);        // autonomous

        let decoded = ControlFlags::from_byte(byte);
        assert!(decoded.estop);
        assert!(decoded.enabled);
        assert_eq!(decoded.mode, Mode::Autonomous);
    }

    #[test]
    fn battery_voltage_encoding() {
        let v = BatteryVoltage { volts: 12.5 };
        let (hi, lo) = v.to_bytes();
        let decoded = BatteryVoltage::from_bytes(hi, lo);
        assert!((decoded.volts - 12.5).abs() < 0.01);
    }

    #[test]
    fn status_flags_brownout() {
        let flags = StatusFlags::from_byte(0b0000_1000);
        assert!(flags.brownout);
        assert!(!flags.enabled);
        assert!(!flags.estop);
    }
}
```

**Step 2: Run tests**

```bash
cargo test -p open-station-protocol
```

Expected: All tests pass.

**Step 3: Commit**

```
feat(protocol): add core FRC protocol types with encoding/decoding
```

---

## Phase 3: Packet Encoding/Decoding

### Task 3: Implement outgoing UDP packet builder

**Files:**
- Create: `crates/open-station-protocol/src/packet/mod.rs`
- Create: `crates/open-station-protocol/src/packet/outgoing.rs`
- Update: `crates/open-station-protocol/src/lib.rs`

**Step 1: Write outgoing packet builder with tests**

`crates/open-station-protocol/src/packet/mod.rs`:
```rust
pub mod outgoing;
pub mod incoming;
pub mod tcp;
```

`crates/open-station-protocol/src/packet/outgoing.rs`:

Build the DS → roboRIO UDP packet. Core structure:
```
[seq_hi][seq_lo][0x01][control][request][alliance][...tags]
```

Implement:
- `DsPacketBuilder::new(seq: u16, control: ControlFlags, request: RequestFlags, alliance: Alliance) -> Vec<u8>` — builds the 6-byte header
- `append_joystick_tag(buf: &mut Vec<u8>, joysticks: &[JoystickData])` — appends tag 0x0c per joystick. Format: `[0x0c][axis_count][axes...][button_count][button_bytes...][pov_count][povs...]`
- `append_datetime_tag(buf: &mut Vec<u8>)` — appends tag 0x0f with current UTC datetime (10 bytes: microseconds u32, seconds u8, minutes u8, hours u8, day u8, month u8, year u8)
- `append_timezone_tag(buf: &mut Vec<u8>, tz: &str)` — appends tag 0x10 with timezone string

Tests:
- `test_header_encoding` — verify 6-byte header with known values
- `test_joystick_tag_empty` — joystick with 0 axes/buttons/povs
- `test_joystick_tag_full` — joystick with 6 axes, 12 buttons, 1 POV
- `test_button_packing` — verify LSB-first bit packing (buttons 1,3 set = 0b00000101)

**Step 2: Run tests**

```bash
cargo test -p open-station-protocol packet::outgoing
```

**Step 3: Commit**

```
feat(protocol): implement outgoing UDP packet builder
```

### Task 4: Implement incoming UDP packet parser

**Files:**
- Create: `crates/open-station-protocol/src/packet/incoming.rs`

**Step 1: Write incoming packet parser with tests**

Parse roboRIO → DS UDP packets:
```
[seq_hi][seq_lo][0x01][status][trace][voltage_hi][voltage_lo][request_date][...tags]
```

Implement:
- `parse_rio_packet(data: &[u8]) -> Result<RioPacket, PacketError>` — parses the 8-byte header
- `parse_tags(data: &[u8]) -> Vec<RioTag>` — parses tagged telemetry data after header

Tag parsers:
- Tag 0x01: Joystick outputs (rumble values)
- Tag 0x04: Disk usage (u32 free bytes)
- Tag 0x05: CPU usage (count + per-core f32)
- Tag 0x06: RAM usage (u32 bytes)
- Tag 0x08: PDP data (21 bytes → 16 channels of 10-bit current values)
- Tag 0x0e: CAN metrics (utilization %, bus-off, TX full, RX/TX errors)

`RioPacket` struct:
```rust
pub struct RioPacket {
    pub sequence: u16,
    pub status: StatusFlags,
    pub trace: u8,
    pub voltage: BatteryVoltage,
    pub request_date: bool,
    pub tags: Vec<RioTag>,
}
```

Tests:
- `test_parse_minimal_packet` — 8 bytes, no tags
- `test_parse_voltage` — verify 12.5V decodes correctly
- `test_parse_can_tag` — known CAN metrics bytes
- `test_parse_pdp_tag` — known PDP bytes → 16 current values
- `test_parse_packet_too_short` — returns error for <8 bytes

**Step 2: Run tests**

```bash
cargo test -p open-station-protocol packet::incoming
```

**Step 3: Commit**

```
feat(protocol): implement incoming UDP packet parser with telemetry tags
```

### Task 5: Implement TCP frame protocol

**Files:**
- Create: `crates/open-station-protocol/src/packet/tcp.rs`

**Step 1: Write TCP frame encoder/decoder with tests**

Frame format: `[size_hi][size_lo][tag][payload...]`

Implement:
- `TcpFrameReader` — accumulates bytes from stream, yields complete frames. Handles partial reads.
- `encode_tcp_frame(tag: u8, payload: &[u8]) -> Vec<u8>` — builds a frame
- `parse_tcp_message(tag: u8, payload: &[u8]) -> Result<TcpMessage, PacketError>` — decodes payload by tag

Tag decoders:
- 0x00: Message string (UTF-8)
- 0x0a: Version info (device_type u8, device_id u8, name string, version string)
- 0x0b: Error report (timestamp f64, sequence u16, error_code i32, flags u16, details string, location string, call_stack string)
- 0x0c: Stdout string (UTF-8)

Outbound frame builders:
- `build_game_data_frame(data: &str) -> Vec<u8>`
- `build_match_info_frame(match_number: u16, replay: u8) -> Vec<u8>`
- `build_joystick_descriptor_frame(slot: u8, name: &str, axis_count: u8, button_count: u8, pov_count: u8) -> Vec<u8>`

Tests:
- `test_frame_round_trip` — encode then decode
- `test_partial_read` — feed bytes one at a time to TcpFrameReader
- `test_parse_stdout` — tag 0x0c with UTF-8 string
- `test_parse_error_report` — tag 0x0b with known bytes

**Step 2: Run tests**

```bash
cargo test -p open-station-protocol packet::tcp
```

**Step 3: Commit**

```
feat(protocol): implement TCP frame protocol for messages and stdout
```

---

## Phase 4: Connection and Driver Station

### Task 6: Implement connection state machine

**Files:**
- Create: `crates/open-station-protocol/src/connection.rs`

**Step 1: Write connection state machine**

States: `Disconnected → Resolving → Connected → CodeRunning`

Implement:
- `ConnectionManager` — async struct managing UDP + TCP connections
- mDNS resolution: query `roboRIO-{team}-FRC.local`, fallback to `10.TE.AM.2`
- USB mode: connect to `172.22.11.2` directly
- UDP send loop at 50Hz (20ms tokio::interval)
- UDP receive loop (non-blocking, timeout-based disconnect detection)
- TCP connection with auto-reconnect
- Exponential backoff on connection failure (100ms → 200ms → 400ms → ... → 2s cap)
- Trip time calculation (sequence number round-trip tracking)
- Lost packet counting

Key methods:
```rust
impl ConnectionManager {
    pub async fn new(team: u32, alliance: Alliance) -> Self;
    pub async fn start(&mut self);
    pub fn set_team(&mut self, team: u32);
    pub fn set_usb_mode(&mut self, usb: bool);
    pub fn send_control(&self, flags: ControlFlags, request: RequestFlags, joysticks: &[JoystickData]);
    pub fn is_connected(&self) -> bool;
    pub fn trip_time_ms(&self) -> f64;
    pub fn lost_packets(&self) -> u32;
}
```

Tests (unit tests with mock sockets where possible):
- `test_state_transitions` — verify state machine flow
- `test_team_to_ip` — team 1234 → 10.12.34.2
- `test_usb_mode_ip` — USB mode → 172.22.11.2
- `test_backoff_capping` — verify backoff caps at 2s

**Step 2: Run tests**

```bash
cargo test -p open-station-protocol connection
```

**Step 3: Commit**

```
feat(protocol): implement connection state machine with mDNS and fallback
```

### Task 7: Implement DriverStation protocol driver

**Files:**
- Create: `crates/open-station-protocol/src/driver_station.rs`

**Step 1: Write the main DriverStation struct**

This is the public API of the protocol crate. It owns a `ConnectionManager` and exposes a clean interface.

```rust
use tokio::sync::{mpsc, watch};

pub struct DriverStation {
    // internal state
    control: ControlFlags,
    request: RequestFlags,
    alliance: Alliance,
    team: u32,
    joysticks: Vec<JoystickData>,
    // channels
    state_tx: watch::Sender<RobotState>,
    stdout_tx: mpsc::UnboundedSender<String>,
    tcp_message_tx: mpsc::UnboundedSender<TcpMessage>,
}

impl DriverStation {
    pub fn new(team: u32, alliance: Alliance) -> (Self, DsReceiver);
    pub async fn run(&mut self); // main loop — call once, runs forever
    pub fn enable(&mut self);
    pub fn disable(&mut self);
    pub fn estop(&mut self);
    pub fn set_mode(&mut self, mode: Mode);
    pub fn set_team(&mut self, team: u32);
    pub fn set_alliance(&mut self, alliance: Alliance);
    pub fn set_joysticks(&mut self, joysticks: Vec<JoystickData>);
    pub fn set_game_data(&mut self, data: String);
    pub fn set_usb_mode(&mut self, usb: bool);
    pub fn reboot_roborio(&mut self);
    pub fn restart_code(&mut self);
    pub fn is_estopped(&self) -> bool;
}

/// Receiver handle for consuming DS events
pub struct DsReceiver {
    pub state: watch::Receiver<RobotState>,
    pub stdout: mpsc::UnboundedReceiver<String>,
    pub messages: mpsc::UnboundedReceiver<TcpMessage>,
}
```

The `run()` method spawns:
1. UDP send task (50Hz) — builds packet from current control/request/joystick state
2. UDP receive task — parses incoming, updates RobotState, pushes to watch channel
3. TCP connection task — connects, reads frames, dispatches to appropriate mpsc channels
4. Date/time sender — sends datetime tag once on first connection, timezone once

Tests:
- `test_enable_disable_state` — enable sets flag, disable clears it
- `test_estop_persists` — estop stays set even after disable
- `test_mode_switching` — mode changes reflected in control flags
- `test_joystick_data_flow` — set joysticks, verify they'd be in next packet

**Step 2: Run tests**

```bash
cargo test -p open-station-protocol driver_station
```

**Step 3: Commit**

```
feat(protocol): implement DriverStation protocol driver with async API
```

---

## Phase 5: Core Layer

### Task 8: Implement configuration persistence

**Files:**
- Create: `crates/open-station-core/src/config.rs`
- Update: `crates/open-station-core/src/lib.rs`

**Step 1: Write config module with tests**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub team_number: u32,
    pub use_usb: bool,
    pub dashboard_command: Option<String>,
    pub game_data: String,
    pub practice_timing: PracticeTiming,
    pub practice_audio: bool,
    pub joystick_locks: HashMap<String, u8>, // UUID → slot
    pub window: WindowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeTiming {
    pub countdown_secs: u32,
    pub auto_secs: u32,
    pub delay_secs: u32,
    pub teleop_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
}

impl Default for Config { /* sensible defaults: team 0, 15s auto, 135s teleop, etc */ }
impl Default for PracticeTiming { /* 3, 15, 1, 135 */ }
impl Default for WindowConfig { /* 800x300 */ }

impl Config {
    pub fn config_dir() -> PathBuf; // platform-specific
    pub fn config_path() -> PathBuf; // config_dir/config.toml
    pub fn load() -> Self; // load from disk or return default
    pub fn save(&self) -> Result<(), std::io::Error>; // write to disk
}
```

Tests:
- `test_default_config` — verify defaults are sensible
- `test_round_trip` — serialize then deserialize, compare
- `test_missing_file_returns_default` — load from nonexistent path

**Step 2: Run tests**

```bash
cargo test -p open-station-core config
```

**Step 3: Commit**

```
feat(core): implement TOML configuration persistence
```

### Task 9: Implement joystick input management

**Files:**
- Create: `crates/open-station-core/src/input/mod.rs`
- Create: `crates/open-station-core/src/input/mapping.rs`
- Update: `crates/open-station-core/src/lib.rs`

**Step 1: Write joystick input module**

`input/mod.rs` — Manages gilrs instance, polls gamepads, emits events:
```rust
pub struct JoystickManager {
    gilrs: Gilrs,
    slots: [Option<JoystickSlot>; 6],
    locks: HashMap<String, u8>,
    on_disconnect_while_enabled: Option<Box<dyn Fn() + Send>>,
}

pub struct JoystickSlot {
    pub uuid: String,
    pub name: String,
    pub gilrs_id: GamepadId,
    pub locked: bool,
    pub data: JoystickData, // from protocol types
}

pub struct JoystickInfo {
    pub slot: u8,
    pub uuid: String,
    pub name: String,
    pub locked: bool,
    pub connected: bool,
    pub axis_count: u8,
    pub button_count: u8,
    pub pov_count: u8,
}

impl JoystickManager {
    pub fn new(locks: HashMap<String, u8>) -> Self;
    pub fn poll(&mut self); // call every 5ms
    pub fn get_joystick_data(&self) -> Vec<JoystickData>; // for protocol
    pub fn get_joystick_info(&self) -> Vec<JoystickInfo>; // for UI
    pub fn reorder(&mut self, order: Vec<String>); // by UUID
    pub fn lock(&mut self, uuid: &str, slot: u8);
    pub fn unlock(&mut self, uuid: &str);
    pub fn rescan(&mut self);
    pub fn any_connected(&self) -> bool;
}
```

`input/mapping.rs` — Maps gilrs axis/button enums to FRC indices:
```rust
pub fn map_axis(axis: gilrs::Axis) -> Option<usize>;    // returns FRC axis index 0-5
pub fn map_button(button: gilrs::Button) -> Option<usize>; // returns FRC button index 0-9
pub fn map_dpad(gilrs_state: &Gamepad) -> i16;           // returns POV angle or -1
```

Tests:
- `test_axis_mapping` — verify LeftStickX→0, RightStickY→5, etc.
- `test_button_mapping` — verify South→0 (FRC button 1), etc.
- `test_dpad_angles` — verify DPadUp→0, DPadRight→90, etc.

**Step 2: Run tests**

```bash
cargo test -p open-station-core input
```

**Step 3: Commit**

```
feat(core): implement joystick input management with FRC mapping
```

### Task 10: Implement practice mode sequencer

**Files:**
- Create: `crates/open-station-core/src/practice.rs`

**Step 1: Write practice mode state machine with tests**

```rust
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticePhase {
    Idle,
    Countdown,
    Autonomous,
    Delay,
    Teleop,
    Done,
}

pub struct PracticeMode {
    phase: PracticePhase,
    timing: PracticeTiming,
    phase_start: Option<Instant>,
    a_stopped: bool,
}

pub struct PracticeTick {
    pub phase: PracticePhase,
    pub elapsed: Duration,
    pub remaining: Duration,
    pub should_enable: bool,
    pub should_disable: bool,
    pub mode: Mode,         // what mode the robot should be in
}

impl PracticeMode {
    pub fn new(timing: PracticeTiming) -> Self;
    pub fn start(&mut self);
    pub fn stop(&mut self);
    pub fn a_stop(&mut self);        // pause during auto, resume at teleop
    pub fn tick(&mut self) -> PracticeTick; // call every 20ms
    pub fn phase(&self) -> PracticePhase;
    pub fn is_running(&self) -> bool;
}
```

Tests:
- `test_phase_transitions` — verify Idle→Countdown→Auto→Delay→Teleop→Done
- `test_a_stop_during_auto` — a_stop sets should_disable, auto-re-enables at teleop
- `test_stop_resets_to_idle` — calling stop at any point returns to Idle
- `test_timing_accuracy` — with mocked time, verify phase durations

**Step 2: Run tests**

```bash
cargo test -p open-station-core practice
```

**Step 3: Commit**

```
feat(core): implement practice mode sequencer with A-Stop support
```

### Task 11: Implement global keyboard hooks

**Files:**
- Create: `crates/open-station-core/src/hotkeys/mod.rs`
- Create: `crates/open-station-core/src/hotkeys/macos.rs`
- Create: `crates/open-station-core/src/hotkeys/linux.rs`
- Create: `crates/open-station-core/src/hotkeys/windows.rs`

**Step 1: Write hotkey module with platform backends**

```rust
pub enum HotkeyAction {
    EStop,
    Disable,
    Enable,
    AStop,
    RescanJoysticks,
}

pub trait HotkeyBackend: Send {
    fn start(&mut self, tx: mpsc::UnboundedSender<HotkeyAction>);
    fn stop(&mut self);
}

pub struct HotkeyManager {
    backend: Box<dyn HotkeyBackend>,
    rx: mpsc::UnboundedReceiver<HotkeyAction>,
}

impl HotkeyManager {
    pub fn new() -> Self; // selects platform backend
    pub fn start(&mut self);
    pub async fn next_action(&mut self) -> Option<HotkeyAction>;
}
```

Platform backends:
- **macOS** (`macos.rs`): `CGEventTapCreate` with `kCGEventKeyDown` mask. Runs on separate thread via `CFRunLoop`.
- **Linux** (`linux.rs`): Read from `/dev/input/event*` devices using evdev. Polls every 10ms. Falls back to X11 `XGrabKey` if evdev unavailable.
- **Windows** (`windows.rs`): `SetWindowsHookExW` with `WH_KEYBOARD_LL`. Runs message pump on dedicated thread.

Key mappings:
- Space (keycode varies by platform) → EStop
- Enter/Return → Disable
- `[` + `]` + `\` simultaneously → Enable (track key-down state for all three)
- Backspace → AStop
- F1 → RescanJoysticks

Note: E-Stop via Space must work even when the app window is unfocused. This is why we need global hooks.

**Step 2: Verify compilation on current platform**

```bash
cargo build -p open-station-core
```

**Step 3: Commit**

```
feat(core): implement global keyboard hooks for e-stop, enable/disable
```

### Task 12: Implement aggregated application state

**Files:**
- Create: `crates/open-station-core/src/state.rs`
- Update: `crates/open-station-core/src/lib.rs`

**Step 1: Write state aggregator**

This is the main entry point for the core crate. It owns and coordinates:
- `DriverStation` (protocol)
- `JoystickManager` (input)
- `PracticeMode` (practice)
- `HotkeyManager` (hotkeys)
- `Config` (persistence)

```rust
use tokio::sync::watch;

pub struct AppState {
    ds: DriverStation,
    ds_rx: DsReceiver,
    joysticks: JoystickManager,
    practice: PracticeMode,
    hotkeys: HotkeyManager,
    config: Config,
    // outbound channels for Tauri
    ui_state_tx: watch::Sender<UiState>,
}

/// Flattened state for the UI
#[derive(Debug, Clone, Serialize)]
pub struct UiState {
    // Robot
    pub connected: bool,
    pub code_running: bool,
    pub voltage: f32,
    pub brownout: bool,
    pub estopped: bool,
    pub enabled: bool,
    pub mode: String,
    // Telemetry
    pub telemetry: TelemetryData,
    // Joysticks
    pub joysticks: Vec<JoystickInfo>,
    pub any_joystick_connected: bool,
    // Practice
    pub practice_phase: String,
    pub practice_elapsed: f64,
    pub practice_remaining: f64,
    // Connection
    pub trip_time_ms: f64,
    pub lost_packets: u32,
    // Meta
    pub team_number: u32,
    pub alliance: String,
}

impl AppState {
    pub async fn new(config: Config) -> Self;
    pub async fn run(&mut self); // main loop coordinating all subsystems

    // Commands (called from Tauri)
    pub fn enable(&mut self);
    pub fn disable(&mut self);
    pub fn estop(&mut self);
    pub fn set_mode(&mut self, mode: Mode);
    pub fn set_team(&mut self, team: u32);
    pub fn set_alliance(&mut self, alliance: Alliance);
    pub fn set_game_data(&mut self, data: String);
    pub fn set_usb_mode(&mut self, usb: bool);
    pub fn reboot_roborio(&mut self);
    pub fn restart_code(&mut self);
    pub fn start_practice(&mut self);
    pub fn stop_practice(&mut self);
    pub fn reorder_joysticks(&mut self, order: Vec<String>);
    pub fn lock_joystick(&mut self, uuid: String, slot: u8);
    pub fn unlock_joystick(&mut self, uuid: String);
    pub fn rescan_joysticks(&mut self);
    pub fn launch_dashboard(&self);
    pub fn save_config(&self);

    // State access
    pub fn subscribe_state(&self) -> watch::Receiver<UiState>;
}
```

The `run()` loop:
1. Spawn DS protocol (ds.run())
2. Every 5ms: poll joysticks, update DS with joystick data
3. Every 20ms: tick practice mode, apply enable/disable/mode changes
4. Listen for hotkey actions, dispatch accordingly
5. Every 20ms: build UiState from all sources, send to watch channel
6. Forward stdout/messages from DS receiver

**Step 2: Verify compilation**

```bash
cargo build -p open-station-core
```

**Step 3: Commit**

```
feat(core): implement aggregated application state coordinator
```

---

## Phase 6: Tauri Integration

### Task 13: Wire Tauri commands and events

**Files:**
- Modify: `src-tauri/src/main.rs`
- Create: `src-tauri/src/commands.rs`
- Create: `src-tauri/src/events.rs`

**Step 1: Write Tauri commands**

`src-tauri/src/commands.rs` — Each command acquires a lock on AppState and calls the corresponding method:

```rust
use tauri::State;
use std::sync::Mutex;
use open_station_core::state::AppState;

type AppStateHandle = Mutex<AppState>;

#[tauri::command]
pub fn enable(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().enable();
}

#[tauri::command]
pub fn disable(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().disable();
}

// ... etc for all commands from the design doc
```

**Step 2: Write event emitter**

`src-tauri/src/events.rs` — Background task that reads from the UiState watch channel and emits Tauri events:

```rust
pub fn spawn_event_emitter(app: tauri::AppHandle, mut rx: watch::Receiver<UiState>) {
    tauri::async_runtime::spawn(async move {
        loop {
            if rx.changed().await.is_ok() {
                let state = rx.borrow().clone();
                app.emit("robot-state", &state).ok();
            }
        }
    });
}
```

Also spawn separate emitters for:
- stdout messages → `stdout-message` event
- TCP messages → `error-message` event
- Practice ticks → `practice-tick` event

**Step 3: Wire up main.rs**

```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config = Config::load();
            let app_state = AppState::new(config);
            let rx = app_state.subscribe_state();
            app.manage(Mutex::new(app_state));
            spawn_event_emitter(app.handle().clone(), rx);
            // spawn app_state.run() on async runtime
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::enable,
            commands::disable,
            commands::estop,
            commands::set_mode,
            commands::set_team_number,
            commands::set_alliance,
            // ... all commands
        ])
        .run(tauri::generate_context!())
        .expect("error running Open Station");
}
```

**Step 4: Verify compilation**

```bash
cargo build -p open-station
```

**Step 5: Commit**

```
feat(tauri): wire Tauri commands and event emitters to core state
```

---

## Phase 7: Frontend — Layout and Status Pane

### Task 14: Set up React app structure and dark theme

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css` → delete, use Tailwind
- Create: `src/index.css` (Tailwind base)
- Modify: `vite.config.ts` (add Tailwind plugin)
- Create: `src/hooks/useTauriEvent.ts`
- Create: `src/hooks/useTauriCommand.ts`
- Create: `src/types.ts`

**Step 1: Configure Tailwind**

`src/index.css`:
```css
@import "tailwindcss";
```

`vite.config.ts` — add `@tailwindcss/vite` plugin.

**Step 2: Create TypeScript types matching Rust UiState**

`src/types.ts`:
```typescript
export interface RobotState {
  connected: boolean;
  code_running: boolean;
  voltage: number;
  brownout: boolean;
  estopped: boolean;
  enabled: boolean;
  mode: string;
  telemetry: TelemetryData;
  joysticks: JoystickInfo[];
  any_joystick_connected: boolean;
  practice_phase: string;
  practice_elapsed: number;
  practice_remaining: number;
  trip_time_ms: number;
  lost_packets: number;
  team_number: number;
  alliance: string;
}

export interface TelemetryData {
  can: CanMetrics;
  pdp_currents: number[];
  cpu_usage: number[];
  ram_usage: number;
  disk_free: number;
}

export interface CanMetrics {
  utilization: number;
  bus_off_count: number;
  tx_full_count: number;
  rx_error_count: number;
  tx_error_count: number;
}

export interface JoystickInfo {
  slot: number;
  uuid: string;
  name: string;
  locked: boolean;
  connected: boolean;
  axis_count: number;
  button_count: number;
  pov_count: number;
}
```

**Step 3: Create Tauri hooks**

`src/hooks/useTauriEvent.ts`:
```typescript
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

export function useTauriEvent<T>(event: string, initial: T): T {
  const [value, setValue] = useState<T>(initial);
  useEffect(() => {
    const unlisten = listen<T>(event, (e) => setValue(e.payload));
    return () => { unlisten.then((f) => f()); };
  }, [event]);
  return value;
}
```

`src/hooks/useTauriCommand.ts`:
```typescript
import { invoke } from "@tauri-apps/api/core";

export function useTauriCommand() {
  return {
    enable: () => invoke("enable"),
    disable: () => invoke("disable"),
    estop: () => invoke("estop"),
    setMode: (mode: string) => invoke("set_mode", { mode }),
    setTeamNumber: (team: number) => invoke("set_team_number", { team }),
    setAlliance: (color: string, station: number) => invoke("set_alliance", { color, station }),
    setGameData: (data: string) => invoke("set_game_data", { data }),
    setUsbConnection: (enabled: boolean) => invoke("set_usb_connection", { enabled }),
    rebootRoborio: () => invoke("reboot_roborio"),
    restartRobotCode: () => invoke("restart_robot_code"),
    startPracticeMode: () => invoke("start_practice_mode"),
    stopPracticeMode: () => invoke("stop_practice_mode"),
    setPracticeTiming: (auto: number, delay: number, teleop: number) =>
      invoke("set_practice_timing", { auto: auto, delay, teleop }),
    reorderJoysticks: (order: string[]) => invoke("reorder_joysticks", { order }),
    lockJoystick: (uuid: string, slot: number) => invoke("lock_joystick", { uuid, slot }),
    unlockJoystick: (uuid: string) => invoke("unlock_joystick", { uuid }),
    rescanJoysticks: () => invoke("rescan_joysticks"),
    launchDashboard: () => invoke("launch_dashboard"),
  };
}
```

**Step 4: Create App layout shell**

`src/App.tsx`:
```tsx
import { useState } from "react";
import { useTauriEvent } from "./hooks/useTauriEvent";
import { RobotState } from "./types";
import StatusPane from "./components/StatusPane";
// tab imports...

const INITIAL_STATE: RobotState = { /* all defaults */ };

type LeftTab = "operation" | "diagnostics" | "setup" | "usb" | "canpower";
type RightTab = "messages" | "charts" | "both";

export default function App() {
  const state = useTauriEvent<RobotState>("robot-state", INITIAL_STATE);
  const [leftTab, setLeftTab] = useState<LeftTab>("operation");
  const [rightTab, setRightTab] = useState<RightTab>("messages");

  return (
    <div className="h-screen flex flex-col bg-[#1e1e1e] text-white text-sm select-none">
      <div className="flex flex-1 overflow-hidden">
        {/* Left panel: tab icons + content */}
        <div className="flex">
          <TabBar tabs={LEFT_TABS} active={leftTab} onSelect={setLeftTab} />
          <div className="w-64 border-r border-gray-700 overflow-y-auto">
            {/* render active left tab */}
          </div>
        </div>

        {/* Center: always-visible status pane */}
        <div className="flex-1 flex items-center justify-center">
          <StatusPane state={state} />
        </div>

        {/* Right panel: tab icons + content */}
        <div className="flex">
          <div className="w-80 border-l border-gray-700 overflow-y-auto">
            {/* render active right tab */}
          </div>
          <TabBar tabs={RIGHT_TABS} active={rightTab} onSelect={setRightTab} />
        </div>
      </div>
    </div>
  );
}
```

**Step 5: Commit**

```
feat(frontend): set up React app structure with Tailwind dark theme
```

### Task 15: Build the center Status Pane

**Files:**
- Create: `src/components/StatusPane.tsx`

**Step 1: Implement StatusPane component**

Displays (always visible):
- Team number
- Battery voltage with color coding (>=12.5V green, 8.5-11.5V yellow, <8.5V red), background red on brownout
- Three status circles: Communications (green/red), Robot Code (green/red), Joysticks (green/red)
- Status string derived from state ("No Robot Communication", "Teleoperated Enabled", "Emergency Stopped", "Voltage Brownout", etc.)

```tsx
function getStatusString(state: RobotState): string {
  if (!state.connected) return "No Robot Communication";
  if (state.estopped) return "Emergency Stopped";
  if (state.brownout) return "Voltage Brownout";
  if (!state.code_running) return "No Robot Code";
  if (state.enabled) return `${state.mode} Enabled`;
  return `${state.mode} Disabled`;
}

function voltageColor(v: number, brownout: boolean): string {
  if (brownout) return "bg-red-600";
  if (v >= 12.5) return "text-green-400";
  if (v >= 8.5) return "text-yellow-400";
  return "text-red-400";
}
```

**Step 2: Verify it renders**

```bash
npm run tauri dev
```

Expected: Status pane visible in center with placeholder data.

**Step 3: Commit**

```
feat(frontend): implement center status pane with voltage and indicators
```

---

## Phase 8: Frontend — Left Tabs

### Task 16: Build Operation tab

**Files:**
- Create: `src/components/tabs/OperationTab.tsx`

Content:
- Radio button group: Teleoperated / Autonomous / Test / Practice
- Large Enable button (green) and Disable button (red)
- Elapsed time (counts up while enabled)
- Alliance station dropdown (Red/Blue 1/2/3)
- PC battery % and CPU % (use Tauri's `os` plugin or a simple interval)

### Task 17: Build Diagnostics tab

**Files:**
- Create: `src/components/tabs/DiagnosticsTab.tsx`

Content:
- DS version string (hardcoded, e.g., "Open Station v0.1.0")
- roboRIO image version (from TCP version tags)
- WPILib version (from TCP version tags)
- Memory stats section (RAM used, disk free from telemetry)
- Connection indicator lights: Enet Link, Robot Radio, Robot, FMS
- Network indicator lights: Ethernet IP, WiFi, USB, Firewall
- "Reboot roboRIO" button with confirmation modal
- "Restart Robot Code" button

### Task 18: Build Setup tab

**Files:**
- Create: `src/components/tabs/SetupTab.tsx`

Content:
- Team number input (max 4 digits, calls setTeamNumber on blur/enter)
- Dashboard type selector (dropdown: None, Custom Command) + command text input
- Game Data input (max 3 characters)
- Practice Mode timing inputs: countdown, auto, delay, teleop (in seconds)
- Audio control checkbox
- "Connect via USB" checkbox

### Task 19: Build USB Devices tab

**Files:**
- Create: `src/components/tabs/USBDevicesTab.tsx`

Content:
- Drag-and-drop sortable list (6 slots). Use a lightweight drag library or HTML5 drag-and-drop.
- Each entry shows: slot number, device name, lock icon (double-click to toggle)
- Disconnected locked devices show grayed out with underline
- Rescan button (also triggered by F1)
- Selected device detail panel: axis value bars (-128 to 127 mapped to visual bar), button indicators (lit when pressed), POV compass (shows angle)
- Rumble test: two horizontal sliders (left/right rumble, 0.0 to 1.0)

### Task 20: Build CAN/Power tab

**Files:**
- Create: `src/components/tabs/CANPowerTab.tsx`

Content:
- Fault counters: Comms Faults, 12V Faults, 6V/5V/3.3V Faults (from telemetry, count up from 0 since connection)
- CAN Bus Utilization bar (0-100%)
- CAN fault counters: Bus Off, TX Full, RX Errors, TX Errors
- Tab icon in sidebar turns red when any fault > 0

**Step for each tab (16-20): Implement, verify renders with `npm run tauri dev`, commit**

Commits:
```
feat(frontend): implement Operation tab with mode selection and enable/disable
feat(frontend): implement Diagnostics tab with connection indicators
feat(frontend): implement Setup tab with team number and practice timing
feat(frontend): implement USB Devices tab with drag-and-drop joystick management
feat(frontend): implement CAN/Power tab with fault counters
```

---

## Phase 9: Frontend — Right Tabs

### Task 21: Build Messages tab

**Files:**
- Create: `src/components/tabs/MessagesTab.tsx`

Content:
- Scrollable log div with auto-scroll (locks to bottom unless user scrolls up)
- Listen to `stdout-message` and `error-message` Tauri events
- Each line: timestamp + message, color-coded (white=info, yellow=warning, red=error)
- Gear menu (dropdown): Clear messages, Open log file location
- Max 1000 lines in memory (ring buffer), oldest dropped

### Task 22: Build Charts tab

**Files:**
- Create: `src/components/tabs/ChartsTab.tsx`

Content:
- Two stacked canvas charts (use HTML5 Canvas directly — no heavy charting library)
- **Top chart**: Trip time in ms (green line, right Y axis) + lost packets/sec (orange line, left Y axis)
- **Bottom chart**: Battery voltage (yellow, left Y axis) + roboRIO CPU % (red, right Y axis) + mode indicator bands at bottom
- Time scale buttons: 5s, 30s, 1m, 5m
- Data stored in rolling buffer matching time scale
- Listen to `chart-data` Tauri event (emitted at 1Hz with latest data point)

### Task 23: Build Both tab

**Files:**
- Create: `src/components/tabs/BothTab.tsx`

Content:
- Simple flex layout: Messages on left, Charts on right, both at 50% width
- Reuse MessagesTab and ChartsTab components

Commits:
```
feat(frontend): implement Messages tab with auto-scrolling log
feat(frontend): implement Charts tab with trip time and voltage graphs
feat(frontend): implement Both tab combining Messages and Charts
```

---

## Phase 10: Integration and Polish

### Task 24: End-to-end integration testing

**Step 1:** Run the full app and verify:
- Tauri window opens with dark theme
- All tabs render and switch correctly
- Status pane shows "No Robot Communication" by default
- Team number input saves to config
- Joystick detection works (plug in a controller)
- Enable/disable buttons invoke Tauri commands

**Step 2:** Test with a real roboRIO (if available):
- Set team number, verify connection
- Verify battery voltage displays
- Verify enable/disable/e-stop work
- Verify joystick data reaches robot
- Verify stdout messages appear
- Verify practice mode cycles correctly

**Step 3:** Test keyboard shortcuts:
- Space → E-Stop
- Enter → Disable
- `[` + `]` + `\` → Enable

### Task 25: Cross-platform build verification

**Step 1:** Build for current platform:
```bash
npm run tauri build
```

**Step 2:** Verify binary runs standalone (no dev server needed).

**Step 3:** Test on other platforms if available (CI can handle this later).

### Task 26: Final polish

- Add app icon (create a simple Open Station logo)
- Set window title to "Open Station"
- Set minimum window size (800x300)
- Add `--team` CLI argument for setting team number at launch
- Ensure config saves on window close
- Add version number display in Diagnostics tab

Commits:
```
test: end-to-end integration verification
build: configure cross-platform Tauri builds
feat: add app icon, window config, and CLI arguments
```
