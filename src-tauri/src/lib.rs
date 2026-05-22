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
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .register_uri_scheme_protocol("ingwe-notify", |ctx, req| {
            commands::handle_notify_protocol(ctx.app_handle(), req.uri().to_string());
            tauri::http::Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .body(std::borrow::Cow::Borrowed(b"" as &[u8]))
                .unwrap()
        })
        .register_uri_scheme_protocol("ingwe-ctrl", |ctx, req| {
            commands::handle_ctrl_protocol(ctx.app_handle(), req.uri().to_string());
            tauri::http::Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .body(std::borrow::Cow::Borrowed(b"" as &[u8]))
                .unwrap()
        })
        .manage(Mutex::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![
            commands::open_service,
            commands::reset_service,
            commands::close_service,
            commands::show_service_view,
            commands::hide_service_view,
            commands::toggle_fullscreen_layout,
            commands::apply_fullscreen_resize,
            commands::update_window_icon,
            commands::reset_window_icon,
            commands::show_titlebar_overlay,
            commands::get_work_area,
            commands::ctrl_action,
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

            commands::init_service_webview(&app.handle())?;

            // Belt-and-suspenders: re-apply the service view resize after a short
            // delay so that any window-state plugin restoration that runs after
            // setup() has finished is reflected in the child webview bounds.
            // Two passes (200 ms + 600 ms) cover both fast and slow state-restore
            // timing, particularly on macOS where layout can settle later.
            {
                let h = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    for delay in [200u64, 600] {
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        if let Some(w) = h.get_webview_window("main") {
                            let h2 = h.clone();
                            let _ = w.run_on_main_thread(move || {
                                commands::resize_service_view(&h2);
                            });
                        }
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                // Capture physical size and scale synchronously from the event/window
                // so the main-thread closure receives the correct dimensions.
                // Re-reading inner_size() inside run_on_main_thread is unreliable on
                // macOS — WKWebView layout can settle after the event fires, causing
                // the child webview to be positioned against a stale/incorrect size.
                match event {
                    tauri::WindowEvent::Resized(size) => {
                        let pw = size.width;
                        let ph = size.height;
                        let scale = window.scale_factor().unwrap_or(1.0);
                        let app = window.app_handle().clone();
                        if let Some(w) = window.app_handle().get_webview_window("main") {
                            let _ = w.run_on_main_thread(move || {
                                commands::resize_service_view_sized(&app, pw, ph, scale);
                            });
                        }
                    }
                    tauri::WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size, .. } => {
                        let pw = new_inner_size.width;
                        let ph = new_inner_size.height;
                        let scale = *scale_factor;
                        let app = window.app_handle().clone();
                        if let Some(w) = window.app_handle().get_webview_window("main") {
                            let _ = w.run_on_main_thread(move || {
                                commands::resize_service_view_sized(&app, pw, ph, scale);
                            });
                        }
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
