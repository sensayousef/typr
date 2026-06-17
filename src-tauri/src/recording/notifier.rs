use tauri::{AppHandle, Emitter, Manager};

use super::state::RecordingState;

pub trait StateNotifier: Send + Sync {
    fn notify(&self, state: &RecordingState);
}

/// Tauri implementation: emits the `recording-state` event and updates the overlay window.
pub struct TauriNotifier {
    pub app: AppHandle,
}

impl StateNotifier for TauriNotifier {
    fn notify(&self, state: &RecordingState) {
        let _ = self.app.emit("recording-state", state.clone());

        if let Some(overlay) = self.app.get_webview_window("overlay") {
            let class = match state {
                RecordingState::Ready => "mic",
                RecordingState::Recording => "mic recording",
                RecordingState::Transcribing => "mic transcribing",
            };
            let js = format!("document.getElementById('mic').className = '{}';", class);
            let _ = overlay.eval(&js);
        }

        let tooltip = match state {
            RecordingState::Ready => "Typr",
            RecordingState::Recording => "Typr — Recording...",
            RecordingState::Transcribing => "Typr — Transcribing...",
        };
        if let Some(tray) = self.app.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(tooltip));
        }
    }
}
