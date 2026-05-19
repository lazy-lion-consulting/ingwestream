# Ingwe — Frontend Deep Context

## Tech

- React 19, TypeScript ~5.8, Vite 7
- Tailwind CSS v4 (Vite plugin — no `tailwind.config.js`, config is in `index.css` `@theme`)
- shadcn/ui (`components.json` — `style: default`, `baseColor: slate`, `cssVariables: true`)
- Zustand 5 (no immer, no persist)
- lucide-react 1.x for all icons
- `clsx` + `tailwind-merge` via `cn()` in `@/lib/utils`

Path alias: `@/` → `src/`

---

## Tailwind v4 — critical differences from v3

- No `tailwind.config.js`. All customisation lives in `index.css` inside `@theme {}`.
- Import is `@import "tailwindcss"` not `@tailwind base/components/utilities`.
- Custom tokens become utilities automatically: `bg-bg-base`, `text-text-primary`, etc.
- Arbitrary values still work: `w-[52px]`, `bg-[#111]`.
- `@layer base {}` for CSS resets and shadcn/ui variable mappings.
- Vite plugin handles JIT — no separate PostCSS step.

---

## Complete design token reference

All defined in `src/index.css` `@theme {}`:

### Backgrounds

```css
--color-bg-base: #000000 /* page background, OLED black */
  --color-bg-surface: #0a0a0a /* cards, panels, titlebar, sidebar */
  --color-bg-elevated: #111111 /* hover state, dropdowns */
  --color-bg-overlay: #1a1a1a /* active selected item */
  --color-bg-subtle: #222222 /* subtle separators, muted areas */;
```

Tailwind classes: `bg-bg-base`, `bg-bg-surface`, `bg-bg-elevated`, `bg-bg-overlay`, `bg-bg-subtle`

### Borders

```css
--color-border-base: #2a2a2a --color-border-strong: #3a3a3a;
```

Tailwind: `border-border-base`, `border-border-strong`

### Text

```css
--color-text-primary: #f0f0f0 --color-text-secondary: #a0a0a0
  --color-text-muted: #606060 --color-text-disabled: #404040;
```

Tailwind: `text-text-primary`, `text-text-secondary`, `text-text-muted`, `text-text-disabled`

### Accent (blue)

```css
--color-accent: #4f86f7 --color-accent-hover: #6a9bf9
  --color-accent-dim: #1a2f5a;
```

Tailwind: `text-accent`, `bg-accent`, `text-accent-hover`, `bg-accent-dim`

### Danger (red)

```css
--color-danger: #e05252 --color-danger-dim: #3b1818;
```

Tailwind: `text-danger`, `bg-danger`, `bg-danger-dim`

### Shape / Shadow

```css
--radius-sm: 4px /* rounded-sm */ --radius-md: 8px /* rounded-md */
  --radius-lg: 12px /* rounded-lg */ --shadow-float: 0 4px 24px
  rgba(0, 0, 0, 0.8);
```

### Animation

```css
--animate-loading-bar: ingwe-loading-bar 1.3s ease-in-out infinite;
```

Tailwind: `animate-loading-bar`
Keyframes: `0% translateX(-100%)` → `100% translateX(210%)` (sliding indeterminate bar)

---

## shadcn/ui CSS variable bridge

`index.css` `@layer base` maps Tailwind tokens → shadcn HSL variables so shadcn
components inherit the dark theme without a separate `globals.css`:

```css
:root {
  --background: 0 0% 0%; /* bg-base */
  --foreground: 0 0% 94%; /* text-primary */
  --card: 0 0% 4%; /* bg-surface */
  --primary: 220 90% 64%; /* accent */
  /* …etc */
}
```

Add shadcn components: `npx shadcn@latest add <name>`
They land in `src/components/ui/` and use `cn()` internally.

---

## Component inventory

### `App.tsx`

Pure layout shell. No state, no effects.

