import { ipc } from "./ipc.ts";
import type { Settings } from "./types.ts";

// Definite assignment: loadSettings() must be called before getSettings().
let current!: Settings;

export async function loadSettings(): Promise<Settings> {
  current = await ipc.getSettings();
  return current;
}

export function getSettings(): Settings {
  return current;
}

export async function updateSettings(partial: Partial<Settings>): Promise<void> {
  current = { ...current, ...partial };
  await ipc.saveSettings(current);
}
