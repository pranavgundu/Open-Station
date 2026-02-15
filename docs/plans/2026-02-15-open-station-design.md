# Open Station Design Document

Cross-platform FRC Driver Station built with Tauri v2 (Rust + React/TypeScript).
Targets macOS, Linux, and Windows. All features of the NI Driver Station except FMS integration.

## Decisions

- **UI Framework**: Tauri v2 (Rust backend, React/TypeScript frontend)
- **Protocol**: Custom from-scratch Rust implementation (no `ds` crate)
- **Frontend**: React with TypeScript
- **Design**: Modern dark theme
- **Dashboard**: DS only, launches external dashboards via config
- **Practice Mode**: Included from day one

## Architecture

Three-layer crate structure:

```
open-station (Tauri app)
  └── open-station-core (app logic)
       └── open-station-protocol (FRC protocol)
```

### Layer 1: `open-station-protocol`

Pure Rust library. Zero UI dependencies. Handles all FRC Driver Station protocol communication.

#### Modules

**`packet/outgoing.rs`** — DS to roboRIO UDP (port 1110)
- Sequence number (u16, incrementing)
- Control byte: e-stop (bit 7), enabled (bit 2), mode (bits 0-1: 00=Teleop, 01=Test, 10=Auto)
- Request byte: reboot roboRIO (bit 3), restart code (bit 2)
- Alliance station encoding
- Tagged data: joystick (0x0c), date/time (0x0f), timezone (0x10), countdown (0x07)

**`packet/incoming.rs`** — roboRIO to DS UDP (port 1150)
- Status byte: e-stop, brownout, enabled, mode
- Trace byte: code execution indicators
- Battery voltage: 2-byte encoding (high + low/256)
- Telemetry tags: CAN metrics (0x0e), PDP (0x08), CPU (0x05), RAM (0x06), disk (0x04), joystick outputs/rumble (0x01)

**`packet/tcp.rs`** — Bidirectional TCP (port 1740)
- Frame protocol: size (u16) + tag (u8) + payload
- Inbound: stdout (0x0c), errors (0x0b), version info (0x0a), messages (0x00)
- Outbound: joystick descriptors, match info, game data

**`connection.rs`** — Connection state machine
- States: Disconnected → Resolving → Connected → CodeRunning
- mDNS for `roboRIO-TEAM-FRC.local`, fallback to `10.TE.AM.2`
- USB fallback to `172.22.11.2`
- Auto-reconnection with backoff
- 50Hz UDP send loop (20ms)

**`types.rs`** — Shared types
- `Mode` (Teleop, Autonomous, Test)
- `Alliance` (Red1-3, Blue1-3)
- `RobotState` (voltage, mode, enabled, estopped, brownout, code running, connected)
- `TelemetryData` (CAN, PDP currents, CPU, RAM, disk)
- `JoystickData` (axes: Vec<i8>, buttons: Vec<bool>, povs: Vec<i16>)

**`driver_station.rs`** — Main protocol driver
- Owns UDP + TCP sockets
- Send/receive on dedicated threads
- API: `enable()`, `disable()`, `estop()`, `set_mode()`, `set_team()`, etc.
- Callbacks: `on_state_change`, `on_telemetry`, `on_stdout`, `on_error`

**Dependencies**: `tokio`, `socket2`, `mdns-sd` only.

### Layer 2: `open-station-core`

Application logic. Depends on `open-station-protocol`.

**`input/mod.rs`** — Joystick management via `gilrs`
- Polls every 5ms on dedicated thread
- Connect/disconnect detection
- Auto-disables robot on mapped joystick disconnect
- 6 slot limit (FRC spec)

**`input/mapping.rs`** — FRC joystick mapping
- Standard axis layout: LX=0, LY=1, LT=2, RT=3, RX=4, RY=5
- Standard buttons: A=1, B=2, Y=3, X=4, LB=5, RB=6, Select=7, Start=8, LS=9, RS=10
- D-pad to POV 0 (0/90/180/270 degrees, -1 released)
- Slot locking by device UUID, persisted

**`practice.rs`** — Practice mode sequencer
- States: Idle → Countdown → Autonomous → Delay → Teleop → Done
- Default timing: 3s countdown, 15s auto, 1s delay, 135s teleop
- A-Stop: disable during auto, auto-re-enable at teleop

