#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod events;

use open_station_core::config::Config;
use open_station_core::state::AppState;
use std::sync::Mutex;

fn main() {
    env_logger::init();

    let config = Config::load();
    let mut app_state = AppState::new(config);

    let state_rx = app_state.subscribe_state();
    let stdout_rx = app_state.take_stdout_rx();
    let message_rx = app_state.take_message_rx();

    tauri::Builder::default()
        .manage(Mutex::new(app_state))
        .setup(move |app| {
            let handle = app.handle().clone();

            // Spawn event emitters
            events::spawn_state_emitter(handle.clone(), state_rx);
            if let Some(rx) = stdout_rx {
                events::spawn_stdout_emitter(handle.clone(), rx);
            }
            if let Some(rx) = message_rx {
                events::spawn_message_emitter(handle.clone(), rx);
            }

            // Spawn run loop
            let run_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(20));
                loop {
                    interval.tick().await;
                    use tauri::Manager;
                    let state = run_handle.state::<Mutex<AppState>>();
                    {
                        let mut s = state.lock().unwrap();
                        s.poll();
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::enable,
            commands::disable,
            commands::estop,
            commands::set_mode,
            commands::set_team_number,
            commands::set_alliance,
            commands::set_game_data,
            commands::set_usb_connection,
            commands::reboot_roborio,
            commands::restart_robot_code,
            commands::start_practice_mode,
            commands::stop_practice_mode,
            commands::set_practice_timing,
            commands::reorder_joysticks,
            commands::lock_joystick,
            commands::unlock_joystick,
            commands::rescan_joysticks,
            commands::launch_dashboard,
            commands::get_config,
            commands::save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Open Station");
}
