import { ipc } from "../ipc.ts";
import { getSettings, updateSettings } from "../store.ts";

/**
 * Debug-console toggle. The app runs headless by default; flipping this on
 * attaches a terminal window showing backend diagnostics, and persists the
 * choice so it reattaches on the next launch.
 */
export function initConsolePanel(): void {
  const toggle = document.getElementById("console-toggle") as HTMLInputElement;
  if (!toggle) return;

  toggle.checked = getSettings().showConsole;

  toggle.addEventListener("change", async () => {
    const enabled = toggle.checked;
    try {
      await ipc.setConsoleVisible(enabled); // live effect
      await updateSettings({ showConsole: enabled }); // persist + sync cache
    } catch (err) {
      toggle.checked = !enabled; // revert on failure
      console.error("Failed to toggle debug console:", err);
    }
  });
}
