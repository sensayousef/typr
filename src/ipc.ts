import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { Settings, MicDevice, DownloadProgress, TranscriptionDone, HistoryEntry, VoiceInfo, SpeakingState } from "./types.ts";

export const ipc = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<void>("save_settings", { settings }),
  listMicrophones: () => invoke<MicDevice[]>("list_microphones"),
  checkModelDownloaded: (modelSize: string) =>
    invoke<boolean>("check_model_downloaded", { modelSize }),
  downloadModel: (modelSize: string) =>
    invoke<void>("download_model", { modelSize }),
  updateHotkey: (hotkey: string) =>
    invoke<void>("update_hotkey", { hotkey }),
  getAutostartEnabled: () => invoke<boolean>("get_autostart_enabled"),
  setAutostart: (enabled: boolean) =>
    invoke<void>("set_autostart", { enabled }),
  setConsoleVisible: (enabled: boolean) =>
    invoke<void>("set_console_visible", { enabled }),
  getHistory: () => invoke<HistoryEntry[]>("get_history"),
  clearHistory: () => invoke<void>("clear_history"),
  onHistoryUpdated: (cb: () => void): Promise<UnlistenFn> =>
    listen("history-updated", () => cb()),

  onRecordingState: (cb: (state: string) => void): Promise<UnlistenFn> =>
    listen<string>("recording-state", (e) => cb(e.payload)),

  onDownloadProgress: (
    cb: (progress: DownloadProgress) => void
  ): Promise<UnlistenFn> =>
    listen<DownloadProgress>("download-progress", (e) => cb(e.payload)),

  onTranscriptionDone: (
    cb: (result: TranscriptionDone) => void
  ): Promise<UnlistenFn> =>
    listen<TranscriptionDone>("transcription-done", (e) => cb(e.payload)),

  listVoices: () => invoke<VoiceInfo[]>("list_voices_cmd"),
  stopSpeaking: () => invoke<void>("stop_speaking"),
  pauseSpeaking: () => invoke<void>("pause_speaking"),
  resumeSpeaking: () => invoke<void>("resume_speaking"),
  speakText: (text: string) => invoke<void>("speak_text_cmd", { text }),
  updateTtsHotkey: (hotkey: string) =>
    invoke<void>("update_tts_hotkey", { hotkey }),

  onSpeakingState: (cb: (state: SpeakingState) => void): Promise<UnlistenFn> =>
    listen<SpeakingState>("speaking-state", (e) => cb(e.payload)),

  convertMarkitdown: (input: string) =>
    invoke<string>("convert_markitdown", { input }),
  saveMarkdown: (path: string, content: string) =>
    invoke<void>("save_markdown", { path, content }),
};
