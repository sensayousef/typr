import { ipc } from "../ipc.ts";
import { getSettings, updateSettings } from "../store.ts";
import { initTtsHotkeyCapture } from "./hotkey-capture.ts";
import type { Settings, SpeakingState } from "../types.ts";

type VoiceKey = "ttsVoiceLocal" | "ttsVoiceCloud";

// Local (OS) and cloud (Orpheus) voices use disjoint id namespaces, so each
// engine remembers its own selection rather than sharing one clobberable field.
function voiceKeyForEngine(engine: string): VoiceKey {
  return engine === "cloud" ? "ttsVoiceCloud" : "ttsVoiceLocal";
}

function currentVoiceForEngine(settings: Settings): string {
  return settings[voiceKeyForEngine(settings.ttsEngine)];
}

export async function initTtsPanel(): Promise<void> {
  const ttsEnable = document.getElementById("tts-enable") as HTMLInputElement;
  const ttsPanel = document.getElementById("tts-panel-body");
  const ttsLocal = document.getElementById("tts-engine-local") as HTMLButtonElement;
  const ttsCloud = document.getElementById("tts-engine-cloud") as HTMLButtonElement;
  const voiceSelect = document.getElementById("tts-voice-select") as HTMLSelectElement;
  const rateSlider = document.getElementById("tts-rate") as HTMLInputElement;
  const rateLabel = document.getElementById("tts-rate-label");
  const testBtn = document.getElementById("tts-test-btn") as HTMLButtonElement;
  const playbackRow = document.getElementById("tts-playback-row");
  const playbackHint = document.getElementById("tts-playback-hint");
  const pauseBtn = document.getElementById("tts-pause-btn") as HTMLButtonElement;
  const resumeBtn = document.getElementById("tts-resume-btn") as HTMLButtonElement;
  const stopBtn = document.getElementById("tts-stop-btn") as HTMLButtonElement;

  if (!ttsEnable || !ttsPanel) return;

  const settings = getSettings();

  ttsEnable.checked = settings.ttsEnabled;
  ttsPanel.classList.toggle("hidden", !settings.ttsEnabled);
  rateSlider.value = String(settings.ttsRate);
  if (rateLabel) rateLabel.textContent = `${settings.ttsRate} WPM`;

  applyEngine(settings.ttsEngine);

  // Populate voices for the active engine, restoring its saved selection.
  await loadVoices(voiceSelect, settings.ttsEngine, currentVoiceForEngine(settings));

  // Hotkey capture for TTS
  initTtsHotkeyCapture();

  // Listeners
  ttsEnable.addEventListener("change", async () => {
    const enabled = ttsEnable.checked;
    ttsPanel.classList.toggle("hidden", !enabled);
    await updateSettings({ ttsEnabled: enabled });
  });

  ttsLocal.addEventListener("click", async () => {
    applyEngine("local");
    await updateSettings({ ttsEngine: "local" });
    await loadVoices(voiceSelect, "local", currentVoiceForEngine(getSettings()));
  });

  ttsCloud.addEventListener("click", async () => {
    applyEngine("cloud");
    await updateSettings({ ttsEngine: "cloud" });
    await loadVoices(voiceSelect, "cloud", currentVoiceForEngine(getSettings()));
  });

  voiceSelect.addEventListener("change", () => {
    const key = voiceKeyForEngine(getSettings().ttsEngine);
    void updateSettings({ [key]: voiceSelect.value });
  });

  rateSlider.addEventListener("input", () => {
    const val = Number(rateSlider.value);
    if (rateLabel) rateLabel.textContent = `${val} WPM`;
  });

  rateSlider.addEventListener("change", () => {
    void updateSettings({ ttsRate: Number(rateSlider.value) });
  });

  testBtn?.addEventListener("click", async () => {
    testBtn.disabled = true;
    try {
      await ipc.speakText("Hello, this is a test of the text to speech feature.");
    } catch (e) {
      console.error("TTS test error:", e);
    } finally {
      testBtn.disabled = false;
    }
  });

  // Playback controls — pause/resume only works for the cloud engine (the
  // local OS-voice engine has no pause primitive, only speak/stop).
  pauseBtn?.addEventListener("click", () => void ipc.pauseSpeaking());
  resumeBtn?.addEventListener("click", () => void ipc.resumeSpeaking());
  stopBtn?.addEventListener("click", () => void ipc.stopSpeaking());

  if (playbackRow && playbackHint && pauseBtn && resumeBtn && stopBtn) {
    void ipc.onSpeakingState((state) =>
      applySpeakingState(state, getSettings().ttsEngine, {
        playbackRow,
        playbackHint,
        pauseBtn,
        resumeBtn,
        stopBtn,
      })
    );
  }
}

