use crate::config::Config;
use crate::hotkeys::HotkeyManager;
use crate::input::JoystickManager;
use crate::practice::PracticeMode;
use open_station_protocol::driver_station::{DriverStation, DsReceiver};
use open_station_protocol::types::*;
use serde::Serialize;
use tokio::sync::watch;

/// Flattened state for the UI — serialized and sent via Tauri events
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
    // Joysticks
    pub joysticks: Vec<JoystickInfoSerialized>,
    pub any_joystick_connected: bool,
    // Practice
    pub practice_phase: String,
    pub practice_elapsed_secs: f64,
    pub practice_remaining_secs: f64,
    // Connection
    pub trip_time_ms: f64,
    pub lost_packets: u32,
    // Meta
    pub team_number: u32,
    pub alliance_color: String,
    pub alliance_station: u8,
}

/// Serializable joystick info for the frontend
#[derive(Debug, Clone, Serialize)]
pub struct JoystickInfoSerialized {
    pub slot: u8,
    pub uuid: String,
    pub name: String,
    pub locked: bool,
    pub connected: bool,
    pub axis_count: u8,
    pub button_count: u8,
    pub pov_count: u8,
    pub axes: Vec<i8>,
    pub buttons: Vec<bool>,
    pub povs: Vec<i16>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            connected: false,
            code_running: false,
            voltage: 0.0,
            brownout: false,
            estopped: false,
            enabled: false,
            mode: "Teleoperated".to_string(),
            joysticks: Vec::new(),
            any_joystick_connected: false,
            practice_phase: "Idle".to_string(),
            practice_elapsed_secs: 0.0,
            practice_remaining_secs: 0.0,
            trip_time_ms: 0.0,
            lost_packets: 0,
            team_number: 0,
            alliance_color: "Red".to_string(),
            alliance_station: 1,
        }
    }
}

pub struct AppState {
    ds: DriverStation,
    #[allow(dead_code)] // Will be used in run loop (Task 13)
    ds_rx: Option<DsReceiver>,
    pub joysticks: JoystickManager,
    practice: PracticeMode,
    #[allow(dead_code)] // Will be used in run loop (Task 13)
    hotkeys: HotkeyManager,
    config: Config,

    // Current state
    mode: Mode,
    alliance: Alliance,
    enabled: bool,

    // Outbound
    ui_state_tx: watch::Sender<UiState>,
    ui_state_rx: watch::Receiver<UiState>,

    // Stdout forwarding
    #[allow(dead_code)] // Will be used in run loop (Task 13)
    stdout_tx: tokio::sync::mpsc::UnboundedSender<String>,
    stdout_rx: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,

    // Message forwarding
    #[allow(dead_code)] // Will be used in run loop (Task 13)
    message_tx: tokio::sync::mpsc::UnboundedSender<TcpMessage>,
    message_rx: Option<tokio::sync::mpsc::UnboundedReceiver<TcpMessage>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let alliance = Alliance::new(AllianceColor::Red, 1);
        let (ds, ds_rx) = DriverStation::new(config.team_number, alliance);
        let joysticks = JoystickManager::new(config.joystick_locks.clone());
        let practice = PracticeMode::new(config.practice_timing.clone());
        let hotkeys = HotkeyManager::new();

        let (ui_state_tx, ui_state_rx) = watch::channel(UiState::default());
        let (stdout_tx, stdout_rx) = tokio::sync::mpsc::unbounded_channel();
        let (message_tx, message_rx) = tokio::sync::mpsc::unbounded_channel();

        let app_state = Self {
            ds,
            ds_rx: Some(ds_rx),
            joysticks,
            practice,
            hotkeys,
            config,
            mode: Mode::Teleop,
            alliance,
            enabled: false,
            ui_state_tx,
            ui_state_rx,
            stdout_tx,
            stdout_rx: Some(stdout_rx),
            message_tx,
            message_rx: Some(message_rx),
        };

