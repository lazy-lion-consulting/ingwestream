use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::commands::dispatch_media_key;

pub fn register_media_shortcuts(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
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
            let action = match shortcut.key.to_string().as_str() {
                "MediaPlayPause" => "play",
                "MediaTrackNext" => "next",
                "MediaTrackPrevious" => "prev",
                "MediaStop" => "stop",
                _ => return,
            };
            dispatch_media_key(&handle, action);
        },
    )?;
    Ok(())
}
