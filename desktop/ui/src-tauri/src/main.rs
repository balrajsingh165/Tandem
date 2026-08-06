//! Tauri shell entry: creates the window, the taskbar tray with its menu, and
//! spawns daemon_bridge for IPC forwarding. Contains no call logic (docs/14
//! layering).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod daemon_bridge;
mod tray;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            daemon_bridge::spawn_event_stream(app.handle().clone());
            tray::install(app.handle())?;
            Ok(())
        })
        // Closing the window hides it instead: a call can arrive at any time, and
        // a dialer that has to be relaunched to answer is useless.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![daemon_bridge::daemon_request])
        .run(tauri::generate_context!())
        .expect("tandem-ui failed to start");
}
