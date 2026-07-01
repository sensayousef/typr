use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use futures_util::stream::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use robin_lib::settings::Settings;

use crate::tts_groq;

/// Cap how many Orpheus synthesis requests are in flight at once. Firing every
/// chunk in parallel (one per ~200 chars) trips Groq's rate limits on medium or
/// large selections, which previously surfaced as the read-aloud silently
/// hanging. A small bound keeps long text reliable without serializing it.
const CLOUD_SYNTHESIS_CONCURRENCY: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub engine: String,
}

pub enum TtsEngine {
    Local,
    Cloud,
}

impl TtsEngine {
    pub fn from_settings(s: &Settings) -> Self {
        match s.tts_engine.as_str() {
            "cloud" => TtsEngine::Cloud,
            _ => TtsEngine::Local,
        }
    }
}

enum SpeechCmd {
    Speak {
        text: String,
        voice: String,
        rate: u32,
    },
    SpeakAudio {
        chunks: Vec<Vec<u8>>,
        speed: f32,
    },
    ListVoices {
        reply: mpsc::Sender<Vec<VoiceInfo>>,
    },
}

pub struct SpeechService {
    tx: Mutex<mpsc::Sender<SpeechCmd>>,
    pub speaking: Arc<AtomicBool>,
    /// Shared with the speech thread so pause/resume/stop can act on the
    /// in-flight cloud sink directly, instead of queuing behind the thread's
    /// blocking wait loop (which would otherwise delay them until playback
    /// finishes on its own).
    active_sink: Arc<Mutex<Option<rodio::Sink>>>,
    /// Polled by the speech thread's wait loops so `stop()` can interrupt
    /// in-progress speech immediately rather than after it completes.
    stop_requested: Arc<AtomicBool>,
    app: AppHandle,
}

impl SpeechService {
    pub fn new(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel::<SpeechCmd>();
        let speaking = Arc::new(AtomicBool::new(false));
        let active_sink: Arc<Mutex<Option<rodio::Sink>>> = Arc::new(Mutex::new(None));
        let stop_requested = Arc::new(AtomicBool::new(false));

        let speaking_clone = speaking.clone();
        let active_sink_clone = active_sink.clone();
        let stop_requested_clone = stop_requested.clone();
        let app_clone = app.clone();

        std::thread::spawn(move || {
            run_speech_thread(
                rx,
                speaking_clone,
                active_sink_clone,
                stop_requested_clone,
                app_clone,
            );
        });

        Self {
            tx: Mutex::new(tx),
            speaking,
            active_sink,
            stop_requested,
            app,
        }
    }

    pub fn speak_local(&self, text: String, voice: String, rate: u32) {
        self.stop_requested.store(false, Ordering::SeqCst);
        let tx = self.tx.lock().unwrap();
        let _ = tx.send(SpeechCmd::Speak { text, voice, rate });
    }

    pub fn speak_audio(&self, chunks: Vec<Vec<u8>>, speed: f32) {
        self.stop_requested.store(false, Ordering::SeqCst);
        let tx = self.tx.lock().unwrap();
        let _ = tx.send(SpeechCmd::SpeakAudio { chunks, speed });
    }

    /// Pauses in-place. Only cloud audio (backed by a `rodio::Sink`) supports
    /// real pause/resume — the local OS-voice engine's crate exposes only
    /// speak/stop, so this is a no-op when speaking locally.
    pub fn pause(&self) {
        let lock = self.active_sink.lock().unwrap();
        if let Some(sink) = lock.as_ref() {
            sink.pause();
            let _ = self.app.emit("speaking-state", "paused");
        }
    }

    pub fn resume(&self) {
        let lock = self.active_sink.lock().unwrap();
        if let Some(sink) = lock.as_ref() {
            sink.play();
            let _ = self.app.emit("speaking-state", "speaking");
        }
    }

    /// Stops immediately: flips the flag the speech thread polls AND empties
    /// the active sink directly, so playback halts now rather than after the
    /// thread's wait loop next wakes up.
    pub fn stop(&self) {
        self.stop_requested.store(true, Ordering::SeqCst);
        let mut lock = self.active_sink.lock().unwrap();
        if let Some(sink) = lock.take() {
            sink.stop();
        }
    }

    pub fn list_local_voices(&self) -> Vec<VoiceInfo> {
        let (reply_tx, reply_rx) = mpsc::channel();
        {
            let tx = self.tx.lock().unwrap();
            let _ = tx.send(SpeechCmd::ListVoices { reply: reply_tx });
        }
        reply_rx.recv().unwrap_or_default()
    }
}

