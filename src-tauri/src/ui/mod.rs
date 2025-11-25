use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

pub mod admin;
mod config_bridge;
pub mod user;

use crate::agent::config::AgentConfig;

#[tauri::command]
async fn get_agent_config() -> Result<AgentConfig, String> {
    config_bridge::load_config().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_agent_config(config: AgentConfig) -> Result<(), String> {
    config_bridge::save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_admin_privileges() -> Result<bool, String> {
    Ok(config_bridge::is_admin())
}

fn show_settings_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => {
                show_settings_window(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                show_settings_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_agent_config,
            save_agent_config,
            check_admin_privileges
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("Failed to run UI: {}", e))?;

    Ok(())
}
