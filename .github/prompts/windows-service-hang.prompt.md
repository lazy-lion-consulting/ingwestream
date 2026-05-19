# Issue: Windows hang when clicking a streaming service

**Status:** Fix applied — awaiting Windows test confirmation.  
**Severity:** Critical (complete window freeze, requires process kill).  
**Platform:** Windows only (not reproducible on Linux/macOS).

---

## Symptom

After launching Ingwe on Windows and clicking any service in the sidebar, the main window
becomes completely unresponsive:

- Window cannot be moved, resized, minimised, or closed.
- The service never loads (no navigation occurs).
- Task Manager shows the process is alive but not responding.
- The window must be killed via Task Manager.

The hang occurs on first click and on subsequent clicks after app restart.

---

## Log evidence — double invocation

The logs show `open_service` being called twice simultaneously with identical parameters and
nearly identical timestamps (within the same second):

```
[2026-05-19][15:28:25][ingwe] INFO open_service: id=youtube logical_size=1200x768
[2026-05-19][15:28:25][ingwe] INFO open_service: id=youtube logical_size=1200x768
```

Both calls reached the point of dispatching `run_on_main_thread`. From this point, the
application hung indefinitely.

---

## Root cause analysis

### Layer 1 — Double IPC invocation

The frontend called `invoke("open_service", …)` twice concurrently. This was caused by:

- **React.StrictMode in dev**: React 19's StrictMode double-invokes effects and event
  handlers in development to surface side-effects. With no guard, a single sidebar click
  triggered two `openService` calls.
- **Rapid clicking**: A user clicking multiple services quickly could also produce this.

### Layer 2 — wry `wait_with_pump` reentrancy

wry's WebView2 creation path on Windows (`InnerWebView::new_in_hwnd`) is:

```
CoInitializeEx(COINIT_APARTMENTTHREADED)
  → create_environment()     → wait_with_pump(env_rx)
  → create_controller(hwnd)  → wait_with_pump(ctrl_rx)
  → init_webview()           [sync]
```

`wait_with_pump` runs a nested Win32 message loop:

```rust
loop {
    // try channel...
    MsgWaitForMultipleObjectsEx(...)  // wait for message or COM callback
    while PeekMessageW(..., PM_REMOVE) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);  // ← dispatches ALL window messages, including winit user-events
    }
}
```

Because `DispatchMessageW` dispatches **all** messages — including the winit user-event
that executes a queued `run_on_main_thread` closure — the sequence becomes:

```
Thread A (Tokio): invoke("open_service", ...)
Thread B (Tokio): invoke("open_service", ...)   ← second concurrent call

Main thread: Closure A runs → enters wait_with_pump for environment creation
  → DispatchMessageW fires → Closure B runs
  → Closure B also tries to create label "service-view"
  → Tauri label-registry already has "service-view" in-flight from Closure A
  → DEADLOCK: Closure B's wait_with_pump also pumps messages, both wait indefinitely
```

wry's own source references this pattern:

> _"Use `dispatch_handler` to schedule the run on the message loop after this callback
> completes, or it will deadlock"_  
> — `webview2_com` threading model note

### Layer 3 — Absence of `.data_directory()` (pre-fix)

Before the fix, `WebviewBuilder` was constructed with `.data_directory(app_data_dir)`.
This forced wry to call `CreateCoreWebView2EnvironmentWithOptions` instead of reusing the
default environment, adding a _third_ `wait_with_pump` chain. This extended the window
during which the second closure could be dispatched and compounded the reentrancy.

---

## Fix applied

### 1. Frontend — `isLoading` guard (`src/store/services.ts`)

```ts
openService: async (service) => {
  if (get().isLoading) return;   // ← primary deadlock prevention
  set({ activeId: service.id, flyoutOpen: false, isLoading: true });
  try {
    await invoke("open_service", { serviceId: service.id, url: service.url });
  } catch (e) {
    console.error("[ingwe] open_service failed:", e);
  } finally {
    set({ isLoading: false });
  }
},
```

This prevents any second `invoke("open_service")` call from being dispatched while one is
already in-flight. The loading state is also reflected visually as a 2px progress bar on
the TitleBar.

**This guard must not be removed or weakened.**

### 2. Backend — Removed `.data_directory()` (`src-tauri/src/commands.rs`)

The `WebviewBuilder` for the child webview no longer sets `.data_directory(…)`. Both the
main WebviewWindow and the child webview now share the default user-data folder, which
means wry reuses the existing `CoreWebView2Environment`. This eliminates one `wait_with_pump`
chain from the critical path.

### 3. Backend — Diagnostic logging (`src-tauri/src/commands.rs`)

Granular `log::info!` calls were added at each step of `open_service` to make future
diagnosis unambiguous:

```
open_service: closing previous child webview
open_service: id=<id> logical_size=<w>x<h>
open_service: dispatching add_child to main thread
open_service: closure running on main thread — calling add_child
open_service: add_child returned ok=<bool>
open_service: closure dispatched — waiting on channel
open_service: channel resolved — webview handle received
open_service: child webview created for '<id>'
```

The last visible log line before a hang will identify the failure point exactly.

---

## Current status

- Fix compiled clean: `cargo check` reports 0 errors, 2 dead_code warnings (unused
  `SUSPEND_SCRIPT` / `RESUME_SCRIPT` constants — expected).
- **Not yet tested on a Windows machine.** Awaiting user confirmation.

---

## How to verify the fix

1. Build on Windows: `npm run tauri build` (requires Rust + WebView2 runtime installed).
2. Run the built `.exe`.
3. Click a service (e.g. Spotify). Expected: service loads within ~3 s, no hang.
4. While loading (progress bar visible), click the same or a different service. Expected:
   click is silently ignored (isLoading guard fires), first load completes normally.
5. After first load, click a different service. Expected: switches correctly.
6. Check `%APPDATA%\ingwe\logs\ingwe.log` for the full log sequence.

---

## Alternative approaches (if fix is insufficient)

If the `isLoading` guard and no-`.data_directory()` fix are not sufficient, consider:

### Option A — Pre-create webview at startup

Create the `"service-view"` child webview once during `app.setup()` with a blank initial
URL (`about:blank`). Switching services would call `webview.navigate(url)` instead of
creating a new child. This removes `add_child` from the hot path entirely.

Tradeoff: The child webview window would always exist (even when no service is active),
consuming a small amount of memory.

### Option B — Backend serialisation with a Tokio mutex

Use a `tokio::sync::Mutex` (not `std::sync::Mutex`) as a dedicated serialisation gate
in the backend, so that `open_service` handlers are truly sequential even if the frontend
guard is bypassed. The handler would attempt to acquire the lock and return an error if
it is already held.

### Option C — Use `WebviewWindow` instead of child `Webview`

Revert to creating a separate `WebviewWindow` (which does not use `add_child`). This
avoids the `unstable` feature requirement and may be more stable, but the window would
appear as a separate OS window rather than being embedded in the main window.

---

## Related files

- `src-tauri/src/commands.rs` — `open_service` implementation, fix location.
- `src/store/services.ts` — `isLoading` guard, `openService` action.
- `.claude/windows-webview2.md` — detailed threading model reference.
- `CLAUDE.md` — "Windows-specific" section with summary notes.
