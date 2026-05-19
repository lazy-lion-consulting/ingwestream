/// Global application state, wrapped in `Mutex<AppState>` and registered via `.manage()`.
pub struct AppState {
    /// The currently displayed child webview, if any.
    pub service_view: Option<tauri::Webview<tauri::Wry>>,
    /// The id of the currently active service.
    pub active_service_id: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            service_view: None,
            active_service_id: None,
        }
    }
}