**`hotkeys.rs`** — Global keyboard hooks
- Space → E-Stop (global, even unfocused)
- Enter → Disable
- `[` + `]` + `\` → Enable
- Backspace → A-Stop (practice mode)
- F1 → Rescan joysticks
- Platform backends: CGEventTap (macOS), evdev (Linux), SetWindowsHookEx (Windows)

**`config.rs`** — Persistent TOML config
- Team number, joystick locks, practice timing, dashboard command, window geometry
- Platform config dirs (~/.config/, ~/Library/Application Support/, %APPDATA%)

**`state.rs`** — Aggregated app state
- Combines protocol + input + practice into unified state
- Emits change events for Tauri layer
- Thread-safe (Arc<Mutex<>>)

### Layer 3: `open-station` (Tauri app)

**Rust side (`src-tauri/`)**

`commands.rs` — Tauri IPC commands:
- `set_team_number`, `enable`, `disable`, `estop`
- `set_mode`, `set_alliance`
- `start_practice_mode`, `stop_practice_mode`, `set_practice_timing`
- `reboot_roborio`, `restart_robot_code`
- `set_game_data`, `set_usb_connection`
- `reorder_joysticks`, `lock_joystick`, `unlock_joystick`, `rescan_joysticks`
- `set_dashboard_command`, `launch_dashboard`
- `get_config`

`events.rs` — Tauri events (Rust → frontend):
- `robot-state` (50Hz): voltage, mode, enabled, estopped, brownout, connected, code running
- `telemetry` (~2Hz): CAN, PDP, CPU, RAM, disk
- `joystick-update` (on change): full joystick list
- `joystick-values` (50Hz): live axis/button/POV
- `stdout-message`, `error-message` (as received)
- `practice-tick`: phase, elapsed, remaining
- `chart-data` (1Hz): trip time, lost packets, voltage, CPU history

**React frontend (`src/`)**

Layout: left sidebar tabs + right sidebar tabs + persistent center status pane.

Center Status Pane (always visible):
- Team number, battery voltage (numeric + mini chart, red on brownout)
- Three indicators: Communications (split TCP/UDP), Robot Code, Joysticks
- Status string

Left Tabs:
1. **Operation** — Mode selector, enable/disable buttons, elapsed time, PC battery/CPU, alliance selector
2. **Diagnostics** — DS/roboRIO/WPILib versions, memory stats, connection indicators (Enet/Radio/Robot/FMS), network indicators, reboot/restart buttons
3. **Setup** — Team number, dashboard config, game data, practice timing, audio toggle, USB checkbox
4. **USB Devices** — Drag-and-drop joystick list (6 slots), lock/unlock, live axis/button/POV display, rumble test
5. **CAN/Power** — Comms/voltage/rail fault counters, CAN utilization bar, CAN fault counters

Right Tabs:
6. **Messages** — Scrollable log (stdout, DS messages, errors), color-coded severity, clear/console/log viewer
7. **Charts** — Trip time + lost packets chart, voltage + CPU + mode chart, time scale controls
8. **Both** — Side-by-side Messages + Charts

### Visual Design

- Dark theme: backgrounds #1e1e1e / #2d2d2d
- Status colors: green (#00bc8c), red (#e74c3c), yellow (#f1c40f), orange (brownout/a-stop flash)
- Compact layout for driver station laptops
- CSS Modules or Tailwind CSS

## Protocol Reference

### UDP DS → roboRIO (port 1110, 50Hz)

```
[seq_hi][seq_lo][0x01][control][request][alliance][...tags]
```

Control byte bits: 7=estop, 3=fms, 2=enabled, 1-0=mode
Request byte bits: 3=reboot, 2=restart code

### UDP roboRIO → DS (port 1150)

```
[seq_hi][seq_lo][0x01][status][trace][voltage_hi][voltage_lo][request_date][...tags]
```

Status byte bits: 7=estop, 4=code_start, 3=brownout, 2=enabled, 1-0=mode

### TCP (port 1740)

```
[size_hi][size_lo][tag][...payload]
```

Tags: 0x00=messages, 0x0a=version, 0x0b=errors, 0x0c=stdout

### Joystick data tag (0x0c)

```
[axis_count][axis_values...][button_count][button_bytes...][pov_count][pov_values...]
```

Axes: i8 (-128 to 127). Buttons: packed bits LSB first. POVs: i16 (0-360, -1=released).

## Out of Scope

- FMS integration (cannot be implemented without FMS access)
- Built-in dashboard / NetworkTables viewer
- Robot simulation hosting
