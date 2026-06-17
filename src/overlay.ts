import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const root = document.documentElement;
const SPEAKING_CLASSES = ["loading", "speaking", "paused"];

// ── Mic level → dynamic glow ──────────────────────────────
listen<number>("mic-level", (event) => {
  root.style.setProperty("--level", String(event.payload));
});

// ── Sound cues ────────────────────────────────────────────
// Create the AudioContext eagerly so it starts in "running" state on
// WebView2 (Edge) which doesn't enforce the autoplay policy for WebViews.
let audioCtx: AudioContext | null = null;
try {
  audioCtx = new AudioContext();
} catch (_) {
  // Will retry lazily on first tone
}

function getAudioCtx(): AudioContext | null {
  if (!audioCtx) {
    try {
      audioCtx = new AudioContext();
    } catch (_) {
      return null;
    }
  }
  if (audioCtx.state === "suspended") {
    audioCtx.resume().catch(() => {});
  }
  return audioCtx;
}

function playTone(
  freq: number,
  duration: number,
  type: OscillatorType = "sine"
): void {
  const ctx = getAudioCtx();
  if (!ctx) return;
  try {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.type = type;
    osc.frequency.value = freq;
    gain.gain.setValueAtTime(0.18, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + duration);
    osc.start(ctx.currentTime);
    osc.stop(ctx.currentTime + duration);
  } catch (_) {}
}

let errorClearTimer: number | undefined;

listen<string>("speaking-state", (event) => {
  const mic = document.getElementById("mic");
  if (!mic) return;

  if (errorClearTimer !== undefined) {
    clearTimeout(errorClearTimer);
    errorClearTimer = undefined;
  }

  mic.classList.remove(...SPEAKING_CLASSES, "error");
  const state = event.payload;
  const busy =
    mic.classList.contains("recording") ||
    mic.classList.contains("transcribing");
  if (busy) return;

  if (SPEAKING_CLASSES.includes(state)) {
    mic.classList.add(state);
  } else if (state === "error") {
    // Flash red so a failed read-aloud is visibly distinct from "still
    // loading", then clear so the overlay returns to its idle state.
    mic.classList.add("error");
    errorClearTimer = window.setTimeout(() => {
      mic.classList.remove("error");
      errorClearTimer = undefined;
    }, 2500);
  }
});

// Click the indicator while it's reading aloud to stop playback immediately —
// the only other way to interrupt it is re-pressing the read-aloud hotkey.
document.getElementById("mic")?.addEventListener("click", () => {
  const mic = document.getElementById("mic");
  if (mic && SPEAKING_CLASSES.some((cls) => mic.classList.contains(cls))) {
    void invoke("stop_speaking");
  }
});

let prevState = "Ready";
listen<string>("recording-state", (event) => {
  const state = event.payload;
  const mic = document.getElementById("mic");

  if (state === "Recording" && prevState === "Ready") {
    playTone(880, 0.12);
    setTimeout(() => playTone(1100, 0.1), 80);
  } else if (state !== "Recording" && prevState === "Recording") {
    playTone(660, 0.12);
    setTimeout(() => playTone(440, 0.12), 80);
    root.style.setProperty("--level", "0");
  }

  if (mic) {
    mic.classList.toggle("recording", state === "Recording");
    mic.classList.toggle("transcribing", state === "Transcribing");
    if (state === "Recording" || state === "Transcribing") {
      mic.classList.remove(...SPEAKING_CLASSES, "error");
    }
  }

  prevState = state;
});
