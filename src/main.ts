import { loadSettings } from "./store.ts";
import { initStatus } from "./ui/status.ts";
import { initNav } from "./ui/nav.ts";
import { initWindowDrag } from "./ui/window-drag.ts";
import { initWindowControls } from "./ui/window-controls.ts";
import { initEnginePanel } from "./ui/engine-panel.ts";
import { initMicPanel } from "./ui/mic-panel.ts";
import { initModePanel } from "./ui/mode-panel.ts";
import { initHotkeyCapture } from "./ui/hotkey-capture.ts";
import { initStartupPanel } from "./ui/startup-panel.ts";
import { initHistoryPanel } from "./ui/history-panel.ts";
import { initOnboarding } from "./ui/onboarding.ts";
import { initTtsPanel } from "./ui/tts-panel.ts";

async function init(): Promise<void> {
  // Settings must load before any UI module calls getSettings().
  await loadSettings();

  initNav();
  initWindowDrag();
  initWindowControls();
  initModePanel();
  initHotkeyCapture();

  await Promise.all([
    initStatus(),
    initMicPanel(),
    initEnginePanel(),
    initStartupPanel(),
    initHistoryPanel(),
    initTtsPanel(),
  ]);

  // Onboarding runs after all panels are ready
  await initOnboarding();
}

void init();
