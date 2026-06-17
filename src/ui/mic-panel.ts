import { ipc } from "../ipc.ts";
import { getSettings, updateSettings } from "../store.ts";

export async function initMicPanel(): Promise<void> {
  const micSelect = document.getElementById("mic-select") as HTMLSelectElement;
  const mics = await ipc.listMicrophones();
  const settings = getSettings();

  micSelect.innerHTML = "";
  mics.forEach((mic) => {
    const option = document.createElement("option");
    option.value = mic.name;
    option.textContent = mic.name + (mic.is_default ? " (default)" : "");
    micSelect.appendChild(option);
  });
  micSelect.value = settings.microphone;

  micSelect.addEventListener("change", () => {
    void updateSettings({ microphone: micSelect.value });
  });
}