```tsx
<div className="flex flex-col h-screen bg-bg-base text-text-primary overflow-hidden">
  <TitleBar /> {/* h-8, shrink-0 */}
  <div className="relative flex-1 overflow-hidden">
    <WebviewMount /> {/* fills space, z-0 */}
    <Sidebar /> {/* absolute, z-20/30 */}
  </div>
</div>
```

### `TitleBar.tsx`

- `data-tauri-drag-region` on outer div and title span
- Left: `<LayoutGrid>` (toggleFlyout) + service label or "Ingwe"
- Right: minimize / maximize / close (standard 32px wide buttons)
- Close button uses `hover:bg-danger` not `hover:bg-bg-elevated`
- Loading bar: absolute bottom, 2px, `animate-loading-bar`, `pointer-events-none`
- Subscribes: `toggleFlyout`, `activeId`, `isLoading` from store

### `Sidebar.tsx`

- Backdrop: `absolute inset-0 z-20 bg-black/50` with opacity transition
- Panel: `absolute left-0 top-0 bottom-0 w-52 bg-bg-surface border-r border-border-base z-30`
- Slides in/out via `translate-x-0` / `-translate-x-full` with `transition-transform duration-200`
- `ServiceItem` button: `w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-sm`
- Active item: `bg-bg-overlay text-text-primary`, icon gets `text-accent`
- Footer: `text-[10px] tracking-widest uppercase text-text-disabled` "Ingwe" branding

### `WebviewMount.tsx`

Read-only — only reads `activeId`. No effects, no invoke calls. Renders either:

- A `div#webview-mount-{activeId}` placeholder (native webview renders above it)
- `<EmptyState />` with "Select a service" + hint text

### Icon usage pattern

```tsx
// In Sidebar — icon map for tree-shaking
const ICON_MAP: Record<string, React.FC<LucideProps>> = { Disc, Music, Play, ... };
function ServiceIcon({ name, ...props }) {
  const Icon = ICON_MAP[name] ?? Music;
  return <Icon {...props} />;
}
// Usage: <ServiceIcon name={service.icon} className="size-4 shrink-0" />
```

For one-off icons import directly:

```tsx
import { Settings, ChevronRight } from "lucide-react";
<Settings className="size-4 text-text-muted" />;
```

---

## Zustand store (`src/store/services.ts`)

```ts
// Read slice selectors — always prefer granular to avoid re-renders
const activeId = useServicesStore((s) => s.activeId);
const isLoading = useServicesStore((s) => s.isLoading);
const openService = useServicesStore((s) => s.openService);
const toggleFlyout = useServicesStore((s) => s.toggleFlyout);
```

**`openService` guard** — always check this pattern is intact when modifying:

```ts
openService: async (service) => {
  if (get().isLoading) return;          // ← CRITICAL — prevents Windows WebView2 deadlock
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

Do NOT remove or reorder the guard. Do NOT add a `useEffect` anywhere that calls
`openService` without a stable dep array gating it to a single call.

---

## New component checklist

1. File: `src/components/MyComponent.tsx`
2. Imports: `cn` from `@/lib/utils`, icons from `lucide-react`, store selectors as needed
3. Styling: use design tokens only — no hardcoded hex/rgb values
4. No `document` / `window` access without `typeof window !== "undefined"` guard
5. For interactive elements: always include `aria-label` on icon-only buttons
6. Prefer `transition-colors duration-150` for hover states; `duration-200` for panel slides

---

## Adding a hook

Place in `src/hooks/useMyHook.ts`. Keep hooks pure — no direct Tauri API calls inside
hooks (put those in the store actions instead).

---

## TypeScript conventions

- Strict mode on (`tsconfig.json` extends strict)
- Prefer `interface` for object shapes, `type` for unions
- `@/` alias everywhere — no relative `../` imports crossing component boundaries
- Export components as named exports, not default (except `App.tsx`)

---

## Performance notes

- Service list is static (`SERVICES` constant) — no need for `useMemo`
- `useServicesStore` selectors are already granular — avoid `useServicesStore(s => s)`
- The native webview runs outside React; there is no VDOM overhead for streaming content
- `React.StrictMode` is ON in dev (double-invokes effects) but OFF in production builds
