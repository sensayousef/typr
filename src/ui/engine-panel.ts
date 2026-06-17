import { ipc } from "../ipc.ts";
import { getSettings, updateSettings } from "../store.ts";

export async function initEnginePanel(): Promise<void> {
  const engineLocal = document.getElementById("engine-local")!;
  const engineCloud = document.getElementById("engine-cloud")!;
  const localSettings = document.getElementById("local-settings")!;
  const cloudSettings = document.getElementById("cloud-settings")!;
  const modelSelect = document.getElementById("model-select") as HTMLSelectElement;
  const downloadBtn = document.getElementById("download-btn") as HTMLButtonElement;
  const downloadProgress = document.getElementById("download-progress")!;
  const progressFill = document.getElementById("progress-fill")!;
  const groqKey = document.getElementById("groq-key") as HTMLInputElement;

  const settings = getSettings();
  applyEngine(settings.engine);
  modelSelect.value = settings.whisperModel;
  groqKey.value = settings.groqApiKey;
  await checkModelStatus();

  function applyEngine(engine: string): void {
    engineLocal.classList.toggle("active", engine === "local");
    engineCloud.classList.toggle("active", engine === "cloud");
    localSettings.classList.toggle("hidden", engine !== "local");
    cloudSettings.classList.toggle("hidden", engine !== "cloud");
  }

  async function checkModelStatus(): Promise<void> {
    const downloaded = await ipc.checkModelDownloaded(modelSelect.value);
    downloadBtn.textContent = downloaded ? "✓" : "Download";
    downloadBtn.disabled = downloaded;
  }

  engineLocal.addEventListener("click", () => {
    applyEngine("local");
    void updateSettings({ engine: "local" });
  });

  engineCloud.addEventListener("click", () => {
    applyEngine("cloud");
    void updateSettings({ engine: "cloud" });
  });

  modelSelect.addEventListener("change", () => {
    void checkModelStatus().then(() => updateSettings({ whisperModel: modelSelect.value }));
  });

  downloadBtn.addEventListener("click", async () => {
    downloadBtn.disabled = true;
    downloadProgress.classList.remove("hidden");
    progressFill.style.width = "0%";
    try {
      await ipc.downloadModel(modelSelect.value);
      downloadBtn.textContent = "✓";
    } catch (e) {
      downloadBtn.textContent = "Retry";
      downloadBtn.disabled = false;
      console.error("Download failed:", e);
    }
    downloadProgress.classList.add("hidden");
  });

  groqKey.addEventListener("change", () => {
    void updateSettings({ groqApiKey: groqKey.value });
  });

  await ipc.onDownloadProgress(({ percent }) => {
    progressFill.style.width = `${percent}%`;
  });
}
