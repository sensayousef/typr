use hound::{WavSpec, WavWriter};
use std::path::Path;

/// Convert multi-channel interleaved samples to mono by averaging channels.
pub fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Resample using linear interpolation.
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src = i as f64 * ratio;
        let idx = src as usize;
        let frac = src - idx as f64;
        let sample = if idx + 1 < samples.len() {
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else {
            *samples.get(idx).unwrap_or(&0.0) as f64
        };
        output.push(sample as f32);
    }
    output
}

/// Compute the root-mean-square energy of a sample buffer.
/// Returns 0.0 for an empty buffer. Useful for silence detection.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|&s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

/// Write normalized f32 mono samples as a 16-bit WAV file to `path`.
pub fn write_wav(samples: &[f32], sample_rate: u32, path: &Path) -> Result<(), String> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        let amp = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(amp).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_mono_passthrough_for_single_channel() {
        let samples = vec![0.1_f32, 0.2, 0.3];
        assert_eq!(to_mono(&samples, 1), samples);
    }

    #[test]
    fn to_mono_averages_stereo_frames() {
        // Stereo interleaved: [L0=0.4, R0=0.8, L1=0.2, R1=0.6]
        let samples = vec![0.4_f32, 0.8, 0.2, 0.6];
        let mono = to_mono(&samples, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.6).abs() < 1e-6, "frame 0 avg should be 0.6");
        assert!((mono[1] - 0.4).abs() < 1e-6, "frame 1 avg should be 0.4");
    }

    #[test]
    fn to_mono_empty_input_returns_empty() {
        assert_eq!(to_mono(&[], 2), Vec::<f32>::new());
    }

    #[test]
    fn resample_identity_when_rates_equal() {
        let samples = vec![0.1_f32, 0.2, 0.3];
        assert_eq!(resample(&samples, 16_000, 16_000), samples);
    }

    #[test]
    fn resample_halves_length_for_double_input_rate() {
        let samples: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let out = resample(&samples, 32_000, 16_000);
        // 100 samples at 32 kHz → ~50 samples at 16 kHz
        assert!((out.len() as i64 - 50).abs() <= 1);
    }

    #[test]
    fn resample_doubles_length_for_half_input_rate() {
        let samples: Vec<f32> = (0..50).map(|i| i as f32).collect();
        let out = resample(&samples, 8_000, 16_000);
        // 50 samples at 8 kHz → ~100 samples at 16 kHz
        assert!((out.len() as i64 - 100).abs() <= 2);
    }
}
