import { getSettings, updateSettings } from "../store.ts";

export function initModePanel(): void {
  const modeToggle = document.getElementById("mode-toggle")!;
  const modePtt = document.getElementById("mode-ptt")!;

  function applyMode(mode: string): void {
    modeToggle.classList.toggle("active", mode === "toggle");
    modePtt.classList.toggle("active", mode === "push-to-talk");
  }

  applyMode(getSettings().recordingMode);

  modeToggle.addEventListener("click", () => {
    applyMode("toggle");
    void updateSettings({ recordingMode: "toggle" });
  });

  modePtt.addEventListener("click", () => {
    applyMode("push-to-talk");
    void updateSettings({ recordingMode: "push-to-talk" });
  });
}
