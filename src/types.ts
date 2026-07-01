export interface Settings {
  microphone: string;
  engine: string;
  whisperModel: string;
  groqApiKey: string;
  recordingMode: string;
  hotkey: string;
  onboardingDone: boolean;
  ttsEnabled: boolean;
  ttsEngine: string;
  ttsHotkey: string;
  ttsVoiceLocal: string;
  ttsVoiceCloud: string;
  ttsRate: number;
  showConsole: boolean;
  runInBackground: boolean;
}

export interface VoiceInfo {
  id: string;
  name: string;
  engine: string;
}

export type SpeakingState = "loading" | "speaking" | "paused" | "idle" | "error";

export interface MicDevice {
  name: string;
  is_default: boolean;
}

export interface DownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

export type RecordingState = "Ready" | "Recording" | "Transcribing";

export interface HistoryEntry {
  text: string;
  timestamp: number;
  engine: string;
}

export interface TranscriptionDone {
  text: string;
  error: string | null;
}
