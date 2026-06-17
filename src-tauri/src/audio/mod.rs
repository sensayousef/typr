pub mod dsp;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct MicDevice {
    pub name: String,
    pub is_default: bool,
}

/// Raw audio samples returned by `AudioCapture::stop`.
pub struct CapturedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn list_microphones() -> Vec<MicDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(MicDevice {
                    is_default: name == default_name,
                    name,
                });
            }
        }
    }
    devices
}

/// SAFETY: cpal::Stream on macOS (CoreAudio) is thread-safe in practice;
/// on Windows WASAPI is also thread-safe. Access is always behind a Mutex.
struct SendStream(#[allow(dead_code)] cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

/// Manages the CPAL input stream and accumulates PCM samples.
pub struct AudioCapture {
    buffer: Arc<Mutex<Vec<f32>>>,
    stream: Option<SendStream>,
    sample_rate: u32,
    channels: u16,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate: 48_000,
            channels: 1,
        }
    }

    /// Start capturing audio. `on_level` is called with a normalised RMS value
    /// (0.0–1.0) roughly every 100 ms so the UI can animate a level meter.
    pub fn start<F>(&mut self, mic_name: &str, on_level: F) -> Result<(), String>
    where
        F: Fn(f32) + Send + 'static,
    {
        self.buffer.lock().unwrap().clear();

        let host = cpal::default_host();
        let device = if mic_name == "default" {
            host.default_input_device()
                .ok_or_else(|| "No default input device found".to_string())?
        } else {
            host.input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == mic_name).unwrap_or(false))
                .ok_or_else(|| format!("Microphone '{}' not found", mic_name))?
        };

        let default_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels();
        println!("[Typr] Mic config: {}Hz, {} channels", sample_rate, channels);

        self.sample_rate = sample_rate;
        self.channels = channels;

        let config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = self.buffer.clone();
        // Emit level at most ~10 times per second (every sample_rate/10 samples).
        let level_interval = sample_rate / 10;
        let sample_counter = Arc::new(AtomicU32::new(0));

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    buffer.lock().unwrap().extend_from_slice(data);

                    let prev = sample_counter.fetch_add(data.len() as u32, Ordering::Relaxed);
                    if prev % level_interval < data.len() as u32 {
                        // Compute RMS and normalise to roughly 0..1 for speech.
                        let rms = if data.is_empty() {
                            0.0_f32
                        } else {
                            let sum: f32 = data.iter().map(|&s| s * s).sum();
                            (sum / data.len() as f32).sqrt()
                        };
                        on_level((rms * 8.0).min(1.0));
                    }
                },
                |err| eprintln!("[Typr] Audio stream error: {}", err),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(SendStream(stream));
        println!("[Typr] Audio recording started");
        Ok(())
    }

    /// Stops the stream and returns the captured audio. Clears the internal buffer.
    pub fn stop(&mut self) -> Result<CapturedAudio, String> {
        self.stream = None; // dropping the stream stops it
        println!("[Typr] Audio recording stopped");

        let mut buf = self.buffer.lock().unwrap();
        if buf.is_empty() {
            return Err("No audio captured".to_string());
        }
        println!("[Typr] Captured {} raw samples", buf.len());

        let samples = std::mem::take(&mut *buf);
        Ok(CapturedAudio {
            samples,
            sample_rate: self.sample_rate,
            channels: self.channels,
        })
    }
}