interface PlaybackControls {
  playbackRow: HTMLElement;
  playbackHint: HTMLElement;
  pauseBtn: HTMLButtonElement;
  resumeBtn: HTMLButtonElement;
  stopBtn: HTMLButtonElement;
}

// Auto-hides the playback row a few seconds after an error so the failure is
// visible but doesn't linger forever.
const ERROR_DISPLAY_MS = 6000;
let errorClearTimer: ReturnType<typeof setTimeout> | undefined;

function applySpeakingState(
  state: SpeakingState,
  engine: string,
  controls: PlaybackControls
): void {
  const { playbackRow, playbackHint, pauseBtn, resumeBtn, stopBtn } = controls;
  const isCloud = engine === "cloud";

  if (errorClearTimer !== undefined) {
    clearTimeout(errorClearTimer);
    errorClearTimer = undefined;
  }

  const hidden = state === "idle";
  playbackRow.classList.toggle("hidden", hidden);
  playbackRow.classList.toggle("error", state === "error");

  // Nothing is playing while loading or after an error, so there is nothing to
  // stop, pause, or resume.
  const playing = state === "speaking" || state === "paused";
  stopBtn.classList.toggle("hidden", !playing);
  // Pause/resume only make sense for cloud audio playback — the local
  // OS-voice engine can only be stopped, never truly paused.
  pauseBtn.classList.toggle("hidden", !isCloud || state !== "speaking");
  resumeBtn.classList.toggle("hidden", !isCloud || state !== "paused");

  switch (state) {
    case "loading":
      playbackHint.textContent = "Preparing speech…";
      break;
    case "speaking":
      playbackHint.textContent = "Reading selection aloud…";
      break;
    case "paused":
      playbackHint.textContent = "Paused";
      break;
    case "error":
      playbackHint.textContent =
        "Speech failed — check your connection or Groq API key.";
      errorClearTimer = setTimeout(() => {
        playbackRow.classList.add("hidden");
        playbackRow.classList.remove("error");
        errorClearTimer = undefined;
      }, ERROR_DISPLAY_MS);
      break;
    case "idle":
      break;
  }
}

function applyEngine(engine: string): void {
  const ttsLocal = document.getElementById("tts-engine-local");
  const ttsCloud = document.getElementById("tts-engine-cloud");
  ttsLocal?.classList.toggle("active", engine === "local");
  ttsCloud?.classList.toggle("active", engine === "cloud");
}

async function loadVoices(
  select: HTMLSelectElement,
  engine: string,
  currentVoice: string
): Promise<void> {
  try {
    const voices = await ipc.listVoices();
    select.innerHTML = "";

    if (voices.length === 0) {
      const opt = document.createElement("option");
      opt.value = "";
      opt.textContent = "Default";
      select.appendChild(opt);
      return;
    }

    for (const v of voices) {
      const opt = document.createElement("option");
      opt.value = v.id;
      opt.textContent = v.name;
      select.appendChild(opt);
    }

    if (currentVoice && voices.some((v) => v.id === currentVoice)) {
      select.value = currentVoice;
    } else {
      select.value = voices[0].id;
      await updateSettings({ [voiceKeyForEngine(engine)]: voices[0].id });
    }
  } catch (e) {
    console.error("Failed to load voices:", e);
  }
}
