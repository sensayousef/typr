import { ipc } from "../ipc.ts";
import { getSettings, updateSettings } from "../store.ts";
import { keyEventToTauriShortcut } from "./hotkey-capture.ts";

const TOTAL_STEPS = 5;

function el<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

function showStep(stepIndex: number): void {
  document.querySelectorAll<HTMLElement>(".ob-step").forEach((s, i) => {
    s.classList.toggle("active", i === stepIndex);
  });
  document.querySelectorAll<HTMLElement>(".ob-dot").forEach((d, i) => {
    d.classList.toggle("active", i === stepIndex);
  });

  const nextBtn = el<HTMLButtonElement>("ob-next");
  const skipBtn = el<HTMLButtonElement>("ob-skip");

  if (stepIndex === TOTAL_STEPS - 1) {
    nextBtn.textContent = "Start using Robin";
    skipBtn.classList.add("hidden");
  } else {
    nextBtn.textContent = stepIndex === 0 ? "Get started →" : "Next →";
    skipBtn.classList.remove("hidden");
  }
}

async function populateMicSelect(): Promise<void> {
  const select = el<HTMLSelectElement>("ob-mic-select");
  const mics = await ipc.listMicrophones();
  select.innerHTML = "";

  const defaultOpt = document.createElement("option");
  defaultOpt.value = "default";
  defaultOpt.textContent = "System Default";
  select.appendChild(defaultOpt);

  for (const mic of mics) {
    const opt = document.createElement("option");
    opt.value = mic.name;
    opt.textContent = mic.name + (mic.is_default ? " (default)" : "");
    if (mic.is_default) opt.selected = true;
    select.appendChild(opt);
  }
}

function setupHotkeyCapture(): void {
  const btn = el<HTMLButtonElement>("ob-hotkey-btn");
  const kbd = el<HTMLElement>("ob-hotkey-text");
  let listening = false;

  btn.addEventListener("click", () => {
    if (listening) return;
    listening = true;
    btn.classList.add("listening");
    kbd.textContent = "Press a key combination...";
  });

  window.addEventListener("keydown", (e) => {
    if (!listening) return;
    e.preventDefault();

    const combo = keyEventToTauriShortcut(e);
    if (!combo) return;
    kbd.textContent = combo;
    btn.classList.remove("listening");
    listening = false;
  });
}

async function finish(): Promise<void> {
  const mic = el<HTMLSelectElement>("ob-mic-select").value || "default";
  const hotkey = el<HTMLElement>("ob-hotkey-text").textContent ?? "Ctrl+Shift+Space";
  const isCloud = el<HTMLButtonElement>("ob-engine-cloud").classList.contains("active");
  const groqKey = el<HTMLInputElement>("ob-groq-key").value.trim();

  const tauri_hotkey = hotkey
    .replace(/Ctrl/g, "Ctrl")
    .replace(/Alt/g, "Alt")
    .replace(/Shift/g, "Shift");

  await updateSettings({
    microphone: mic,
    hotkey: tauri_hotkey,
    engine: isCloud ? "cloud" : "local",
    groqApiKey: isCloud ? groqKey : getSettings().groqApiKey,
    onboardingDone: true,
  });

  try {
    await ipc.updateHotkey(tauri_hotkey);
  } catch (_) {}

  dismiss();
}

function dismiss(): void {
  el("onboarding").classList.add("hidden");
}

export async function initOnboarding(): Promise<void> {
  const settings = getSettings();
  if (settings.onboardingDone) return;

  el("onboarding").classList.remove("hidden");

  await populateMicSelect();
  setupHotkeyCapture();

  // Sync hotkey display with current setting
  el("ob-hotkey-text").textContent = settings.hotkey.replace(/CmdOrCtrl/g, "Ctrl");
  el("ob-final-hotkey").textContent = settings.hotkey.replace(/CmdOrCtrl/g, "Ctrl");

  // Engine toggle
  const localBtn = el<HTMLButtonElement>("ob-engine-local");
  const cloudBtn = el<HTMLButtonElement>("ob-engine-cloud");
  const groqInput = el<HTMLInputElement>("ob-groq-key");

  localBtn.addEventListener("click", () => {
    localBtn.classList.add("active");
    cloudBtn.classList.remove("active");
    groqInput.classList.add("hidden");
  });

  cloudBtn.addEventListener("click", () => {
    cloudBtn.classList.add("active");
    localBtn.classList.remove("active");
    groqInput.classList.remove("hidden");
  });

  // Navigation
  let currentStep = 0;
  showStep(currentStep);

  el("ob-next").addEventListener("click", async () => {
    if (currentStep === TOTAL_STEPS - 1) {
      await finish();
      return;
    }

    // Update final hotkey display when moving to last step
    if (currentStep === 2) {
      el("ob-final-hotkey").textContent =
        el("ob-hotkey-text").textContent ?? settings.hotkey;
    }

    currentStep++;
    showStep(currentStep);
  });

  el("ob-skip").addEventListener("click", async () => {
    await updateSettings({ onboardingDone: true });
    dismiss();
  });
}
