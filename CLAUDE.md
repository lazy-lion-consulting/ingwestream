# Ingwe — Claude CLI Master Context

Ingwe is a cross-platform desktop streaming consolidator. It wraps native OS webviews
(WebView2 / WKWebView / WebKitGTK) in a Tauri v2 shell with a custom OLED-dark
React UI. One child webview renders the active streaming service; the shell handles
tray, media keys, window chrome, and service switching.

---

## Stack (exact versions)

| Layer         | Tech            | Version                                  |
| ------------- | --------------- | ---------------------------------------- |
| Shell         | Tauri           | 2.x (`unstable` feature for `add_child`) |
| Backend       | Rust / Cargo    | edition 2021                             |
| Frontend      | React           | 19                                       |
| Language      | TypeScript      | ~5.8                                     |
| Styling       | Tailwind CSS v4 | 4.x (Vite plugin, no config file)        |
| UI primitives | shadcn/ui       | via `components.json`                    |
| Icons         | lucide-react    | 1.x                                      |
| State         | Zustand         | 5.x                                      |
| Bundler       | Vite            | 7.x                                      |

---

## Project layout

```
ingwestream/
├── CLAUDE.md                    ← you are here
├── .claude/
│   ├── frontend.md              ← UI system, component patterns, Tailwind tokens
│   ├── backend.md               ← Rust commands, state, IPC, tray, shortcuts
│   └── windows-webview2.md      ← WebView2/wry threading model, known issues
├── src/                         ← React frontend
│   ├── main.tsx                 ← ReactDOM.createRoot (React.StrictMode ON)
│   ├── App.tsx                  ← Root layout: TitleBar + Sidebar + WebviewMount
│   ├── index.css                ← Tailwind v4 @theme, custom tokens, animations
│   ├── components/
│   │   ├── TitleBar.tsx         ← Drag region, service label, window controls, loading bar
│   │   ├── Sidebar.tsx          ← Fly-out panel, service list, backdrop
│   │   └── WebviewMount.tsx     ← Placeholder div; native webview renders above it
│   ├── store/
│   │   └── services.ts          ← Zustand store: activeId, isLoading, openService, closeService
│   ├── services/
│   │   └── serviceRegistry.ts   ← SERVICES array: id, label, url, icon
│   ├── lib/utils.ts             ← cn() = clsx + twMerge
│   └── assets/
├── src-tauri/
│   ├── tauri.conf.json          ← Window config, CSP, bundle
│   ├── Cargo.toml               ← Dependencies
│   ├── capabilities/default.json ← IPC permissions
│   └── src/
│       ├── lib.rs               ← Builder setup: plugins, manage, invoke_handler, setup, on_window_event
│       ├── main.rs              ← Binary entry (calls lib::run)
│       ├── commands.rs          ← IPC handlers: open_service, close_service, show/hide_service_view
│       ├── state.rs             ← AppState { service_view, active_service_id }
│       ├── scripts.rs           ← WEBVIEW_DARK_INIT, SUSPEND_SCRIPT, RESUME_SCRIPT
│       ├── tray.rs              ← System tray: show/prev/play/next/quit
│       └── shortcuts.rs         ← Global media key registration
└── .github/
    └── copilot-instructions.md  ← VS Code Copilot context (separate from this file)
```

---

## Build & dev commands

```bash
# Frontend hot-reload + Tauri dev build
npm run tauri dev

# Production build (all targets)
npm run tauri build
# or
./build-all.sh

# TypeScript check only
npm run build

# Rust check only (fast, no link)
cd src-tauri && cargo check

# Rust lint
cd src-tauri && cargo clippy

# Run tests
cd src-tauri && cargo test
```