fn wpm_to_rate(tts_inst: &tts::Tts, wpm: u32) -> f32 {
    let min = tts_inst.min_rate();
    let norm = tts_inst.normal_rate();
    let max = tts_inst.max_rate();

    if wpm <= 175 {
        norm - (175u32.saturating_sub(wpm)) as f32 / 95.0 * (norm - min)
    } else {
        norm + (wpm - 175) as f32 / 225.0 * (max - norm)
    }
}

fn run_speech_thread(
    rx: mpsc::Receiver<SpeechCmd>,
    speaking: Arc<AtomicBool>,
    active_sink: Arc<Mutex<Option<rodio::Sink>>>,
    stop_requested: Arc<AtomicBool>,
    app: AppHandle,
) {
    let tts_result = tts::Tts::default();
    let mut tts_engine = match tts_result {
        Ok(t) => Some(t),
        Err(e) => {
            eprintln!("[Robin TTS] Failed to initialize TTS engine: {}", e);
            None
        }
    };

    // rodio output stream must stay alive for the duration of the thread
    let output = rodio::OutputStream::try_default();
    let (_stream, stream_handle) = match output {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("[Robin TTS] Failed to open audio output: {}", e);
            for cmd in rx {
                if let SpeechCmd::ListVoices { reply } = cmd {
                    let _ = reply.send(vec![]);
                }
            }
            return;
        }
    };

    for cmd in rx {
        match cmd {
            SpeechCmd::Speak { text, voice, rate } => {
                speaking.store(true, Ordering::SeqCst);
                let _ = app.emit("speaking-state", "speaking");

                if let Some(ref mut t) = tts_engine {
                    if !voice.is_empty() {
                        if let Ok(voices) = t.voices() {
                            if let Some(v) = voices.iter().find(|v| v.id() == voice) {
                                let _ = t.set_voice(v);
                            }
                        }
                    }

                    let target_rate = wpm_to_rate(t, rate).clamp(t.min_rate(), t.max_rate());
                    let _ = t.set_rate(target_rate);

                    match t.speak(&text, true) {
                        Ok(_) => loop {
                            if stop_requested.load(Ordering::SeqCst) {
                                let _ = t.stop();
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            match t.is_speaking() {
                                Ok(true) => {}
                                _ => break,
                            }
                        },
                        Err(e) => eprintln!("[Robin TTS] Speak error: {}", e),
                    }
                }

                speaking.store(false, Ordering::SeqCst);
                let _ = app.emit("speaking-state", "idle");
            }

            SpeechCmd::SpeakAudio { chunks, speed } => {
                speaking.store(true, Ordering::SeqCst);
                let _ = app.emit("speaking-state", "speaking");

                match rodio::Sink::try_new(&stream_handle) {
                    Ok(sink) => {
                        sink.set_speed(speed);
                        let mut queued = false;
                        for chunk in chunks {
                            match rodio::Decoder::new(Cursor::new(chunk)) {
                                Ok(source) => {
                                    sink.append(source);
                                    queued = true;
                                }
                                Err(e) => eprintln!("[Robin TTS] Decode error: {}", e),
                            }
                        }

                        if queued {
                            {
                                let mut lock = active_sink.lock().unwrap();
                                *lock = Some(sink);
                            }
                            loop {
                                if stop_requested.load(Ordering::SeqCst) {
                                    break;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                let done = {
                                    let lock = active_sink.lock().unwrap();
                                    lock.as_ref().map(|s| s.empty()).unwrap_or(true)
                                };
                                if done {
                                    break;
                                }
                            }
                            let mut lock = active_sink.lock().unwrap();
                            if let Some(s) = lock.take() {
                                s.stop();
                            }
                        }
                    }
                    Err(e) => eprintln!("[Robin TTS] Sink error: {}", e),
                }

                speaking.store(false, Ordering::SeqCst);
                let _ = app.emit("speaking-state", "idle");
            }

            SpeechCmd::ListVoices { reply } => {
                let voices = if let Some(ref mut t) = tts_engine {
                    t.voices()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|v| VoiceInfo {
                            id: v.id().to_string(),
                            name: v.name().to_string(),
                            engine: "local".to_string(),
                        })
                        .collect()
                } else {
                    vec![]
                };
                let _ = reply.send(voices);
            }
        }
    }
}

/// Toggle speak/stop. If currently speaking, stops. Otherwise captures selection and reads it.
pub async fn do_toggle_speak(app: &AppHandle) -> Result<(), String> {
    let app_state = app.state::<crate::app_state::AppState>();
    let speech = app.state::<SpeechService>();

    if speech.speaking.load(Ordering::SeqCst) {
        speech.stop();
        return Ok(());
    }

    let settings = app_state.settings.lock().unwrap().clone();
    if !settings.tts_enabled {
        return Ok(());
    }

    let text = robin_lib::paste::capture_selection()?;
    if text.trim().is_empty() {
        return Ok(());
    }

    speak(app, &speech, &settings, text).await
}

