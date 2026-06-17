pub mod notifier;
pub mod state;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

use notifier::{StateNotifier, TauriNotifier};
use state::{RecordingState, TranscribeGuard};

use crate::audio::{dsp, AudioCapture};
use crate::cleanup::cleanup_text;
use crate::engine::Engine;
use crate::paste::paste_text;
use crate::settings::Settings;

pub struct Recorder {
    pub(crate) state: Arc<Mutex<RecordingState>>,
    capture: Arc<Mutex<AudioCapture>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RecordingState::Ready)),
            capture: Arc::new(Mutex::new(AudioCapture::new())),
        }
    }

    pub fn get_state(&self) -> RecordingState {
        self.state.lock().unwrap().clone()
    }

    pub fn start_recording(&self, app: &AppHandle, mic_name: &str) -> Result<(), String> {
        let notifier = TauriNotifier { app: app.clone() };

        let mut state = self.state.lock().unwrap();
        if *state != RecordingState::Ready {
            return Err("Already recording or transcribing".to_string());
        }

        let app_for_level = app.clone();
        let mut capture = self.capture.lock().unwrap();
        capture.start(mic_name, move |level| {
            let _ = app_for_level.emit("mic-level", level);
        })?;

        *state = RecordingState::Recording;
        notifier.notify(&RecordingState::Recording);
        Ok(())
    }

    pub async fn stop_and_transcribe(
        &self,
        app: &AppHandle,
        settings: &Settings,
        app_dir: &PathBuf,
    ) -> Result<String, String> {
        let notifier = Arc::new(TauriNotifier { app: app.clone() });

        // Enter Transcribing state. The RAII guard guarantees state returns to
        // Ready on every exit — including early returns via `?`.
        let _guard = {
            let mut state = self.state.lock().unwrap();
            if *state != RecordingState::Recording {
                return Err("Not currently recording".to_string());
            }
            *state = RecordingState::Transcribing;
            notifier.notify(&RecordingState::Transcribing);
            TranscribeGuard::new(self.state.clone(), notifier)
        };

        // Stop audio capture and retrieve raw samples.
        let captured = {
            let mut capture = self.capture.lock().unwrap();
            capture.stop()?
        };

        // DSP pipeline: downmix → resample → write WAV.
        let mono = dsp::to_mono(&captured.samples, captured.channels);
        let resampled = dsp::resample(&mono, captured.sample_rate, 16_000);

        // Skip transcription if the audio is too short (< 0.3 s) or nearly silent.
        // This prevents Whisper from hallucinating on silence or a muted mic.
        let duration_secs = resampled.len() as f32 / 16_000.0;
        let rms = dsp::compute_rms(&resampled);
        if duration_secs < 0.3 || rms < 0.002 {
            println!("[Typr] Skipping transcription: duration={:.2}s rms={:.4}", duration_secs, rms);
            let _ = app.emit("transcription-done", serde_json::json!({ "text": "", "error": null }));
            return Ok(String::new());
        }

        let temp_path = app_dir.join("temp_recording.wav");
        dsp::write_wav(&resampled, 16_000, &temp_path)?;

        // Dispatch to the configured transcription engine.
        let transcription = Engine::from_settings(settings, app_dir)
            .transcribe(app, &temp_path)
            .await;

        let _ = std::fs::remove_file(&temp_path);

        // Always surface the result (or error) to the frontend before propagating.
        match &transcription {
            Ok(text) => {
                let _ = app.emit("transcription-done", serde_json::json!({ "text": text, "error": null }));
            }
            Err(e) => {
                let _ = app.emit("transcription-done", serde_json::json!({ "text": "", "error": e }));
            }
        }

        let raw_text = transcription?;
        let cleaned = cleanup_text(&raw_text);
        if !cleaned.is_empty() {
            paste_text(&cleaned)?;
        }

        Ok(cleaned)
        // _guard drops here → state resets to Ready and frontend is notified.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_ready() {
        let recorder = Recorder::new();
        assert_eq!(recorder.get_state(), RecordingState::Ready);
    }
}
