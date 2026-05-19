import { getCurrentWindow } from "@tauri-apps/api/window";
import type { MouseEvent } from "react";

type ResizeDir =
  | "East" | "North" | "NorthEast" | "NorthWest"
  | "South" | "SouthEast" | "SouthWest" | "West";

const S = 4;   // edge strip width/height (px)
const C = 12;  // corner square size (px)

function Handle({ dir, style }: { dir: ResizeDir; style: React.CSSProperties }) {
  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    getCurrentWindow().startResizeDragging(dir).catch(() => {});
  }
  return <div style={{ position: "fixed", zIndex: 99999, ...style }} onMouseDown={onMouseDown} />;
}

export function ResizeBorder() {
  return (
    <>
      {/* Top edge */}
      <Handle dir="North"     style={{ top: 0, left: C, right: C, height: S, cursor: "n-resize" }} />
      {/* Top corners */}
      <Handle dir="NorthWest" style={{ top: 0, left: 0, width: C, height: C, cursor: "nw-resize" }} />
      <Handle dir="NorthEast" style={{ top: 0, right: 0, width: C, height: C, cursor: "ne-resize" }} />
      {/* Bottom edge — only reachable when service view is hidden */}
      <Handle dir="South"     style={{ bottom: 0, left: C, right: C, height: S, cursor: "s-resize" }} />
      {/* Bottom corners */}
      <Handle dir="SouthWest" style={{ bottom: 0, left: 0, width: C, height: C, cursor: "sw-resize" }} />
      <Handle dir="SouthEast" style={{ bottom: 0, right: 0, width: C, height: C, cursor: "se-resize" }} />
      {/* Side edges — only reachable when service view is hidden */}
      <Handle dir="West"      style={{ top: C, bottom: C, left: 0, width: S, cursor: "w-resize" }} />
      <Handle dir="East"      style={{ top: C, bottom: C, right: 0, width: S, cursor: "e-resize" }} />
    </>
  );
}