/// Synthesizes/plays `text` per the user's engine choice. Emits a `loading`
/// state immediately so the UI can show feedback during the network round
/// trip (cloud) or engine warm-up (local) instead of looking frozen.
async fn speak(
    app: &AppHandle,
    speech: &SpeechService,
    settings: &Settings,
    text: String,
) -> Result<(), String> {
    let _ = app.emit("speaking-state", "loading");

    match TtsEngine::from_settings(settings) {
        TtsEngine::Local => {
            speech.speak_local(
                text,
                settings.tts_voice_for_engine().to_string(),
                settings.tts_rate,
            );
        }
        TtsEngine::Cloud => {
            let speed = wpm_to_playback_speed(settings.tts_rate);
            let chunks_text = tts_groq::chunk_text(&text, tts_groq::ORPHEUS_MAX_CHARS);
            let api_key = settings.groq_api_key.clone();
            let voice = settings.tts_voice_for_engine().to_string();

            // Bounded concurrency keeps order (so playback isn't scrambled) while
            // limiting how hard we hit Groq's rate limits on long selections.
            let synthesis = futures_util::stream::iter(chunks_text.into_iter().map(|chunk| {
                let api_key = api_key.clone();
                let voice = voice.clone();
                async move { tts_groq::synthesize_groq(&api_key, &chunk, &voice).await }
            }))
            .buffered(CLOUD_SYNTHESIS_CONCURRENCY)
            .try_collect::<Vec<Vec<u8>>>()
            .await;

            match synthesis {
                Ok(chunks) => speech.speak_audio(chunks, speed),
                Err(e) => {
                    // Clear the "loading" state so the UI doesn't appear frozen,
                    // and signal the failure so it can surface feedback.
                    let _ = app.emit("speaking-state", "error");
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

/// Groq's Orpheus endpoint has no `speed` parameter, so the WPM setting must
/// be applied as a client-side playback-rate multiplier instead.
fn wpm_to_playback_speed(wpm: u32) -> f32 {
    (wpm as f32 / 175.0).clamp(0.5, 4.0)
}

pub fn list_voices(app: &AppHandle) -> Vec<VoiceInfo> {
    let app_state = app.state::<crate::app_state::AppState>();
    let speech = app.state::<SpeechService>();
    let settings = app_state.settings.lock().unwrap();
    match TtsEngine::from_settings(&settings) {
        TtsEngine::Local => speech.list_local_voices(),
        TtsEngine::Cloud => tts_groq::ORPHEUS_VOICES
            .iter()
            .map(|(id, name)| VoiceInfo {
                id: id.to_string(),
                name: name.to_string(),
                engine: "cloud".to_string(),
            })
            .collect(),
    }
}

#[tauri::command]
pub fn list_voices_cmd(app: AppHandle) -> Vec<VoiceInfo> {
    list_voices(&app)
}

#[tauri::command]
pub fn stop_speaking(app: AppHandle) {
    let speech = app.state::<SpeechService>();
    speech.stop();
}

/// Pauses in-place. Only the cloud engine (audio playback via `rodio::Sink`)
/// supports real pause/resume — calling this while speaking locally is a
/// harmless no-op, since the OS-voice engine has no pause primitive.
#[tauri::command]
pub fn pause_speaking(app: AppHandle) {
    let speech = app.state::<SpeechService>();
    speech.pause();
}

#[tauri::command]
pub fn resume_speaking(app: AppHandle) {
    let speech = app.state::<SpeechService>();
    speech.resume();
}

#[tauri::command]
pub async fn speak_text_cmd(app: AppHandle, text: String) -> Result<(), String> {
    let app_state = app.state::<crate::app_state::AppState>();
    let speech = app.state::<SpeechService>();
    let settings = app_state.settings.lock().unwrap().clone();

    speak(&app, &speech, &settings, text).await
}

#[tauri::command]
pub async fn update_tts_hotkey(
    app: AppHandle,
    state: tauri::State<'_, crate::app_state::AppState>,
    hotkey: String,
) -> Result<(), String> {
    let dictation_hotkey = state.settings.lock().unwrap().hotkey.clone();
    if hotkey == dictation_hotkey {
        return Err("TTS hotkey must be different from the dictation hotkey".to_string());
    }
    crate::hotkey::register_all_hotkeys(&app, &dictation_hotkey, &hotkey)?;
    let mut settings = state.settings.lock().unwrap();
    settings.tts_hotkey = hotkey;
    settings.save(&state.app_dir)?;
    Ok(())
}
