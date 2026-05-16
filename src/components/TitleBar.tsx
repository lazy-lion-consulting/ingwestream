import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";

export function TitleBar() {
  const win = getCurrentWindow();

  return (
    <div
      data-tauri-drag-region
      className="h-8 flex items-center justify-between bg-bg-surface border-b border-border-base select-none shrink-0"
    >
      {/* App name */}
      <span
        data-tauri-drag-region
        className="pl-3 text-xs font-semibold tracking-widest uppercase text-text-muted"
      >
        Ingwe
      </span>

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
    </div>
  );
}
