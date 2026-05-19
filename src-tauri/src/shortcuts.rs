use std::panic::{self, AssertUnwindSafe};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::commands::{dispatch_media_key, toggle_fullscreen_from_shortcut};

pub fn register_media_shortcuts(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("register_media_shortcuts: starting");

    let media_keys: &[(&str, &str)] = &[
        ("MediaPlayPause", "play"),
        ("MediaTrackNext", "next"),
        ("MediaTrackPrevious", "prev"),
        ("MediaStop", "stop"),
    ];

    let mut registered = 0usize;
    for &(key, action) in media_keys {
        log::info!("register_media_shortcuts: attempting {key}");
        let handle = app.clone();
        let action_str = action.to_string();

        // Wrap in catch_unwind — on Linux/WSLg the X11 key-grab can panic when
        // the desktop environment has already claimed the media keys.
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            app.global_shortcut().on_shortcut(key, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    dispatch_media_key(&handle, &action_str);
                }
            })
        }));

        match result {
            Ok(Ok(_)) => {
                registered += 1;
                log::info!("register_media_shortcuts: registered {key}");
            }
            Ok(Err(e)) => log::warn!("register_media_shortcuts: could not register {key}: {e}"),
            Err(_) => log::warn!("register_media_shortcuts: {key} panicked during registration (likely grabbed by DE)"),
        }
    }

    log::info!("register_media_shortcuts: {registered}/{} registered via global shortcut", media_keys.len());

    log::info!("register_media_shortcuts: attempting F11");
    let handle2 = app.clone();
    let f11_result = panic::catch_unwind(AssertUnwindSafe(|| {
        app.global_shortcut().on_shortcut("F11", move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_fullscreen_from_shortcut(&handle2);
            }
        })
    }));
    match f11_result {
        Ok(Ok(_)) => log::info!("register_media_shortcuts: registered F11"),
        Ok(Err(e)) => log::warn!("register_media_shortcuts: F11 unavailable: {e}"),
        Err(_) => log::warn!("register_media_shortcuts: F11 panicked during registration"),
    }

    Ok(())
}
