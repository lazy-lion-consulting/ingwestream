import { useServicesStore } from "@/store/services";

/**
 * Renders the DOM anchor for the active service's native WebviewWindow.
 * The Tauri WebviewWindow renders outside the React tree — this div provides
 * a visible placeholder and maintains layout when no service is selected.
 */
export function WebviewMount() {
  const activeId = useServicesStore((s) => s.activeId);

  return (
    <div className="relative flex-1 overflow-hidden bg-bg-base">
      {activeId ? (
        <div
          id={`webview-mount-${activeId}`}
          className="absolute inset-0"
        />
      ) : (
        <EmptyState />
      )}
    </div>
  );
}

function EmptyState() {
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center gap-3">
      <p className="text-text-muted text-sm tracking-widest uppercase">
        Select a service
      </p>
      <p className="text-text-disabled text-xs">
        Choose a streaming service from the sidebar to get started.
      </p>
    </div>
  );
}
