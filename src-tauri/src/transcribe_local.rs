use std::path::PathBuf;

/// Run Whisper inference in-process via whisper-rs (no external sidecar binary).
/// The function is async so it fits the engine dispatch interface; the heavy
/// CPU work is offloaded to a dedicated OS thread via `spawn_blocking` so the
/// Tokio runtime is never blocked.
pub async fn transcribe_local(
    model_path: &PathBuf,
    audio_path: &PathBuf,
) -> Result<String, String> {
    if !model_path.exists() {
        return Err(format!(
            "Whisper model not found at {:?}. Please download a model first.",
            model_path
        ));
    }

    let model_path = model_path.clone();
    let audio_path = audio_path.clone();

    tokio::task::spawn_blocking(move || run_whisper(&model_path, &audio_path))
        .await
        .map_err(|e| format!("Whisper inference task panicked: {e}"))?
}

fn run_whisper(model_path: &PathBuf, audio_path: &PathBuf) -> Result<String, String> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    let samples = read_wav_f32(audio_path)?;

    let ctx = WhisperContext::new_with_params(
        model_path.to_str().ok_or("Invalid model path encoding")?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| format!("Failed to load Whisper model: {e}"))?;

    let mut state = ctx
        .create_state()
        .map_err(|e| format!("Failed to create Whisper state: {e}"))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_progress(false);

    state
        .full(params, &samples)
        .map_err(|e| format!("Whisper inference failed: {e}"))?;

    let n_segments = state
        .full_n_segments()
        .map_err(|e| format!("Failed to get segment count: {e}"))?;

    let mut text = String::new();
    for i in 0..n_segments {
        match state.full_get_segment_text(i) {
            Ok(seg) => text.push_str(&seg),
            Err(e) => eprintln!("[Robin] Warning: segment {i} error: {e}"),
        }
    }

    println!("[Robin] Whisper output: {}", text.trim());
    Ok(text.trim().to_string())
}

/// Read a 16-bit PCM WAV (as written by `dsp::write_wav`) back into f32
/// samples in [-1.0, 1.0], which is the format whisper-rs expects.
fn read_wav_f32(path: &PathBuf) -> Result<Vec<f32>, String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|e| format!("Failed to open WAV: {e}"))?;

    let scale = 1.0_f32 / i16::MAX as f32;
    reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 * scale).map_err(|e| e.to_string()))
        .collect::<Result<Vec<f32>, _>>()
        .map_err(|e| format!("Failed to decode WAV samples: {e}"))
}

// ── Path utilities used by the downloader and settings UI ────────────────────

pub fn model_filename(model_size: &str) -> String {
    format!("ggml-{}.bin", model_size)
}

pub fn model_download_url(model_size: &str) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        model_size
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_filename() {
        assert_eq!(model_filename("small"), "ggml-small.bin");
        assert_eq!(model_filename("medium"), "ggml-medium.bin");
    }

    #[test]
    fn test_model_download_url() {
        assert_eq!(
            model_download_url("small"),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
        );
    }
}
