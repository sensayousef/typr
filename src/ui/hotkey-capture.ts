import { ipc } from "../ipc.ts";
import { getSettings, updateSettings } from "../store.ts";

export function formatHotkeyDisplay(hotkey: string): string {
  return hotkey
    .replace("CmdOrCtrl", "Ctrl")
    .replace("CommandOrControl", "Ctrl");
}

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);
const FUNCTION_KEY_RE = /^F\d+$/i;
const CAPTURE_BLOCKLIST = new Set(["Ctrl+C", "Ctrl+V"]);

export function normalizeShortcutText(value: string): string | null {
  const parts = value
    .trim()
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);

  if (parts.length === 0) return null;

  const modifiers: string[] = [];
  let key: string | null = null;

  for (const part of parts) {
    const token = part.toLowerCase();
    if (["control", "ctrl"].includes(token)) modifiers.push("Ctrl");
    else if (["option", "alt"].includes(token)) modifiers.push("Alt");
    else if (token === "shift") modifiers.push("Shift");
    else if (["cmdorctrl", "cmdorcontrol", "commandorcontrol"].includes(token)) {
      modifiers.push("CmdOrCtrl");
    } else if (["meta", "cmd", "command", "super", "win", "windows"].includes(token)) {
      modifiers.push("Super");
    } else if (key) {
      return null;
    } else {
      key = normalizeShortcutKey(part);
    }
  }

  if (!key || MODIFIER_KEYS.has(key)) return null;

  const uniqueModifiers = [...new Set(modifiers)];
  const isFunctionKey = FUNCTION_KEY_RE.test(key);
  if (uniqueModifiers.length === 0 && !isFunctionKey) return null;

  return [...uniqueModifiers, key].join("+");
}

export function keyEventToTauriShortcut(event: KeyboardEvent): string | null {
  const modifiers: string[] = [];
  if (event.ctrlKey || event.metaKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");

  // WebKitGTK reports F13–F24 as "Unidentified"/undefined in event.key;
  // event.code still carries the physical key name (e.g. "F22").
  let key = event.key;
  if ((!key || !FUNCTION_KEY_RE.test(key)) && FUNCTION_KEY_RE.test(event.code)) {
    key = event.code;
  }
  if (!key || key === "Unidentified") return null;
  if (MODIFIER_KEYS.has(key)) return null;

  const isFunctionKey = FUNCTION_KEY_RE.test(key);
  if (modifiers.length === 0 && !isFunctionKey) return null;

  const formattedKey = normalizeShortcutKey(key);
  const shortcut = [...modifiers, formattedKey].join("+");
  if (CAPTURE_BLOCKLIST.has(shortcut)) return null;

  return shortcut;
}

function normalizeShortcutKey(key: string): string {
  let formattedKey: string;
  if (key === " ") formattedKey = "Space";
  else if (key.length === 1) formattedKey = key.toUpperCase();
  else formattedKey = key;
  return formattedKey.replace(/^f(\d+)$/i, "F$1");
}

interface HotkeyCaptureOptions {
  btnId: string;
  textId: string;
  inputId: string;
  getCurrentKey: () => string;
  applyFn: (shortcut: string) => Promise<void>;
}

function initHotkeyCaptureFor(opts: HotkeyCaptureOptions): void {
  const btn = document.getElementById(opts.btnId) as HTMLButtonElement;
  const text = document.getElementById(opts.textId)!;
  const input = document.getElementById(opts.inputId) as HTMLInputElement | null;
  if (!btn || !text) return;

  function syncHotkeyDisplay(shortcut: string): void {
    const display = formatHotkeyDisplay(shortcut);
    text.textContent = display;
    if (input) {
      input.value = display;
      input.style.setProperty("--hotkey-input-ch", `${display.length}ch`);
    }
  }

  syncHotkeyDisplay(opts.getCurrentKey());

  let listening = false;

  async function applyHotkey(shortcut: string): Promise<void> {
    try {
      await opts.applyFn(shortcut);
      syncHotkeyDisplay(shortcut);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      text.textContent = `Error: ${msg}`;
      setTimeout(() => {
        syncHotkeyDisplay(opts.getCurrentKey());
      }, 3000);
    }
  }

  input?.addEventListener("change", () => {
    const shortcut = normalizeShortcutText(input.value);
    if (!shortcut) {
      syncHotkeyDisplay(opts.getCurrentKey());
      return;
    }
    void applyHotkey(shortcut);
  });

  input?.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      input.blur();
    }
    e.stopPropagation();
  });

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
      syncHotkeyDisplay(opts.getCurrentKey());
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
    inputId: "hotkey-input",
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
    inputId: "tts-hotkey-input",
    getCurrentKey: () => getSettings().ttsHotkey,
    applyFn: async (shortcut) => {
      await ipc.updateTtsHotkey(shortcut);
      await updateSettings({ ttsHotkey: shortcut });
    },
  });
}
