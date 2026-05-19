use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::commands::{dispatch_media_key, toggle_fullscreen_from_shortcut};

pub fn register_media_shortcuts(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Media keys — registered as a group; all must succeed together
    let handle = app.clone();
    app.global_shortcut().on_shortcuts(
        [
            "MediaPlayPause",
            "MediaTrackNext",
            "MediaTrackPrevious",
            "MediaStop",
        ],
        move |_app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            match shortcut.key.to_string().as_str() {
                "MediaPlayPause" => dispatch_media_key(&handle, "play"),
                "MediaTrackNext" => dispatch_media_key(&handle, "next"),
                "MediaTrackPrevious" => dispatch_media_key(&handle, "prev"),
                "MediaStop" => dispatch_media_key(&handle, "stop"),
                _ => {}
            }
        },
    )?;

    // F11 for cinema-mode toggle — registered separately so a failure here
    // never prevents media keys from working
    let handle2 = app.clone();
    if let Err(e) = app.global_shortcut().on_shortcut("F11", move |_app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            toggle_fullscreen_from_shortcut(&handle2);
        }
    }) {
        log::warn!("F11 global shortcut unavailable (cinema exit via button only): {e}");
    }

    Ok(())
}
