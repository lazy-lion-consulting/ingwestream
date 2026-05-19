# Ingwe — Backend Deep Context

## Crate topology

```
src-tauri/src/
├── main.rs       — binary entry; calls lib::run()
├── lib.rs        — tauri::Builder setup (plugins, state, handler, setup, events)
├── state.rs      — AppState { service_view, active_service_id }
├── commands.rs   — #[tauri::command] fns + AppError + helpers + init_service_webview
├── scripts.rs    — JS injection string constants
├── tray.rs       — system tray construction + menu event handler
└── shortcuts.rs  — global media key registration
```

---

## Key dependencies (Cargo.toml)

```toml
tauri                    = { version = "2", features = ["tray-icon", "unstable", "devtools"] }
tauri-plugin-opener      = "2"
tauri-plugin-window-state = "2"
tauri-plugin-global-shortcut = "2"
tauri-plugin-log         = "2"
log                      = "0.4"
thiserror                = "2"
serde                    = { version = "1", features = ["derive"] }
serde_json               = "1"
tokio                    = { version = "1", features = ["time"] }
```

- `unstable` feature is required for `Window::add_child()`.
- `devtools` feature enables the WebView devtools panel in dev builds.
- `tray-icon` feature enables system tray support.

---

## AppState (`state.rs`)

```rust
pub struct AppState {
    pub service_view: Option<tauri::Webview<tauri::Wry>>,  // persistent child webview handle
    pub active_service_id: Option<String>,                  // None when no service is shown
}
```

Stored as `Mutex<AppState>`. Always: lock → take/clone what you need → drop lock → do I/O.
Never hold the lock across async boundaries.

The child webview is created once at startup by `init_service_webview` and lives for the
entire app lifetime. `service_view` is `None` only until setup completes.

---

## AppError (`commands.rs`)

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum AppError {
    #[error("State lock poisoned")]       StatePoisoned,
    #[error("Tauri error: {0}")]          Tauri(String),
    #[error("Invalid URL: {0}")]          InvalidUrl(String),
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self { AppError::Tauri(e.to_string()) }
}
```

All commands return `Result<T, AppError>`. Tauri serialises errors as `{ message: "…" }`.

---

## Startup: `init_service_webview(app)` (non-command)

Called once from `setup` in `lib.rs`. Creates the persistent child webview on the Win32
main thread during app initialisation — the only safe context for `add_child`.

```rust
pub fn init_service_webview(app: &AppHandle) -> Result<(), AppError> {
    // Compute logical size from main window dimensions
    // Call main.add_child("service-view", about:blank, WEBVIEW_DARK_INIT)
    // service_view.hide()
    // Store handle in AppState
}
```

The webview loads `about:blank` and is immediately hidden. `open_service` navigates it
via `eval()` when the user selects a service — no further `add_child` calls ever occur.

**Critical:** Never call `add_child` from a command handler or Tokio thread.
See `.claude/windows-webview2.md` for full threading model documentation.

---

## IPC commands

### `open_service(state, service_id, url)`

Pure navigation — no COM work, safe from any thread.

1. Parse + validate URL → `AppError::InvalidUrl` on failure.
2. Lock state → record `active_service_id`, check `is_same_service`, clone view → drop lock.
3. If not same service: `v.eval("window.location.href = <url>;")`.
4. `v.show()` + `v.set_focus()`.

Returns `Err("service webview not initialized")` if `init_service_webview` failed at startup.

### `close_service(state)`

Lock → set `active_service_id = None`, clone view → drop lock → `view.hide()`.

The webview is **hidden, not destroyed.** Next `open_service` navigates the retained handle.

### `show_service_view(state)` / `hide_service_view(state)`

Lock → clone view handle → drop lock → `view.show()` / `view.hide()`.
Called when the sidebar flyout opens/closes so the service content isn't visible through
the backdrop.

### `resize_service_view(app)` (non-command, called from `on_window_event`)

Called on `WindowEvent::Resized`. Gets main window size, locks state, calls
`view.set_size()` + `view.set_position()` on the child webview if one exists.

### `dispatch_media_key(app, action)` (non-command, called from tray + shortcuts)

Locks state, reads `active_service_id` — returns early if `None` (webview is hidden).
Evals `window.__ingweMedia('action')` on the child webview.

---

## Registering a new command

1. Add `pub fn` with `#[tauri::command]` in `commands.rs`.
2. Add to `invoke_handler` in `lib.rs`: `commands::my_command`.
3. Add permission to `capabilities/default.json`: `"core:invoke:allow-my-command"`.
4. Add TypeScript binding in `src/utils/` or call via `invoke("my_command", { … })`.

