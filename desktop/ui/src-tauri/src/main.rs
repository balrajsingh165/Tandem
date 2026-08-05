//! Tauri shell entry: creates the window, tray icon, and notification bridge,
//! and spawns daemon_bridge for IPC forwarding. Contains no call logic (docs/14
//! layering).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod daemon_bridge;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            fit_webview_to_window(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![daemon_bridge::daemon_request])
        .run(tauri::generate_context!())
        .expect("tandem-ui failed to start");
}

/// On Windows the webview lays out against the window's physical width and then
/// rasterizes at the monitor scale, so at any scaling above 100% the page is
/// wider than the window and the right edge is clipped. Compensating with zoom
/// makes one CSS pixel map to one window pixel again.
fn fit_webview_to_window(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(scale) = window.scale_factor() else {
        return;
    };
    if scale > 1.0 {
        let _ = window.set_zoom(1.0 / scale);
    }
}
