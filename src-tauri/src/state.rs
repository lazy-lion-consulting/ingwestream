use std::collections::HashMap;
use std::time::Instant;

/// Lifecycle state of a managed WebviewWindow.
#[derive(Debug, PartialEq, Clone)]
pub enum WebviewState {
    Active,
    Suspended,
    Destroyed,
}

/// A single managed streaming-service webview.
pub struct WebviewEntry {
    pub window: tauri::WebviewWindow,
    pub last_active: Instant,
    pub state: WebviewState,
}

/// Global application state, wrapped in `Mutex<AppState>` and registered via `.manage()`.
pub struct AppState {
    /// All known webview entries, keyed by service id (e.g. `"spotify"`, `"youtube"`).
    pub entries: HashMap<String, WebviewEntry>,
    /// The service id that is currently visible to the user.
    pub active_service: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            active_service: None,
        }
    }
}