---

## lib.rs builder pattern

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_log::Builder::new()…build())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .manage(Mutex::new(AppState::new()))
    .invoke_handler(tauri::generate_handler![
        commands::open_service,
        commands::close_service,
        commands::show_service_view,
        commands::hide_service_view,
    ])
    .setup(|app| {
        tray::build_tray(&app.handle())?;
        shortcuts::register_media_shortcuts(&app.handle())?;  // soft-fail on error
        app.get_webview_window("main").map(|w| { w.show(); w.set_focus(); });
        commands::init_service_webview(&app.handle())?;       // pre-create child webview
        Ok(())
    })
    .on_window_event(|window, event| {
        if window.label() == "main" {
            if let tauri::WindowEvent::Resized(_) = event {
                commands::resize_service_view(window.app_handle());
            }
        }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

`init_service_webview` must come after `w.show()` so the main window HWND is realised
before `add_child` attaches to it.

---

## tray.rs

Menu items: Show Ingwe / Previous / Play-Pause / Next / — / Quit.
Left-click tray icon: show + focus main window.
Media items call `commands::dispatch_media_key(app, action)`.
`build_tray()` called once in setup.

---

## shortcuts.rs

Registers `MediaPlayPause`, `MediaTrackNext`, `MediaTrackPrevious`, `MediaStop` via
`tauri_plugin_global_shortcut`. Dispatches via `commands::dispatch_media_key`.
`register_media_shortcuts` is soft-fail — Linux/Wayland may lack support; error is
printed to stderr with `eprintln!` and setup continues.

---

## scripts.rs — JS constants

`WEBVIEW_DARK_INIT` — injected via `.initialization_script()` on the child webview:

- Appends `<meta name="color-scheme" content="dark">` and baseline dark CSS.
- Overrides `window.matchMedia` so services detect dark mode.
- Installs `window.__ingweMedia(action)` for media key bridging.

Initialization scripts persist for the webview's lifetime in WebView2 and re-execute on
every navigation, so dark mode and the media bridge work correctly on each service switch.

`SUSPEND_SCRIPT` / `RESUME_SCRIPT` — available but not yet actively called (future
background throttling). Suspend freezes timers, mutes audio/video. Resume restores.

---

## Logging

`tauri-plugin-log` writes to:

- `stdout` (visible in `npm run tauri dev` terminal).
- Log file: `{app_local_data_dir}/logs/Ingwe.log` (Windows: `%LOCALAPPDATA%\com.lazylionconsulting.ingwestream\logs\Ingwe.log`).
- Level: `Info` in dev, adjust in `lib.rs` `tauri_plugin_log::Builder::new().level()`.

Use only:

```rust
log::info!("…");
log::warn!("…");
log::error!("…");
```

Never `println!` in production code paths.

---

## Tauri window configuration (`tauri.conf.json`)

```json
{
  "title": "Ingwe",
  "width": 1200, "height": 800,
  "minWidth": 800, "minHeight": 600,
  "decorations": false,   ← frameless; React renders its own titlebar
  "visible": false         ← shown in setup after state is ready
}
```

Child webview is NOT declared in `tauri.conf.json` — it is created by `init_service_webview`
in `setup` and lives for the entire app lifetime.

---

## Capabilities (`capabilities/default.json`)

Window `main` has these permissions:

```
core:default
opener:default
window-state:default
global-shortcut:allow-register / allow-unregister / allow-is-registered
core:window:allow-close / allow-minimize / allow-toggle-maximize
core:window:allow-start-dragging / allow-set-focus / allow-show / allow-hide
```

When adding a new plugin, append its permission identifiers here.
The schema file is at `src-tauri/gen/schemas/desktop-schema.json` — do not edit.

---

## Adding a new streaming service (backend has zero changes)

Services are purely frontend data in `src/services/serviceRegistry.ts`.
The backend accepts any valid HTTPS URL via `open_service`. No Rust changes needed.
