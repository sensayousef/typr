import { ipc } from "../ipc.ts";

export async function initStartupPanel(): Promise<void> {
  const toggle = document.getElementById("autostart-toggle") as HTMLInputElement;
  if (!toggle) return;

  toggle.checked = await ipc.getAutostartEnabled();

  toggle.addEventListener("change", async () => {
    try {
      await ipc.setAutostart(toggle.checked);
    } catch (err) {
      // Revert on failure
      toggle.checked = !toggle.checked;
      console.error("Failed to set autostart:", err);
    }
  });
}
