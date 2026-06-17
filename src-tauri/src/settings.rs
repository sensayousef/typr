use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub microphone: String,
    pub engine: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "groqApiKey")]
    pub groq_api_key: String,
    #[serde(rename = "recordingMode")]
    pub recording_mode: String,
    pub hotkey: String,
    #[serde(rename = "onboardingDone", default)]
    pub onboarding_done: bool,
    #[serde(rename = "ttsEnabled", default)]
    pub tts_enabled: bool,
    #[serde(rename = "ttsEngine", default = "default_tts_engine")]
    pub tts_engine: String,
    #[serde(rename = "ttsHotkey", default = "default_tts_hotkey")]
    pub tts_hotkey: String,
    /// Last-selected voice per engine. Local (OS) and cloud (Orpheus) use
    /// disjoint voice-id namespaces, so a single shared field would be
    /// clobbered every time the user switches engines. Persisting one per
    /// engine lets each remember its own selection across restarts.
    #[serde(rename = "ttsVoiceLocal", default)]
    pub tts_voice_local: String,
    #[serde(rename = "ttsVoiceCloud", default)]
    pub tts_voice_cloud: String,
    #[serde(rename = "ttsRate", default = "default_tts_rate")]
    pub tts_rate: u32,
}

fn default_tts_engine() -> String {
    "local".to_string()
}

fn default_tts_hotkey() -> String {
    "CmdOrCtrl+Shift+R".to_string()
}

fn default_tts_rate() -> u32 {
    175
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            microphone: "default".to_string(),
            engine: "local".to_string(),
            whisper_model: "small".to_string(),
            groq_api_key: String::new(),
            recording_mode: "toggle".to_string(),
            hotkey: "CmdOrCtrl+Shift+Space".to_string(),
            onboarding_done: false,
            tts_enabled: false,
            tts_engine: default_tts_engine(),
            tts_hotkey: default_tts_hotkey(),
            tts_voice_local: String::new(),
            tts_voice_cloud: String::new(),
            tts_rate: default_tts_rate(),
        }
    }
}

impl Settings {
    /// The voice id to use for the currently selected TTS engine. Local and
    /// cloud voices live in separate namespaces, so each is stored separately.
    pub fn tts_voice_for_engine(&self) -> &str {
        match self.tts_engine.as_str() {
            "cloud" => &self.tts_voice_cloud,
            _ => &self.tts_voice_local,
        }
    }

    pub fn config_path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("config.json")
    }

    pub fn load(app_dir: &PathBuf) -> Self {
        let path = Self::config_path(app_dir);
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, app_dir: &PathBuf) -> Result<(), String> {
        let path = Self::config_path(app_dir);
        fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.microphone, "default");
        assert_eq!(settings.engine, "local");
        assert_eq!(settings.whisper_model, "small");
        assert_eq!(settings.groq_api_key, "");
        assert_eq!(settings.recording_mode, "toggle");
        assert_eq!(settings.hotkey, "CmdOrCtrl+Shift+Space");
        assert!(!settings.tts_enabled);
        assert_eq!(settings.tts_engine, "local");
        assert_eq!(settings.tts_hotkey, "CmdOrCtrl+Shift+R");
        assert_eq!(settings.tts_voice_local, "");
        assert_eq!(settings.tts_voice_cloud, "");
        assert_eq!(settings.tts_rate, 175);
    }

    #[test]
    fn tts_voice_for_engine_picks_per_engine_field() {
        let mut settings = Settings::default();
        settings.tts_voice_local = "Microsoft David".to_string();
        settings.tts_voice_cloud = "diana".to_string();

        settings.tts_engine = "local".to_string();
        assert_eq!(settings.tts_voice_for_engine(), "Microsoft David");

        settings.tts_engine = "cloud".to_string();
        assert_eq!(settings.tts_voice_for_engine(), "diana");
    }

    #[test]
    fn test_save_and_load() {
        let dir = temp_dir().join("typr_test_settings");
        let _ = fs::remove_dir_all(&dir);

        let mut settings = Settings::default();
        settings.engine = "cloud".to_string();
        settings.groq_api_key = "test-key-123".to_string();

        settings.save(&dir).unwrap();
        let loaded = Settings::load(&dir);

        assert_eq!(loaded.engine, "cloud");
        assert_eq!(loaded.groq_api_key, "test-key-123");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = temp_dir().join("typr_test_missing");
        let _ = fs::remove_dir_all(&dir);
        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn test_load_corrupt_json_returns_default() {
        let dir = temp_dir().join("typr_test_corrupt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.json"), "not json").unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());

        let _ = fs::remove_dir_all(&dir);
    }
}
