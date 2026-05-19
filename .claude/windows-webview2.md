# Ingwe — Windows WebView2 Threading Model & Known Issues

## Architecture summary

On Windows, Tauri uses **wry** which uses **WebView2** (Chromium-based, via EdgeHTML COM API).
WebView2 is a COM Single-Threaded Apartment (STA) component. All WebView2 creation and
event callbacks must be delivered on a thread that is pumping Win32 messages.

wry's `new_as_child` / `new_in_hwnd` calls:

```
new_in_hwnd()
  → CoInitializeEx(COINIT_APARTMENTTHREADED)
  → CreateWindowExW(...)           // create container HWND
  → create_environment(...)        // async, waits via wait_with_pump(rx)
  → create_controller(hwnd, ...)   // async, waits via wait_with_pump(rx)
  → init_webview(...)              // sync setup
```

`wait_with_pump` pumps the calling thread's Win32 message queue via
`MsgWaitForMultipleObjectsEx` + `PeekMessage` + `DispatchMessage` until the COM
completion callback fires and sends on a `mpsc::Sender`.

---

## Rule 1 — Always use `run_on_main_thread`

**Never call `add_child` from a Tokio background thread.**

Tauri command handlers (`#[tauri::command]`) run on Tokio worker threads. Those threads
may not be properly initialised as COM STA, and even if they are, `wait_with_pump` on a
background thread pumps messages that can dispatch other queued work in ways that interfere
with the main event loop.

The correct pattern:

```rust
let (tx, rx) = std::sync::mpsc::channel::<Result<tauri::Webview<tauri::Wry>, tauri::Error>>();
app.run_on_main_thread(move || {
    let result = main_window.add_child(
        WebviewBuilder::new("service-view", WebviewUrl::External(url))
            .initialization_script(WEBVIEW_DARK_INIT),
        position,
        size,
    );
    let _ = tx.send(result);
})?;
let new_view = rx.recv()…?;
```

The closure runs on the Win32 event loop thread (the same STA thread that owns the parent
HWND). `wait_with_pump` inside wry pumps that thread's message queue, WebView2 delivers its
COM callbacks, the channel unblocks, and control returns.

---

## Rule 2 — Never queue two concurrent `add_child` closures

**The `isLoading` guard in the frontend store is mandatory.**

When `wait_with_pump` pumps Win32 messages, it dispatches ALL pending messages — including
the winit user-event that would fire a second `run_on_main_thread` closure. If two
`open_service` calls are in-flight simultaneously:

1. Closure A runs, enters `wait_with_pump` for WebView2 environment creation.
2. `wait_with_pump` processes all messages, including the queued winit event for Closure B.
3. Closure B runs _inside_ Closure A's `wait_with_pump` loop.
4. Closure B also tries to create `"service-view"` → Tauri label-registry conflict OR
   WebView2 reentrancy deadlock (wry's own comment references this exact pattern).
5. The window freezes.

**Fix in place:** `src/store/services.ts` `openService` checks `if (get().isLoading) return`
before calling `invoke`. This prevents the second IPC call from ever being dispatched.

**Never remove or weaken this guard.**

---

## Rule 3 — Never pass `.data_directory()` to WebviewBuilder

`.data_directory()` forces wry to call `CreateCoreWebView2EnvironmentWithOptions` with a
specific user-data path, which creates a brand-new `CoreWebView2Environment`. This is a
second async operation requiring its own `wait_with_pump` chain. On top of the existing
controller creation chain, this:

- Doubles the number of COM async callbacks in flight.
- Increases the window of time during which a second closure could be dispatched.
- Has been observed to cause deadlocks on the Ingwe codebase.

Without `.data_directory()`, wry reuses the default user-data folder shared with the main
WebviewWindow, skipping the environment-creation step.

---

## WebView2 reentrancy reference

From wry's own source comment in `webview2/mod.rs`:

```rust
// Use `dispatch_handler` to schedule the run on the message loop after this callback completes,
// this is needed for `new_window_req_handler` to create new webviews for `NewWindowResponse::Create`
// or it will deadlock, see
// https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/threading-model#reentrancy
```

The same reentrancy applies when two `add_child` closures are dispatched and the first
one's `wait_with_pump` delivers the second.

---

## Diagnostic logging (current state)

`commands.rs` `open_service` emits these log lines in order. If the process hangs,
the last visible line identifies the exact failure point:

```
open_service: closing previous child webview          ← old view teardown
open_service: id=<id> logical_size=<w>x<h>            ← size computed
open_service: dispatching add_child to main thread    ← about to run_on_main_thread
open_service: closure running on main thread — calling add_child   ← closure fired ✓
open_service: add_child returned ok=true/false        ← wry finished
open_service: closure dispatched — waiting on channel ← background thread blocking
open_service: channel resolved — webview handle received          ← success
open_service: child webview created for '<id>'        ← stored in state
```

If logs stop at **"dispatching add_child"** → `run_on_main_thread` itself blocked (rare,
check if main window event loop is processing a blocking operation).

If logs stop at **"closure running on main thread"** → `add_child` is hanging inside wry
(WebView2 `wait_with_pump` never completes). Likely causes:

- A second concurrent `open_service` call was dispatched (frontend guard bypassed).
- A `.data_directory()` is set on the WebviewBuilder (check for regression).
- The system's WebView2 runtime is corrupt or not installed.

---

## `wait_with_pump` internals (wry source)

```rust
// webview2_com crate — schematic
pub fn wait_with_pump<T>(rx: Receiver<T>) -> Result<T> {
    let mut msg = MSG::default();
    loop {
        match rx.try_recv() {
            Ok(result) => return result,
            Err(_) => {
                MsgWaitForMultipleObjectsEx(0, None, SOME_TIMEOUT, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
                while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE) {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);      // ← dispatches ALL window messages
                }
            }
        }
    }
}
```

`DispatchMessageW` dispatches to any HWND's window procedure, including winit's hidden
message window which handles `run_on_main_thread` user events. This is why a queued
second closure can fire during the first closure's WebView2 wait.

---

## Test procedure for Windows

1. Build: `npm run tauri build` on a Windows machine (or CI).
2. Run the `.exe`. Observe logs in the console or `%APPDATA%\ingwe\logs\ingwe.log`.
3. Click a service once. Wait for it to load. Expected: service opens within ~3 s.
4. Rapidly click a different service while the first is loading. Expected: second click
   is silently ignored (loading bar still running), first service loads, then re-clicking
   works.
5. Check that the window stays responsive (can minimise/maximise/drag) throughout.

---

## Other Windows-specific notes

- Window frame: `decorations: false` in `tauri.conf.json`. React renders the custom
  titlebar. `data-tauri-drag-region` attribute on the titlebar div enables dragging.
- DPI: Tauri handles DPI scaling. Use logical pixels in `LogicalSize` / `LogicalPosition`.
- WebView2 devtools: press F12 inside the child webview in dev builds (`devtools` feature).
- Widevine DRM: not yet configured. Requires passing `--enable-features=Widevine` via
  `additional_browser_args` on the `WebviewBuilder` (platform-specific, Windows only).
