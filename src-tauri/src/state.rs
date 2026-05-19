pub struct AppState {
    pub service_view: Option<tauri::Webview<tauri::Wry>>,
    pub active_service_id: Option<String>,
    pub is_fullscreen: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            service_view: None,
            active_service_id: None,
            is_fullscreen: false,
        }
    }
}
