mod commands;
mod events;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Same as main.rs setup but for mobile entry point
    // For now, just the basic setup
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Open Station");
}
