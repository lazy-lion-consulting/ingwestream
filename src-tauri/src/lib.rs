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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Mutex::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![
            commands::open_service,
            commands::switch_service,
            commands::close_service,
        ])
        .setup(|app| {
            tray::build_tray(&app.handle())?;
            if let Err(e) = shortcuts::register_media_shortcuts(&app.handle()) {
                eprintln!("Warning: media key shortcuts unavailable on this system: {e}");
            }

            // Show main window after setup (window starts hidden in tauri.conf.json)
            if let Some(w) = app.get_webview_window("main") {
                w.show()?;
                w.set_focus()?;
            }

            // Background GC timer — every 60 s destroy webviews idle > 10 min
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    if let Some(state) = handle.try_state::<Mutex<AppState>>() {
                        if let Ok(mut s) = state.lock() {
                            commands::gc_idle_webviews(&mut s);
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
