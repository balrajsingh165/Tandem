//! Taskbar tray: keeps Tandem reachable while its window is hidden, with a
//! left-click to reveal and a menu offering Open and Quit. Quit is the only path
//! that actually exits, since closing the window only hides it.

use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Tandem", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    TrayIconBuilder::with_id("tandem")
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound("no window icon to use for the tray".into())
        })?)
        .tooltip("Tandem")
        .menu(&menu)
        // Without this the left click only opens the menu, and the obvious
        // gesture — click the tray icon to get the dialer — would do nothing.
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu)
        .on_tray_icon_event(handle_icon)
        .build(app)?;

    Ok(())
}

fn handle_menu<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id.as_ref() {
        "open" => reveal(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn handle_icon<R: Runtime>(tray: &tauri::tray::TrayIcon<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        reveal(tray.app_handle());
    }
}

fn reveal<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
