import { getCurrentWindow } from "@tauri-apps/api/window";
import { LayoutGrid, Minus, Square, X } from "lucide-react";
import { useServicesStore, SERVICES } from "@/store/services";

export function TitleBar() {
  const win = getCurrentWindow();
  const toggleFlyout = useServicesStore((s) => s.toggleFlyout);
  const activeId = useServicesStore((s) => s.activeId);
  const isLoading = useServicesStore((s) => s.isLoading);

  const activeLabel = activeId
    ? (SERVICES.find((s) => s.id === activeId)?.label ?? null)
    : null;

  return (
    <div
      data-tauri-drag-region
      className="relative h-8 flex items-center justify-between bg-bg-surface border-b border-border-base select-none shrink-0"
    >
      {/* Left: menu toggle + app/service name */}
      <div className="flex items-center h-full">
        <button
          onClick={toggleFlyout}
          className="h-full px-3 flex items-center text-text-muted hover:text-text-primary hover:bg-bg-elevated transition-colors duration-150"
          aria-label="Services"
        >
          <LayoutGrid className="size-3.5" />
        </button>
        <span
          data-tauri-drag-region
          className="pl-1 text-xs font-semibold tracking-widest uppercase text-text-muted"
        >
          {activeLabel ?? "Ingwe"}
        </span>
      </div>

      {/* Window controls */}
      <div className="flex items-center h-full">
        <button
          onClick={() => win.minimize()}
          className="h-full px-4 flex items-center text-text-muted hover:text-text-primary hover:bg-bg-elevated transition-colors duration-150"
          aria-label="Minimise"
        >
          <Minus className="size-3.5" />
        </button>
        <button
          onClick={() => win.toggleMaximize()}
          className="h-full px-4 flex items-center text-text-muted hover:text-text-primary hover:bg-bg-elevated transition-colors duration-150"
          aria-label="Maximise"
        >
          <Square className="size-3.5" />
        </button>
        <button
          onClick={() => win.close()}
          className="h-full px-4 flex items-center text-text-muted hover:text-text-primary hover:bg-danger transition-colors duration-150"
          aria-label="Close"
        >
          <X className="size-3.5" />
        </button>
      </div>

      {/* Loading bar */}
      {isLoading && (
        <div className="absolute bottom-0 left-0 right-0 h-[2px] overflow-hidden pointer-events-none">
          <div className="absolute h-full w-1/2 bg-accent animate-loading-bar" />
        </div>
      )}
    </div>
  );
}
