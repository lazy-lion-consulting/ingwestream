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

## Solution: pre-create in `setup`, navigate via `eval()`

**The fundamental fix is to call `add_child` exactly once, during `setup`, before any
WebView2 IPC traffic or user interaction.** The child webview starts on `about:blank` and
is immediately hidden. Service switching navigates it via `eval()`:

```rust
// In setup (Win32 main thread, clean message pump):
commands::init_service_webview(&app.handle())?;

// open_service command (any thread — no COM work):
v.eval(&format!("window.location.href = {url_json};"))?;
v.show()?;
v.set_focus()?;
```

`eval()` posts the script to WebView2 asynchronously and returns immediately. It is safe
to call from any thread, including Tokio worker threads. No COM STA requirement. No
reentrancy risk. No `run_on_main_thread` needed.

`close_service` **hides** the webview (does not destroy it). The handle stays in `AppState`
so the next `open_service` hits the fast eval path. `dispatch_media_key` gates on
`active_service_id` so media keys are not dispatched to a hidden/inactive webview.

Initialization scripts (`WEBVIEW_DARK_INIT`) persist for the webview's lifetime in WebView2
and re-execute on every navigation, so dark mode and the media bridge work on each service.

Session isolation between services is preserved — WebView2 scopes cookies/storage by
origin domain, so Spotify and YouTube use separate storage even in one webview instance.

---

## Rule 1 — Call `add_child` only from `setup`

**Never call `add_child` from a `#[tauri::command]` handler or any Tokio worker thread.**

Tauri command handlers run on Tokio worker threads. `wait_with_pump` called on a background
thread pumps the *main thread's* message queue by posting messages cross-thread — this
interferes with in-flight WebView2 IPC events and can cause the COM wait to never complete.

Even on the correct Win32 main thread via `run_on_main_thread`, calling `add_child` from
inside an active WebView2 IPC event handler context creates a reentrant `wait_with_pump`
loop that can hang indefinitely.

The only safe context for `add_child` is Tauri's `setup` callback:

- Runs on the Win32 main thread.
- Runs before the event loop starts, before any WebView2 IPC events.
- Message pump is clean — no competing COM callbacks in flight.
- Consistent with how Tauri itself initialises all declared windows.

---

## Rule 2 — Never queue two concurrent `add_child` closures

This is now a historical note — `add_child` is called once in `setup` and never again.
The mechanism that caused the original deadlock is documented here for reference.

When `wait_with_pump` pumps Win32 messages, it dispatches ALL pending messages — including
winit user-events queued by `run_on_main_thread`. If two `add_child` closures were queued:

1. Closure A runs, enters `wait_with_pump` for WebView2 environment creation.
2. `wait_with_pump` processes pending messages, including the winit event for Closure B.
3. Closure B runs *inside* Closure A's `wait_with_pump` loop.
4. Closure B tries to register label `"service-view"` → label-registry conflict or
   WebView2 reentrancy deadlock.
5. Window freezes permanently.

Since `add_child` now runs only in `setup` (before the event loop, before any user
interaction), this scenario cannot occur.

The `isLoading` guard in `src/store/services.ts` is retained as a UX guard — it prevents
visible spinner flicker from rapid-clicking while a service is navigating — but it is no
longer load-bearing for deadlock prevention.

---

## Rule 3 — Never pass `.data_directory()` to `WebviewBuilder`

`.data_directory()` forces wry to call `CreateCoreWebView2EnvironmentWithOptions` with a
specific user-data path, creating a brand-new `CoreWebView2Environment`. This is a second
async operation requiring its own `wait_with_pump` chain, even when called from `setup`.

Without `.data_directory()`, wry reuses the default user-data folder shared with the main
WebviewWindow, skipping the environment-creation step entirely.

---

## WebView2 reentrancy reference

From wry's own source comment in `webview2/mod.rs`:

```rust
// Use `dispatch_handler` to schedule the run on the message loop after this callback completes,
// this is needed for `new_window_req_handler` to create new webviews for `NewWindowResponse::Create`
// or it will deadlock, see
// https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/threading-model#reentrancy
```

The same reentrancy applies when `add_child` is called during any active WebView2 callback
or event handler context.

---

## Diagnostic logging

`init_service_webview` (called once in `setup`):
```
init_service_webview: creating child webview logical_size=<w>x<h>
init_service_webview: child webview created and hidden
```

`open_service` command (called on every service selection):
```
open_service: id=<service-id> same_service=false   ← navigates + shows
open_service: id=<service-id> same_service=true    ← shows only (no reload)
```

If `init_service_webview` logs stop at **"creating child webview"** → `add_child` hung.
Likely causes:
- A `.data_directory()` was added to the `WebviewBuilder` (regression check).
- The system's WebView2 runtime is corrupt or not installed.
- Another `add_child` was called concurrently (should be impossible given current code).

If `open_service` returns `Err("service webview not initialized")` → `init_service_webview`
failed during setup and the error was swallowed. Check logs for the init failure.

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
message window which handles `run_on_main_thread` user events. This is why calling
`add_child` during active WebView2 IPC processing is unsafe.

---

## Test procedure for Windows

1. Build: `./build-all.sh` (or `npm run tauri build` on a Windows machine).
2. Run the `.exe`. Observe logs in `%LOCALAPPDATA%\com.lazylionconsulting.ingwestream\logs\Ingwe.log`.
3. On startup, confirm both init log lines appear: `init_service_webview: creating…` then `…created and hidden`.
4. Click a service once. Expected: service opens within ~3 s.
5. Rapidly click a different service while the first is loading. Expected: service switches cleanly.
6. Click the already-active service. Expected: no reload (same_service=true in log).
7. Check that the window stays responsive (can minimise/maximise/drag) throughout.

---

## Other Windows-specific notes

- Window frame: `decorations: false` in `tauri.conf.json`. React renders the custom
  titlebar. `data-tauri-drag-region` attribute on the titlebar div enables dragging.
- DPI: Tauri handles DPI scaling. Use logical pixels in `LogicalSize` / `LogicalPosition`.
- WebView2 devtools: press F12 inside the child webview in dev builds (`devtools` feature).
- Widevine DRM: not yet configured. Requires passing `--enable-features=Widevine` via
  `additional_browser_args` on the `WebviewBuilder` (platform-specific, Windows only).
