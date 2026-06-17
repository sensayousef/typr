import { ipc } from "../ipc.ts";
import { getSettings, updateSettings } from "../store.ts";

export function formatHotkeyDisplay(hotkey: string): string {
  return hotkey
    .replace("CmdOrCtrl", "Ctrl")
    .replace("CommandOrControl", "Ctrl");
}

function keyEventToTauriShortcut(event: KeyboardEvent): string | null {
  const modifiers: string[] = [];
  if (event.ctrlKey || event.metaKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");

  // WebKitGTK reports F13–F24 as "Unidentified"/undefined in event.key;
  // event.code still carries the physical key name (e.g. "F22").
  const FUNCTION_KEY_RE = /^F\d+$/;
  let key = event.key;
  if ((!key || !FUNCTION_KEY_RE.test(key)) && FUNCTION_KEY_RE.test(event.code)) {
    key = event.code;
  }
  if (!key || key === "Unidentified") return null;
  if (["Control", "Alt", "Shift", "Meta"].includes(key)) return null;

  const isFunctionKey = FUNCTION_KEY_RE.test(key);
  if (modifiers.length === 0 && !isFunctionKey) return null;

  let formattedKey: string;
  if (key === " ") formattedKey = "Space";
  else if (key.length === 1) formattedKey = key.toUpperCase();
  else formattedKey = key;

  return [...modifiers, formattedKey].join("+");
}

interface HotkeyCaptureOptions {
  btnId: string;
  textId: string;
  getCurrentKey: () => string;
  applyFn: (shortcut: string) => Promise<void>;
}

function initHotkeyCaptureFor(opts: HotkeyCaptureOptions): void {
  const btn = document.getElementById(opts.btnId) as HTMLButtonElement;
  const text = document.getElementById(opts.textId)!;
  if (!btn || !text) return;

  text.textContent = formatHotkeyDisplay(opts.getCurrentKey());

  let listening = false;

  btn.addEventListener("click", () => {
    if (listening) return;
    listening = true;
    btn.classList.add("listening");
    text.textContent = "Press keys…";

    function cleanup(): void {
      listening = false;
      btn.classList.remove("listening");
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("click", onClickAway, true);
    }

    function cancel(): void {
      cleanup();
      text.textContent = formatHotkeyDisplay(opts.getCurrentKey());
    }

    async function applyHotkey(shortcut: string): Promise<void> {
      try {
        await opts.applyFn(shortcut);
        text.textContent = formatHotkeyDisplay(shortcut);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        text.textContent = `Error: ${msg}`;
        setTimeout(() => {
          text.textContent = formatHotkeyDisplay(opts.getCurrentKey());
        }, 3000);
      }
    }

    function onKeyDown(e: KeyboardEvent): void {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        cancel();
        return;
      }
      const shortcut = keyEventToTauriShortcut(e);
      if (!shortcut) return;
      cleanup();
      void applyHotkey(shortcut);
    }

    function onClickAway(e: MouseEvent): void {
      if (!btn.contains(e.target as Node)) cancel();
    }

    window.addEventListener("keydown", onKeyDown, true);
    setTimeout(() => window.addEventListener("click", onClickAway, true), 0);
  });
}

export function initHotkeyCapture(): void {
  initHotkeyCaptureFor({
    btnId: "hotkey-btn",
    textId: "hotkey-text",
    getCurrentKey: () => getSettings().hotkey,
    applyFn: async (shortcut) => {
      await ipc.updateHotkey(shortcut);
      await updateSettings({ hotkey: shortcut });
    },
  });
}

export function initTtsHotkeyCapture(): void {
  initHotkeyCaptureFor({
    btnId: "tts-hotkey-btn",
    textId: "tts-hotkey-text",
    getCurrentKey: () => getSettings().ttsHotkey,
    applyFn: async (shortcut) => {
      await ipc.updateTtsHotkey(shortcut);
      await updateSettings({ ttsHotkey: shortcut });
    },
  });
}
