use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use robin_lib::recording::state::RecordingState;

use crate::app_state::AppState;
use crate::history::HistoryEntry;

/// Core recording toggle logic shared by the Tauri command and the hotkey handler.
pub async fn do_toggle_recording(
    app: &AppHandle,
    state: &AppState,
) -> Result<String, String> {
    match state.recorder.get_state() {
        RecordingState::Ready => {
            let mic = state.settings.lock().unwrap().microphone.clone();
            state.recorder.start_recording(app, &mic)?;
            Ok("recording".to_string())
        }
        RecordingState::Recording => {
            let settings = state.settings.lock().unwrap().clone();
            let result = state
                .recorder
                .stop_and_transcribe(app, &settings, &state.app_dir)
                .await?;

            save_history(app, state, &result, &settings.engine);
            Ok(result)
        }
        RecordingState::Transcribing => Err("Currently transcribing, please wait".to_string()),
    }
}

fn save_history(app: &AppHandle, state: &AppState, text: &str, engine: &str) {
    if text.is_empty() {
        return;
    }
    let entry = HistoryEntry {
        text: text.to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        engine: engine.to_string(),
    };
    state.history.lock().unwrap().push(entry, &state.app_dir);
    let _ = app.emit("history-updated", ());
}

/// Register both the dictation hotkey and the TTS hotkey together.
/// Always call this instead of registering them individually — unregister_all()
/// is called once here to ensure neither key clobbers the other.
pub fn register_all_hotkeys(app: &AppHandle, dictation: &str, tts: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    if let Err(e) = gs.unregister_all() {
        eprintln!("[Robin] Failed to unregister shortcuts: {}", e);
    }

    let dictation_key = dictation.to_string();
    let tts_key = tts.to_string();

    let handle_d = app.clone();
    let tts_key_for_dictation = tts_key.clone();
    gs.on_shortcut(dictation, move |_app, shortcut, event| {
        println!("[Robin] Dictation hotkey: {:?} state={:?}", shortcut, event.state);
        let handle = handle_d.clone();
        let mode = handle
            .state::<AppState>()
            .settings
            .lock()
            .unwrap()
            .recording_mode
            .clone();
        let _ = tts_key_for_dictation.clone(); // keep alive
        match event.state {
            ShortcutState::Pressed => handle_pressed(handle, mode),
            ShortcutState::Released => handle_released(handle, mode),
        }
    })
    .map_err(|e| format!("Failed to register dictation hotkey '{}': {}", dictation_key, e))?;

    let handle_t = app.clone();
    gs.on_shortcut(tts, move |_app, shortcut, event| {
        println!("[Robin] TTS hotkey: {:?} state={:?}", shortcut, event.state);
        if event.state == ShortcutState::Pressed {
            handle_tts_pressed(handle_t.clone());
        }
    })
    .map_err(|e| format!("Failed to register TTS hotkey '{}': {}", tts_key, e))?;

    Ok(())
}

fn handle_pressed(app: AppHandle, mode: String) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        match mode.as_str() {
            "toggle" => match do_toggle_recording(&app, state.inner()).await {
                Ok(result) => println!("[Robin] Toggle result: {}", result),
                Err(e) => eprintln!("[Robin] Toggle error: {}", e),
            },
            "push-to-talk" => {
                if state.recorder.get_state() == RecordingState::Ready {
                    let mic = state.settings.lock().unwrap().microphone.clone();
                    match state.recorder.start_recording(&app, &mic) {
                        Ok(_) => println!("[Robin] PTT recording started"),
                        Err(e) => eprintln!("[Robin] PTT start error: {}", e),
                    }
                }
            }
            _ => {}
        }
    });
}

fn handle_released(app: AppHandle, mode: String) {
    if mode != "push-to-talk" {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if state.recorder.get_state() == RecordingState::Recording {
            let settings = state.settings.lock().unwrap().clone();
            match state
                .recorder
                .stop_and_transcribe(&app, &settings, &state.app_dir)
                .await
            {
                Ok(result) => {
                    println!("[Robin] PTT transcription: {}", result);
                    save_history(&app, state.inner(), &result, &settings.engine);
                }
                Err(e) => eprintln!("[Robin] PTT transcription error: {}", e),
            }
        }
    });
}

fn handle_tts_pressed(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::tts::do_toggle_speak(&app).await {
            eprintln!("[Robin] TTS error: {}", e);
        }
    });
}

#[tauri::command]
pub async fn update_hotkey(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    hotkey: String,
) -> Result<(), String> {
    let tts_hotkey = state.settings.lock().unwrap().tts_hotkey.clone();
    if hotkey == tts_hotkey {
        return Err("Dictation hotkey must be different from the TTS hotkey".to_string());
    }
    register_all_hotkeys(&app, &hotkey, &tts_hotkey)?;
    let mut settings = state.settings.lock().unwrap();
    settings.hotkey = hotkey;
    settings.save(&state.app_dir)?;
    Ok(())
}
