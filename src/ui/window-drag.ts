import { getCurrentWindow } from "@tauri-apps/api/window";

export function initWindowDrag(): void {
  const titlebar = document.getElementById("titlebar")!;
  const sidebar = document.getElementById("sidebar")!;
  const appWindow = getCurrentWindow();

  function isInteractive(target: EventTarget | null): boolean {
    return !!(target as HTMLElement).closest(
      "button, select, input, a, .nav-item"
    );
  }

  titlebar.addEventListener("mousedown", (e) => {
    if (!isInteractive(e.target)) appWindow.startDragging();
  });

  sidebar.addEventListener("mousedown", (e) => {
    if (!isInteractive(e.target)) appWindow.startDragging();
  });
}
