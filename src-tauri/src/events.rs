use tauri::{AppHandle, Emitter};
use tokio::sync::watch;
use open_station_core::state::UiState;

/// Spawn a background task that emits "robot-state" events whenever state changes
pub fn spawn_state_emitter(app: AppHandle, mut rx: watch::Receiver<UiState>) {
    tauri::async_runtime::spawn(async move {
        loop {
            if rx.changed().await.is_ok() {
                let state = rx.borrow().clone();
                let _ = app.emit("robot-state", &state);
            }
        }
    });
}

/// Spawn a background task that emits "stdout-message" events
pub fn spawn_stdout_emitter(app: AppHandle, mut rx: tokio::sync::mpsc::UnboundedReceiver<String>) {
    tauri::async_runtime::spawn(async move {
        while let Some(line) = rx.recv().await {
            let _ = app.emit("stdout-message", &line);
        }
    });
}

/// Spawn a background task that emits "tcp-message" events
pub fn spawn_message_emitter(app: AppHandle, mut rx: tokio::sync::mpsc::UnboundedReceiver<open_station_protocol::types::TcpMessage>) {
    tauri::async_runtime::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // Serialize the message appropriately
            let payload = match &msg {
                open_station_protocol::types::TcpMessage::Message(s) => {
                    serde_json::json!({"type": "message", "text": s})
                }
                open_station_protocol::types::TcpMessage::Stdout(s) => {
                    serde_json::json!({"type": "stdout", "text": s})
                }
                open_station_protocol::types::TcpMessage::ErrorReport { details, location, is_error, .. } => {
                    serde_json::json!({
                        "type": if *is_error { "error" } else { "warning" },
                        "details": details,
                        "location": location,
                    })
                }
                open_station_protocol::types::TcpMessage::VersionInfo { name, version, .. } => {
                    serde_json::json!({"type": "version", "name": name, "version": version})
                }
            };
            let _ = app.emit("tcp-message", &payload);
        }
    });
}
