import { getCurrentWindow } from "@tauri-apps/api/window";

export function initWindowControls(): void {
  const appWindow = getCurrentWindow();
  const minimizeBtn = document.getElementById("win-minimize")!;
  const maximizeBtn = document.getElementById("win-maximize")!;
  const closeBtn = document.getElementById("win-close")!;

  async function syncMaximizeIcon(): Promise<void> {
    const isMaximized = await appWindow.isMaximized();
    maximizeBtn.classList.toggle("is-maximized", isMaximized);
    const label = isMaximized ? "Restore" : "Maximize";
    maximizeBtn.title = label;
    maximizeBtn.setAttribute("aria-label", label);
  }

  minimizeBtn.addEventListener("click", () => {
    void appWindow.minimize();
  });

  maximizeBtn.addEventListener("click", async () => {
    await appWindow.toggleMaximize();
    await syncMaximizeIcon();
  });

  // Close behavior is controlled by the persisted run-in-background setting.
  closeBtn.addEventListener("click", () => {
    void appWindow.close();
  });

  // Keep the icon correct when the window is maximized/restored by other
  // means (double-click drag region, Win+Up, snap, etc.)
  void appWindow.onResized(() => {
    void syncMaximizeIcon();
  });

  void syncMaximizeIcon();
}