        app_state.update_ui_state();
        app_state
    }

    /// Get a receiver for UI state updates
    pub fn subscribe_state(&self) -> watch::Receiver<UiState> {
        self.ui_state_rx.clone()
    }

    /// Take the stdout receiver (can only be called once)
    pub fn take_stdout_rx(&mut self) -> Option<tokio::sync::mpsc::UnboundedReceiver<String>> {
        self.stdout_rx.take()
    }

    /// Take the message receiver (can only be called once)
    pub fn take_message_rx(&mut self) -> Option<tokio::sync::mpsc::UnboundedReceiver<TcpMessage>> {
        self.message_rx.take()
    }

    // === Commands (called from Tauri) ===

    pub fn enable(&mut self) {
        self.ds.enable();
        self.enabled = true;
        self.update_ui_state();
    }

    pub fn disable(&mut self) {
        self.ds.disable();
        self.enabled = false;
        self.update_ui_state();
    }

    pub fn estop(&mut self) {
        self.ds.estop();
        self.enabled = false;
        self.update_ui_state();
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.ds.set_mode(mode);
        self.update_ui_state();
    }

    pub fn set_team(&mut self, team: u32) {
        self.config.team_number = team;
        self.ds.set_team(team);
        self.update_ui_state();
    }

    pub fn set_alliance(&mut self, alliance: Alliance) {
        self.alliance = alliance;
        self.ds.set_alliance(alliance);
        self.update_ui_state();
    }

    pub fn set_game_data(&mut self, data: String) {
        self.config.game_data = data.clone();
        self.ds.set_game_data(data);
    }

    pub fn set_usb_mode(&mut self, usb: bool) {
        self.config.use_usb = usb;
        self.ds.set_usb_mode(usb);
    }

    pub fn reboot_roborio(&mut self) {
        self.ds.reboot_roborio();
    }

    pub fn restart_code(&mut self) {
        self.ds.restart_code();
    }

    pub fn start_practice(&mut self) {
        self.practice.start();
    }

    pub fn stop_practice(&mut self) {
        self.practice.stop();
        self.disable();
    }

    pub fn a_stop(&mut self) {
        self.practice.a_stop();
        self.disable();
    }

    pub fn set_practice_timing(&mut self, timing: crate::config::PracticeTiming) {
        self.config.practice_timing = timing.clone();
        self.practice.set_timing(timing);
    }

    pub fn reorder_joysticks(&mut self, order: Vec<String>) {
        self.joysticks.reorder(order);
        self.update_ui_state();
    }

    pub fn lock_joystick(&mut self, uuid: String, slot: u8) {
        self.joysticks.lock(&uuid, slot);
        self.update_ui_state();
    }

    pub fn unlock_joystick(&mut self, uuid: String) {
        self.joysticks.unlock(&uuid);
        self.update_ui_state();
    }

    pub fn rescan_joysticks(&mut self) {
        self.joysticks.rescan();
        self.update_ui_state();
    }

    pub fn poll(&mut self) {
        self.joysticks.poll();
        self.update_ui_state();
    }

    pub fn launch_dashboard(&self) {
        if let Some(cmd) = &self.config.dashboard_command {
            let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
        }
    }

    pub fn save_config(&self) {
        let _ = self.config.save();
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    // === Internal ===

    fn build_ui_state(&self) -> UiState {
        let joystick_data = self.joysticks.get_joystick_data();
        let joystick_info: Vec<JoystickInfoSerialized> = self
            .joysticks
            .get_joystick_info()
            .into_iter()
            .map(|j| {
                let data = joystick_data.get(j.slot as usize);
                JoystickInfoSerialized {
                    slot: j.slot,
                    uuid: j.uuid,
                    name: j.name,
                    locked: j.locked,
                    connected: j.connected,
                    axis_count: j.axis_count,
                    button_count: j.button_count,
                    pov_count: j.pov_count,
                    axes: data.map(|d| d.axes.clone()).unwrap_or_default(),
                    buttons: data.map(|d| d.buttons.clone()).unwrap_or_default(),
                    povs: data.map(|d| d.povs.clone()).unwrap_or_default(),
                }
            })
            .collect();

        let (alliance_color, alliance_station) = match self.alliance.color {
            AllianceColor::Red => ("Red".to_string(), self.alliance.station),
            AllianceColor::Blue => ("Blue".to_string(), self.alliance.station),
        };

        let practice_phase = format!("{:?}", self.practice.phase());

        UiState {
            connected: false, // Updated from DS receiver in run loop
            code_running: false,
            voltage: 0.0,
            brownout: false,
            estopped: self.ds.is_estopped(),
            enabled: self.enabled,
            mode: format!("{}", self.mode),
            joysticks: joystick_info,
            any_joystick_connected: self.joysticks.any_connected(),
            practice_phase,
            practice_elapsed_secs: 0.0,
            practice_remaining_secs: 0.0,
            trip_time_ms: 0.0,
            lost_packets: 0,
            team_number: self.config.team_number,
            alliance_color,
            alliance_station,
        }
    }

    fn update_ui_state(&self) {
        let _ = self.ui_state_tx.send(self.build_ui_state());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_app_state() {
        let config = Config::default();
        let state = AppState::new(config);
        let ui = state.build_ui_state();
        assert_eq!(ui.team_number, 0);
        assert!(!ui.connected);
        assert!(!ui.enabled);
        assert_eq!(ui.mode, "Teleoperated");
        assert_eq!(ui.alliance_color, "Red");
        assert_eq!(ui.alliance_station, 1);
    }

    #[test]
    fn test_set_team() {
        let mut state = AppState::new(Config::default());
        state.set_team(1234);
        let ui = state.build_ui_state();
        assert_eq!(ui.team_number, 1234);
    }

    #[test]
    fn test_mode_switching() {
        let mut state = AppState::new(Config::default());
        state.set_mode(Mode::Autonomous);
        let ui = state.build_ui_state();
        assert_eq!(ui.mode, "Autonomous");
    }

    #[test]
    fn test_enable_disable() {
        let mut state = AppState::new(Config::default());
        state.enable();
        assert!(state.build_ui_state().enabled);
        state.disable();
        assert!(!state.build_ui_state().enabled);
    }

    #[test]
    fn test_estop() {
        let mut state = AppState::new(Config::default());
        state.enable();
        state.estop();
        let ui = state.build_ui_state();
        assert!(ui.estopped);
        assert!(!ui.enabled);
    }
}
