#[cfg(desktop)]
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

/// Set up the system tray for the application
#[cfg(desktop)]
pub fn setup_systray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle();

    // Create tray menu items
    let open_item = MenuItem::with_id(app_handle, "open", "Abrir Jakson ToDo", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app_handle, "quit", "Sair", true, None::<&str>)?;
    
    let menu = Menu::with_items(app_handle, &[&open_item, &quit_item])?;

    let icon = app.default_window_icon()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Default window icon not found in configuration"))?
        .clone();

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app_handle, event| {
            match event.id().as_ref() {
                "open" => {
                    show_main_window(app_handle);
                }
                "quit" => {
                    app_handle.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

/// Helper function to show and focus the main webview window
#[cfg(desktop)]
fn show_main_window(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
