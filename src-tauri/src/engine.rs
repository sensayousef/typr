use std::path::{Path, PathBuf};
use tauri::AppHandle;

use crate::settings::Settings;
use crate::transcribe_groq;
use crate::transcribe_local;

/// Strategy enum for transcription backends.
pub enum Engine {
    Local { model_path: PathBuf },
    Cloud { api_key: String },
}

impl Engine {
    /// Factory: build the correct engine from current settings.
    /// Falls back to Local for any unrecognised engine string.
    pub fn from_settings(settings: &Settings, app_dir: &Path) -> Self {
        match settings.engine.as_str() {
            "cloud" => Engine::Cloud {
                api_key: settings.groq_api_key.clone(),
            },
            _ => Engine::Local {
                model_path: app_dir
                    .join(transcribe_local::model_filename(&settings.whisper_model)),
            },
        }
    }

    /// Transcribe the WAV file at `audio_path`.
    pub async fn transcribe(&self, _app: &AppHandle, audio_path: &Path) -> Result<String, String> {
        match self {
            Engine::Local { model_path } => {
                transcribe_local::transcribe_local(model_path, &audio_path.to_path_buf()).await
            }
            Engine::Cloud { api_key } => {
                transcribe_groq::transcribe_groq(api_key, &audio_path.to_path_buf()).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settings(engine: &str) -> Settings {
        Settings {
            engine: engine.to_string(),
            whisper_model: "small".to_string(),
            groq_api_key: "key".to_string(),
            ..Settings::default()
        }
    }

    #[test]
    fn local_engine_selected_for_local_setting() {
        let e = Engine::from_settings(&make_settings("local"), Path::new("/tmp"));
        assert!(matches!(e, Engine::Local { .. }));
    }

    #[test]
    fn cloud_engine_selected_for_cloud_setting() {
        let e = Engine::from_settings(&make_settings("cloud"), Path::new("/tmp"));
        assert!(matches!(e, Engine::Cloud { .. }));
    }

    #[test]
    fn unknown_engine_falls_back_to_local() {
        let e = Engine::from_settings(&make_settings("unknown"), Path::new("/tmp"));
        assert!(matches!(e, Engine::Local { .. }));
    }

    #[test]
    fn local_engine_uses_correct_model_path() {
        let settings = make_settings("local");
        let app_dir = Path::new("/tmp/typr");
        let e = Engine::from_settings(&settings, app_dir);
        if let Engine::Local { model_path } = e {
            assert_eq!(model_path, PathBuf::from("/tmp/typr/ggml-small.bin"));
        } else {
            panic!("expected Local engine");
        }
    }
}