Dev server: `http://localhost:1420` HMR websocket: `ws://localhost:1421`

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  OS Window  (decorations=false, 1200×800 default)        │
│  ┌───────────────────────────────────────────────────┐   │
│  │ TitleBar  h=32px  [drag-region]  [window chrome]  │   │
│  ├─────────────┬─────────────────────────────────────┤   │
│  │ Sidebar     │  WebviewMount (placeholder)          │   │
│  │ (fly-out,   │                                      │   │
│  │  z-30)      │  ┌──────────────────────────────┐   │   │
│  │             │  │ Child Webview "service-view"  │   │   │
│  │             │  │ (native WebView2/WKWebView)   │   │   │
│  │             │  │ pos: (0, 32)  size: fill      │   │   │
│  └─────────────┴──┴──────────────────────────────┴───┘   │
└─────────────────────────────────────────────────────────┘
```

The child webview is a Tauri `Webview` added via `Window::add_child()` (requires
`unstable` feature). It renders natively above the React tree — `WebviewMount` only
provides layout context.

---

## IPC surface (frontend → backend)

| Command             | Args                             | Returns | Notes                          |
| ------------------- | -------------------------------- | ------- | ------------------------------ |
| `open_service`      | `serviceId: string, url: string` | `void`  | Creates/replaces child webview |
| `close_service`     | —                                | `void`  | Destroys child webview         |
| `show_service_view` | —                                | `void`  | Shows child (flyout closed)    |
| `hide_service_view` | —                                | `void`  | Hides child (flyout open)      |

All commands return `Result<(), AppError>` serialised as `{ message: string }` on error.

---

## UI design system ▸ full detail in `.claude/frontend.md`

**Palette** (OLED — all from `index.css` `@theme`):

```
bg:      base=#000  surface=#0a0a0a  elevated=#111  overlay=#1a1a1a  subtle=#222
border:  base=#2a2a2a  strong=#3a3a3a
text:    primary=#f0f0f0  secondary=#a0a0a0  muted=#606060  disabled=#404040
accent:  #4f86f7  hover=#6a9bf9  dim=#1a2f5a
danger:  #e05252  dim=#3b1818
radius:  sm=4px  md=8px  lg=12px
shadow:  float = 0 4px 24px rgba(0,0,0,0.8)
```

**Typography conventions:**

- Labels / meta: `text-xs tracking-widest uppercase text-text-muted`
- Body: `text-sm text-text-secondary`
- Active / emphasis: `text-text-primary`
- Never use light backgrounds. No whites, no grays above `#222` for surfaces.

**Interaction conventions:**

- Hover state: `hover:bg-bg-elevated hover:text-text-primary transition-colors duration-150`
- Active/selected: `bg-bg-overlay text-text-primary`
- Destructive hover: `hover:bg-danger`
- Loading: 2px bottom bar with `animate-loading-bar` (defined in `index.css`)
- Focus rings: use `focus-visible:` only; suppress default outlines

**Component imports:**

```tsx
import { cn } from "@/lib/utils"; // className merge
import { SomeIcon } from "lucide-react"; // icons, always size-4 or size-3.5
```

**shadcn/ui:** Available via `components.json`. Add components with:

```bash
npx shadcn@latest add <component>
```

---

## State management

```ts
// src/store/services.ts — Zustand v5 (no persist middleware)
interface ServicesState {
  activeId: string | null;
  flyoutOpen: boolean;
  isLoading: boolean; // true while open_service is in-flight

  openService(service: ServiceDefinition): Promise<void>;
  closeService(): Promise<void>;
  toggleFlyout(): void;
  closeFlyout(): void;
}
```

**Critical guard:** `openService` bails immediately if `isLoading` is true — prevents
concurrent `add_child` calls that cause a WebView2 reentrancy deadlock on Windows.

---

## Adding a new streaming service

1. Add entry to `SERVICES` in `src/services/serviceRegistry.ts`
2. Import its lucide icon in `src/components/Sidebar.tsx` → `ICON_MAP`
3. No Rust changes needed.

---

## Rust conventions

- Error type: `AppError` in `commands.rs` (derives `thiserror::Error + serde::Serialize`)
- State access: always take lock, clone/take what you need, drop lock before IO
- Logging: `log::info!()` / `log::warn!()` / `log::error!()` (never `println!`)
- New commands: add `pub fn`, `#[tauri::command]`, register in `lib.rs` invoke_handler,
  add permission to `capabilities/default.json`

---

## Windows-specific ▸ full detail in `.claude/windows-webview2.md`

- `add_child` must run on the Win32 main thread via `app.run_on_main_thread()`
- **Never** call `add_child` from a Tokio background thread (COM STA / wry `wait_with_pump` incompatibility)
- **Never** queue two `run_on_main_thread` closures that both call `add_child` — the nested
  `wait_with_pump` message pump will dispatch the second closure inside the first's COM wait,
  causing a label-registry deadlock
- Do **not** pass `.data_directory()` to `WebviewBuilder` — it forces a new `CoreWebView2Environment`
  which adds another `wait_with_pump` chain to an already-complex call path
- The `isLoading` guard in the frontend store is the primary protection against double-dispatch

---

## CSP (tauri.conf.json)

```
default-src 'self' tauri: asset:;
script-src 'self' 'unsafe-inline';
style-src 'self' 'unsafe-inline';
img-src 'self' data: asset: tauri: blob:;
connect-src 'self' ipc: http://ipc.localhost
```

The child webview navigates to external URLs and is NOT subject to this CSP.

---

## Capabilities (capabilities/default.json)

Window `main` has: `core:default`, window show/hide/minimize/maximize/close/focus/drag,
opener default, window-state default, global-shortcut register/unregister/is-registered.

When adding new plugin permissions, append to the `permissions` array here.

---

## Do not touch

- `src-tauri/target/` — build artifacts, never read or write
- `dist/` — Vite output, generated
- `src-tauri/gen/schemas/` — auto-generated Tauri JSON schemas
- `node_modules/` — dependencies
