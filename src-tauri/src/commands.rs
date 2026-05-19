use std::sync::Mutex;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, State, WebviewBuilder, WebviewUrl};

use crate::scripts::WEBVIEW_DARK_INIT;
use crate::state::AppState;

const TITLEBAR_H: f64 = 32.0;

#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum AppError {
    #[error("State lock poisoned")]
    StatePoisoned,
    #[error("Tauri error: {0}")]
    Tauri(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Tauri(e.to_string())
    }
}

// ── Startup initialisation ────────────────────────────────────────────────────

/// Create the persistent child webview at app startup.
///
/// Called from `setup` which runs on the Win32 main thread — the only safe
/// context for `add_child` / WebView2 COM STA initialisation. The webview
/// loads `about:blank` and is immediately hidden; `open_service` navigates
/// it via `eval()` with no further COM work required.
pub fn init_service_webview(app: &AppHandle) -> Result<(), AppError> {
    let main = app
        .get_window("main")
        .ok_or_else(|| AppError::Tauri("main window not found".into()))?;

    let inner = main.inner_size()?;
    let scale = main.scale_factor()?;
    let w = inner.width as f64 / scale;
    let h = (inner.height as f64 / scale) - TITLEBAR_H;

    let blank_url = "about:blank"
        .parse::<tauri::Url>()
        .map_err(|e| AppError::InvalidUrl(e.to_string()))?;

    log::info!("init_service_webview: creating child webview logical_size={w:.0}x{h:.0}");
    let service_view = main.add_child(
        WebviewBuilder::new("service-view", WebviewUrl::External(blank_url))
            .initialization_script(WEBVIEW_DARK_INIT),
        LogicalPosition::new(0.0, TITLEBAR_H),
        LogicalSize::new(w, h.max(0.0)),
    )?;
    service_view.hide()?;
    log::info!("init_service_webview: child webview created and hidden");

    let state = app
        .try_state::<Mutex<AppState>>()
        .ok_or_else(|| AppError::Tauri("AppState not managed".into()))?;
    let mut s = state.lock().map_err(|_| AppError::StatePoisoned)?;
    s.service_view = Some(service_view);
    Ok(())
}

// ── IPC commands ─────────────────────────────────────────────────────────────

/// Navigate the persistent child webview to `url` and show it.
///
/// The webview was created in `setup`; this command only needs `eval()` to
/// change the page, which is async and safe to call from any thread.
#[tauri::command]
pub fn open_service(
    state: State<'_, Mutex<AppState>>,
    service_id: String,
    url: String,
) -> Result<(), AppError> {
    let parsed_url = url
        .parse::<tauri::Url>()
        .map_err(|e| AppError::InvalidUrl(e.to_string()))?;

    let (view, is_same_service) = {
        let mut s = state.lock().map_err(|_| AppError::StatePoisoned)?;
        let same = s.active_service_id.as_deref() == Some(service_id.as_str());
        s.active_service_id = Some(service_id.clone());
        (s.service_view.clone(), same)
    };

    let v = view.ok_or_else(|| AppError::Tauri("service webview not initialized".into()))?;

    log::info!("open_service: id={service_id} same_service={is_same_service}");
    if !is_same_service {
        let url_json = serde_json::to_string(parsed_url.as_str())
            .map_err(|e| AppError::Tauri(e.to_string()))?;
        v.eval(&format!("window.location.href = {url_json};"))?;
    }
    v.show()?;
    v.set_focus()?;
    Ok(())
}

/// Hide the active service child webview. The webview is kept alive so the
/// next `open_service` call can resume or navigate instantly.
#[tauri::command]
pub fn close_service(state: State<'_, Mutex<AppState>>) -> Result<(), AppError> {
    let view = {
        let mut s = state.lock().map_err(|_| AppError::StatePoisoned)?;
        s.active_service_id = None;
        s.service_view.clone()
    };

    if let Some(v) = view {
        log::info!("close_service: hiding service view (webview retained for reuse)");
        v.hide()?;
    }

    Ok(())
}

/// Show the service child webview — called when the flyout closes.
#[tauri::command]
pub fn show_service_view(state: State<'_, Mutex<AppState>>) -> Result<(), AppError> {
    let view = {
        let s = state.lock().map_err(|_| AppError::StatePoisoned)?;
        s.service_view.clone()
    };

    if let Some(v) = view {
        v.show()?;
        v.set_focus()?;
    }

    Ok(())
}

/// Hide the service child webview — called when the flyout opens.
#[tauri::command]
pub fn hide_service_view(state: State<'_, Mutex<AppState>>) -> Result<(), AppError> {
    let view = {
        let s = state.lock().map_err(|_| AppError::StatePoisoned)?;
        s.service_view.clone()
    };

    if let Some(v) = view {
        v.hide()?;
    }

    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Resize the service child webview to fill the main window content area.
/// Called from the `WindowEvent::Resized` handler in lib.rs.
pub fn resize_service_view(app: &AppHandle) {
    let view = if let Some(state) = app.try_state::<Mutex<AppState>>() {
        if let Ok(s) = state.lock() {
            s.service_view.clone()
        } else {
            return;
        }
    } else {
        return;
    };

    let Some(v) = view else { return };
    let Some(main) = app.get_window("main") else {
        return;
    };
    let Ok(inner) = main.inner_size() else { return };
    let Ok(scale) = main.scale_factor() else {
        return;
    };

    let w = inner.width as f64 / scale;
    let h = (inner.height as f64 / scale) - TITLEBAR_H;

    let _ = v.set_size(tauri::Size::Logical(LogicalSize::new(w, h.max(0.0))));
    let _ = v.set_position(tauri::Position::Logical(LogicalPosition::new(
        0.0, TITLEBAR_H,
    )));
}

/// Forward a media key action to the active service webview via the injected JS bridge.
/// Guards on `active_service_id` — the webview persists even when "closed" (hidden),
/// so we must not dispatch to it unless a service is logically active.
pub fn dispatch_media_key(app: &AppHandle, action: &str) {
    let view = if let Some(state) = app.try_state::<Mutex<AppState>>() {
        if let Ok(s) = state.lock() {
            if s.active_service_id.is_some() {
                s.service_view.clone()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some(v) = view {
        let js = format!("if(window.__ingweMedia)window.__ingweMedia('{}');", action);
        let _ = v.eval(&js);
    }
}
