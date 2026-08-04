//! Tauri shell entry: creates the window, tray icon, and notification bridge,
//! and spawns daemon_bridge for IPC forwarding. Contains no call logic (docs/14
//! layering).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod daemon_bridge;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![daemon_bridge::daemon_request])
        .run(tauri::generate_context!())
        .expect("tandem-ui failed to start");
}
