mod commands;
mod scripts;
mod shortcuts;
mod state;
mod tray;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("ingwe".into()),
                    },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Mutex::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![
            commands::open_service,
            commands::close_service,
            commands::show_service_view,
            commands::hide_service_view,
        ])
        .setup(|app| {
            tray::build_tray(&app.handle())?;
            if let Err(e) = shortcuts::register_media_shortcuts(&app.handle()) {
                eprintln!("Warning: media key shortcuts unavailable on this system: {e}");
            }

            if let Some(w) = app.get_webview_window("main") {
                w.show()?;
                w.set_focus()?;
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::Resized(_) = event {
                    commands::resize_service_view(window.app_handle());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
