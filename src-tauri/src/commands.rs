use open_station_core::config::PracticeTiming;
use open_station_core::state::AppState;
use open_station_protocol::types::*;
use std::sync::Mutex;
use tauri::State;

pub type AppStateHandle = Mutex<AppState>;

#[tauri::command]
pub fn enable(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().enable();
}

#[tauri::command]
pub fn disable(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().disable();
}

#[tauri::command]
pub fn estop(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().estop();
}

#[tauri::command]
pub fn set_mode(state: State<'_, AppStateHandle>, mode: String) {
    let m = match mode.as_str() {
        "teleop" | "Teleoperated" => Mode::Teleop,
        "autonomous" | "Autonomous" => Mode::Autonomous,
        "test" | "Test" => Mode::Test,
        _ => return,
    };
    state.lock().unwrap().set_mode(m);
}

#[tauri::command]
pub fn set_team_number(state: State<'_, AppStateHandle>, team: u32) {
    state.lock().unwrap().set_team(team);
}

#[tauri::command]
pub fn set_alliance(state: State<'_, AppStateHandle>, color: String, station: u8) {
    let c = match color.as_str() {
        "Red" | "red" => AllianceColor::Red,
        "Blue" | "blue" => AllianceColor::Blue,
        _ => return,
    };
    if station >= 1 && station <= 3 {
        state
            .lock()
            .unwrap()
            .set_alliance(Alliance::new(c, station));
    }
}

#[tauri::command]
pub fn set_game_data(state: State<'_, AppStateHandle>, data: String) {
    state.lock().unwrap().set_game_data(data);
}

#[tauri::command]
pub fn set_usb_connection(state: State<'_, AppStateHandle>, enabled: bool) {
    state.lock().unwrap().set_usb_mode(enabled);
}

#[tauri::command]
pub fn reboot_roborio(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().reboot_roborio();
}

#[tauri::command]
pub fn restart_robot_code(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().restart_code();
}

#[tauri::command]
pub fn start_practice_mode(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().start_practice();
}

#[tauri::command]
pub fn stop_practice_mode(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().stop_practice();
}

#[tauri::command]
pub fn set_practice_timing(
    state: State<'_, AppStateHandle>,
    countdown: u32,
    auto_secs: u32,
    delay: u32,
    teleop: u32,
) {
    state.lock().unwrap().set_practice_timing(PracticeTiming {
        countdown_secs: countdown,
        auto_secs,
        delay_secs: delay,
        teleop_secs: teleop,
    });
}

#[tauri::command]
pub fn reorder_joysticks(state: State<'_, AppStateHandle>, order: Vec<String>) {
    state.lock().unwrap().reorder_joysticks(order);
}

#[tauri::command]
pub fn lock_joystick(state: State<'_, AppStateHandle>, uuid: String, slot: u8) {
    state.lock().unwrap().lock_joystick(uuid, slot);
}

#[tauri::command]
pub fn unlock_joystick(state: State<'_, AppStateHandle>, uuid: String) {
    state.lock().unwrap().unlock_joystick(uuid);
}

#[tauri::command]
pub fn rescan_joysticks(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().rescan_joysticks();
}

#[tauri::command]
pub fn launch_dashboard(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().launch_dashboard();
}

#[tauri::command]
pub fn get_config(state: State<'_, AppStateHandle>) -> serde_json::Value {
    let s = state.lock().unwrap();
    let config = s.config();
    serde_json::json!({
        "team_number": config.team_number,
        "use_usb": config.use_usb,
        "dashboard_command": config.dashboard_command,
        "game_data": config.game_data,
        "practice_timing": {
            "countdown_secs": config.practice_timing.countdown_secs,
            "auto_secs": config.practice_timing.auto_secs,
            "delay_secs": config.practice_timing.delay_secs,
            "teleop_secs": config.practice_timing.teleop_secs,
        },
        "practice_audio": config.practice_audio,
    })
}

#[tauri::command]
pub fn save_config(state: State<'_, AppStateHandle>) {
    state.lock().unwrap().save_config();
}
