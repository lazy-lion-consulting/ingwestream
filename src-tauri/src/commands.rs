use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::scripts::{RESUME_SCRIPT, SUSPEND_SCRIPT, WEBVIEW_DARK_INIT};
use crate::state::{AppState, WebviewEntry, WebviewState};

#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum AppError {
    #[error("Webview not found: {0}")]
    WebviewNotFound(String),
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

// ── helpers ──────────────────────────────────────────────────────────────────

pub fn suspend_webview(wv: &tauri::WebviewWindow) -> tauri::Result<()> {
    wv.hide()?;
    wv.eval(SUSPEND_SCRIPT)?;
    Ok(())
}

pub fn resume_webview(wv: &tauri::WebviewWindow) -> tauri::Result<()> {
    wv.eval(RESUME_SCRIPT)?;
    wv.show()?;
    wv.set_focus()?;
    Ok(())
}

// ── IPC commands ─────────────────────────────────────────────────────────────

/// Open a new service webview. No-op if one with the same id already exists.
#[tauri::command]
pub fn open_service(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    service_id: String,
    url: String,
) -> Result<(), AppError> {
    // Check under lock, then drop it before building the webview.
    // Holding the mutex across WebviewWindowBuilder::build() deadlocks on
    // Windows because WebView2 initialisation dispatches back to the main
    // thread, which may itself need the state lock.
    {
        let s = state.lock().map_err(|_| AppError::StatePoisoned)?;
        if s.entries.contains_key(&service_id) {
            return Ok(());
        }
    }

    let parsed_url = url
        .parse::<tauri::Url>()
        .map_err(|e| AppError::InvalidUrl(e.to_string()))?;

    let wv = WebviewWindowBuilder::new(&app, &service_id, WebviewUrl::External(parsed_url))
        .initialization_script(WEBVIEW_DARK_INIT)
        .visible(false)
        .title(format!("Ingwe — {}", service_id))
        .inner_size(1200.0, 800.0)
        .build()?;

    // Re-acquire lock to insert. Use entry() so a concurrent open doesn't
    // create a duplicate (the extra window will be dropped immediately).
    let mut s = state.lock().map_err(|_| AppError::StatePoisoned)?;
    s.entries.entry(service_id).or_insert(WebviewEntry {
        window: wv,
        last_active: Instant::now(),
        state: WebviewState::Suspended,
    });

    Ok(())
}

/// Switch the active service. Suspends the previous one after a grace period.
#[tauri::command]
pub fn switch_service(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    service_id: String,
) -> Result<(), AppError> {
    // Collect window handles and update bookkeeping under the lock, then drop
    // the lock before touching any window APIs (show/hide/eval all dispatch to
    // the main thread and will deadlock if the mutex is still held).
    let (new_window, prev_id) = {
        let mut s = state.lock().map_err(|_| AppError::StatePoisoned)?;

        let entry = s
            .entries
            .get_mut(&service_id)
            .ok_or_else(|| AppError::WebviewNotFound(service_id.clone()))?;

        entry.state = WebviewState::Active;
        entry.last_active = Instant::now();
        let new_window = entry.window.clone();

        let prev_id = s.active_service.replace(service_id.clone());

        if let Some(ref prev) = prev_id {
            if prev != &service_id {
                if let Some(prev_entry) = s.entries.get_mut(prev) {
                    prev_entry.state = WebviewState::Suspended;
                }
            }
        }

        (new_window, prev_id)
    }; // mutex released

    resume_webview(&new_window)?;

    if let Some(prev) = prev_id {
        if prev != service_id {
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                crate::commands::delayed_suspend(app_clone, prev);
            });
        }
    }

    Ok(())
}

/// Called after the grace period — suspends if still not active.
pub(crate) fn delayed_suspend(app: AppHandle, service_id: String) {
    let window = if let Some(state) = app.try_state::<Mutex<AppState>>() {
        if let Ok(mut s) = state.lock() {
            if s.active_service.as_deref() != Some(&service_id) {
                if let Some(entry) = s.entries.get_mut(&service_id) {
                    entry.state = WebviewState::Suspended;
                    Some(entry.window.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some(wv) = window {
        let _ = suspend_webview(&wv);
    }
}

/// Close and destroy a service webview, freeing OS resources.
#[tauri::command]
pub fn close_service(
    state: State<'_, Mutex<AppState>>,
    service_id: String,
) -> Result<(), AppError> {
    let window = {
        let mut s = state.lock().map_err(|_| AppError::StatePoisoned)?;

        if s.active_service.as_deref() == Some(&service_id) {
            s.active_service = None;
        }

        s.entries.remove(&service_id).map(|e| e.window)
    }; // mutex released

    if let Some(wv) = window {
        let _ = wv.close();
    }

    Ok(())
}

/// Garbage-collect webviews idle longer than 10 minutes (called on a background timer).
pub fn gc_idle_webviews(state: &mut AppState) {
    let cutoff = std::time::Duration::from_secs(600);
    let now = Instant::now();
    let active = state.active_service.clone();

    let to_destroy: Vec<String> = state
        .entries
        .iter()
        .filter(|(id, e)| {
            active.as_deref() != Some(id.as_str())
                && e.state == WebviewState::Suspended
                && now.duration_since(e.last_active) > cutoff
        })
        .map(|(id, _)| id.clone())
        .collect();

    for id in to_destroy {
        if let Some(entry) = state.entries.remove(&id) {
            let _ = entry.window.close();
        }
    }
}

/// Dispatch a media action string to the active webview's JS bridge.
pub fn dispatch_media_key(app: &AppHandle, action: &str) {
    let window = if let Some(state) = app.try_state::<Mutex<AppState>>() {
        if let Ok(s) = state.lock() {
            s.active_service
                .as_ref()
                .and_then(|id| s.entries.get(id))
                .map(|e| e.window.clone())
        } else {
            None
        }
    } else {
        None
    };

    if let Some(wv) = window {
        let js = format!("window.__ingweMedia('{}');", action);
        let _ = wv.eval(&js);
    }
}

