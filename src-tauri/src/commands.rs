use std::sync::Mutex;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewBuilder, WebviewUrl};

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
    #[error("HTTP error: {0}")]
    Http(String),
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Tauri(e.to_string())
    }
}

// ── Startup initialisation ────────────────────────────────────────────────────

/// Create the persistent child webview at app startup.
///
/// Called from `setup` on the Win32 main thread — the only safe context for
/// `add_child` / WebView2 COM STA initialisation. Registers the `ingwe-notify`
/// URI scheme so the child page can bridge web Notifications to native.
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

pub fn handle_notify_protocol(app: &AppHandle, uri: String) {
    let title = extract_query_param(&uri, "title").unwrap_or_default();
    let body = extract_query_param(&uri, "body").unwrap_or_default();
    if title.is_empty() {
        return;
    }
    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title(&title)
        .body(&body)
        .show();
    log::info!("notify: title={title:?}");
}

fn extract_query_param(uri: &str, key: &str) -> Option<String> {
    let query = uri.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next()? == key {
            let raw = kv.next().unwrap_or("");
            return Some(url_decode(raw));
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                out.push(byte as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

// ── IPC commands ─────────────────────────────────────────────────────────────

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

#[tauri::command]
pub fn close_service(state: State<'_, Mutex<AppState>>) -> Result<(), AppError> {
    let view = {
        let mut s = state.lock().map_err(|_| AppError::StatePoisoned)?;
        s.active_service_id = None;
        s.service_view.clone()
    };

    if let Some(v) = view {
        log::info!("close_service: hiding service view");
        v.hide()?;
    }

    Ok(())
}

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

#[tauri::command]
pub fn toggle_fullscreen_layout(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let new_fullscreen = {
        let mut s = state.lock().map_err(|_| AppError::StatePoisoned)?;
        s.is_fullscreen = !s.is_fullscreen;
        s.is_fullscreen
    };
    // Webview bounds changes (set_size / set_position) must happen on the Win32
    // main thread — tauri::command handlers run on Tokio's thread pool.
    let app_clone = app.clone();
    if let Some(main) = app.get_webview_window("main") {
        main.run_on_main_thread(move || resize_service_view(&app_clone))
            .map_err(|e| AppError::Tauri(e.to_string()))?;
    }
    app.emit("fullscreen-changed", new_fullscreen)
        .map_err(|e| AppError::Tauri(e.to_string()))?;
    log::info!("toggle_fullscreen_layout: fullscreen={new_fullscreen}");
    Ok(())
}

#[tauri::command]
pub async fn update_window_icon(app: AppHandle, favicon_url: String) -> Result<(), AppError> {
    let bytes = reqwest::get(&favicon_url)
        .await
        .map_err(|e| AppError::Http(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?
        .to_vec();

    match tauri::image::Image::from_bytes(&bytes) {
        Ok(img) => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_icon(img);
            }
        }
        Err(e) => log::warn!("update_window_icon: could not decode favicon: {e}"),
    }
    Ok(())
}

#[tauri::command]
pub fn reset_window_icon(app: AppHandle) -> Result<(), AppError> {
    if let Some(w) = app.get_webview_window("main") {
        if let Some(icon) = app.default_window_icon() {
            w.set_icon(icon.clone())?;
        }
    }
    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

pub fn resize_service_view(app: &AppHandle) {
    let (view, is_fullscreen) = if let Some(state) = app.try_state::<Mutex<AppState>>() {
        if let Ok(s) = state.lock() {
            (s.service_view.clone(), s.is_fullscreen)
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
    let total_h = inner.height as f64 / scale;

    let (y, h) = if is_fullscreen {
        (0.0, total_h)
    } else {
        (TITLEBAR_H, (total_h - TITLEBAR_H).max(0.0))
    };

    let _ = v.set_size(tauri::Size::Logical(LogicalSize::new(w, h)));
    let _ = v.set_position(tauri::Position::Logical(LogicalPosition::new(0.0, y)));
}

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

/// Called from F11 shortcut handler — shortcut callbacks also run off the main
/// thread in Tauri, so we still need run_on_main_thread for the resize.
pub fn toggle_fullscreen_from_shortcut(app: &AppHandle) {
    let state = match app.try_state::<Mutex<AppState>>() {
        Some(s) => s,
        None => return,
    };
    let new_fullscreen = match state.lock() {
        Ok(mut s) => {
            s.is_fullscreen = !s.is_fullscreen;
            s.is_fullscreen
        }
        Err(_) => return,
    };
    let app_clone = app.clone();
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.run_on_main_thread(move || resize_service_view(&app_clone));
    }
    let _ = app.emit("fullscreen-changed", new_fullscreen);
    log::info!("F11 fullscreen: {new_fullscreen}");
}
