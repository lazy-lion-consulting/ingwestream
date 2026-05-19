# Ingwe — Backend Deep Context

## Crate topology

```
src-tauri/src/
├── main.rs       — binary entry; calls lib::run()
├── lib.rs        — tauri::Builder setup (plugins, state, handler, setup, events)
├── state.rs      — AppState { service_view, active_service_id }
├── commands.rs   — #[tauri::command] fns + AppError + helpers
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
    pub service_view: Option<tauri::Webview<tauri::Wry>>,   // current child webview handle
    pub active_service_id: Option<String>,
}
```

Stored as `Mutex<AppState>`. Always: lock → take/clone what you need → drop lock → do I/O.
Never hold the lock across async boundaries or across `add_child`.

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

## IPC commands — current implementation

### `open_service(app, state, service_id, url)`

1. Parse + validate URL → `AppError::InvalidUrl` on failure.
2. Lock state → take `old_view` + set `active_service_id` → drop lock.
3. Close old view outside the lock: `old_view.close()`.
4. Get main window → compute logical size (subtract 32px titlebar).
5. Dispatch `add_child` closure to Win32 main thread via `app.run_on_main_thread()`.
6. Block on `mpsc::channel` until closure reports result.
7. Lock state again → store new view handle.

**Critical:** `run_on_main_thread` is mandatory on Windows — see `.claude/windows-webview2.md`.
**Critical:** Do NOT pass `.data_directory()` to `WebviewBuilder` — see `.claude/windows-webview2.md`.

### `close_service(state)`

Lock → take view → drop lock → `view.close()`.

### `show_service_view(state)` / `hide_service_view(state)`

Lock → clone view handle → drop lock → `view.show()` / `view.hide()`.

### `resize_service_view(app)` (non-command, called from `on_window_event`)

Called on `WindowEvent::Resized`. Gets main window size, locks state, calls
`view.set_size()` on the child webview if one exists.

### `dispatch_media_key(app, action)` (non-command, called from tray + shortcuts)

Locks state, reads `active_service_id`, evals `window.__ingweMedia('action')` on the
child webview.

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

`SUSPEND_SCRIPT` / `RESUME_SCRIPT` — available but not yet actively called (future
background throttling). Suspend freezes timers, mutes audio/video. Resume restores.

---

## Logging

`tauri-plugin-log` writes to:

- `stdout` (visible in `npm run tauri dev` terminal).
- Log file: `{app_data_dir}/ingwe.log`.
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

Child webview is NOT declared in `tauri.conf.json` — it is created dynamically via
`Window::add_child()` at runtime.

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
