import { ipc } from "../ipc.ts";
import { getSettings, updateSettings } from "../store.ts";

export async function initStartupPanel(): Promise<void> {
  const toggle = document.getElementById("autostart-toggle") as HTMLInputElement | null;

  if (toggle) {
    toggle.checked = await ipc.getAutostartEnabled();

    toggle.addEventListener("change", async () => {
      try {
        await ipc.setAutostart(toggle.checked);
      } catch (err) {
        toggle.checked = !toggle.checked;
        console.error("Failed to set autostart:", err);
      }
    });
  }

  const backgroundToggle = document.getElementById(
    "background-toggle",
  ) as HTMLInputElement | null;
  if (!backgroundToggle) return;

  backgroundToggle.checked = getSettings().runInBackground;

  backgroundToggle.addEventListener("change", async () => {
    const enabled = backgroundToggle.checked;
    try {
      await updateSettings({ runInBackground: enabled });
    } catch (err) {
      backgroundToggle.checked = !enabled;
      console.error("Failed to set background behavior:", err);
    }
  });
}
