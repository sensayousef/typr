import { ipc } from "../ipc.ts";
import type { RecordingState } from "../types.ts";

const DOT_CLASS: Record<RecordingState, string> = {
  Ready: "ready",
  Recording: "recording",
  Transcribing: "transcribing",
};

const STATE_TEXT: Record<RecordingState, string> = {
  Ready: "Ready",
  Recording: "Recording...",
  Transcribing: "Transcribing...",
};

/** After this many ms without a Ready event, force the UI back to Ready. */
const WATCHDOG_MS = 180_000;

/** How long to show a transient error/result message before returning to Ready. */
const FEEDBACK_MS = 5_000;

export async function initStatus(): Promise<void> {
  const dot = document.getElementById("status-dot")!;
  const text = document.getElementById("status-text")!;
  let watchdog: ReturnType<typeof setTimeout> | null = null;
  let feedbackTimer: ReturnType<typeof setTimeout> | null = null;

  function applyState(state: RecordingState): void {
    if (watchdog !== null) {
      clearTimeout(watchdog);
      watchdog = null;
    }
    if (feedbackTimer !== null) {
      clearTimeout(feedbackTimer);
      feedbackTimer = null;
    }
    dot.className = DOT_CLASS[state];
    text.textContent = STATE_TEXT[state];

    if (state === "Transcribing") {
      watchdog = setTimeout(() => applyState("Ready"), WATCHDOG_MS);
    }
  }

  function showFeedback(message: string, isError: boolean): void {
    if (feedbackTimer !== null) clearTimeout(feedbackTimer);
    dot.className = isError ? "error" : "ready";
    text.textContent = message;
    feedbackTimer = setTimeout(() => {
      dot.className = "ready";
      text.textContent = STATE_TEXT.Ready;
      feedbackTimer = null;
    }, FEEDBACK_MS);
  }

  await ipc.onRecordingState((raw) => {
    const state = raw as RecordingState;
    if (state in DOT_CLASS) applyState(state);
  });

  await ipc.onTranscriptionDone(({ text: transcribed, error }) => {
    if (error) {
      showFeedback(`Error: ${error}`, true);
    } else if (!transcribed) {
      showFeedback("Nothing transcribed", false);
    }
    // On success with text, the state will naturally return to Ready via the backend event.
  });
}
