use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use robin_lib::{audio, downloader, recording::state::RecordingState, settings::Settings, transcribe_local};

use crate::app_state::AppState;
use crate::history::HistoryEntry;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    settings.save(&state.app_dir)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub fn list_microphones() -> Vec<audio::MicDevice> {
    audio::list_microphones()
}

#[tauri::command]
pub fn get_recording_state(state: State<AppState>) -> RecordingState {
    state.recorder.get_state()
}

#[tauri::command]
pub fn check_model_downloaded(state: State<AppState>, model_size: String) -> bool {
    let model_file = transcribe_local::model_filename(&model_size);
    state.app_dir.join(&model_file).exists()
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_size: String,
) -> Result<(), String> {
    let url = transcribe_local::model_download_url(&model_size);
    let model_file = transcribe_local::model_filename(&model_size);
    let dest = state.app_dir.join(&model_file);
    downloader::download_model(app, &url, &dest).await
}

#[tauri::command]
pub async fn toggle_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    crate::hotkey::do_toggle_recording(&app, state.inner()).await
}

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Vec<HistoryEntry> {
    state.history.lock().unwrap().list().to_vec()
}

#[tauri::command]
pub fn clear_history(state: State<AppState>) {
    state.history.lock().unwrap().clear(&state.app_dir);
}

#[tauri::command]
pub fn get_autostart_enabled(app: AppHandle) -> bool {
    if cfg!(debug_assertions) {
        return false;
    }

    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    if cfg!(debug_assertions) && enabled {
        return Err(
            "Launch on startup is only available from a built Robin app. Run `npm run app:build`, then launch Robin with `npm run app`.".into(),
        );
    }

    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

/// Show or hide the debug console immediately. Persistence of the preference
/// flows through the normal `save_settings` path (the frontend updates the
/// `showConsole` setting alongside this call).
#[tauri::command]
pub fn set_console_visible(enabled: bool) {
    crate::console::set_visible(enabled);
}
