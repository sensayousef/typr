// Voices available for Groq's canopylabs/orpheus-v1-english TTS model
// (PlayAI's playai-tts was decommissioned by Groq; Orpheus is its replacement).
pub const ORPHEUS_VOICES: &[(&str, &str)] = &[
    ("autumn", "Autumn"),
    ("diana", "Diana"),
    ("hannah", "Hannah"),
    ("austin", "Austin"),
    ("daniel", "Daniel"),
    ("troy", "Troy"),
];

/// Orpheus rejects (or silently mishandles) input over this length, and it
/// accepts no `speed` parameter — playback rate must be applied client-side.
pub const ORPHEUS_MAX_CHARS: usize = 200;

pub async fn synthesize_groq(api_key: &str, text: &str, voice: &str) -> Result<Vec<u8>, String> {
    if api_key.is_empty() {
        return Err("Groq API key not set. Please enter your API key in settings.".to_string());
    }

    let voice = if voice.is_empty() { "autumn" } else { voice };

    let body = serde_json::json!({
        "model": "canopylabs/orpheus-v1-english",
        "input": text,
        "voice": voice,
        "response_format": "wav",
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post("https://api.groq.com/openai/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Groq TTS request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(format!("Groq TTS error {}: {}", status, err_body));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read TTS audio: {}", e))?;

    let mut bytes = bytes.to_vec();
    fix_streamed_wav_header(&mut bytes);
    Ok(bytes)
}

/// Splits `text` into pieces no longer than `max_len` characters so each one
/// fits within Orpheus's per-request limit, preferring sentence boundaries
/// (then word, then raw character boundaries) so speech doesn't fragment
/// mid-thought any more than it has to.
pub fn chunk_text(text: &str, max_len: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for sentence in split_sentences(text) {
        if sentence.chars().count() > max_len {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            chunks.extend(split_by_words(&sentence, max_len));
            continue;
        }

        let candidate_len = current.chars().count()
            + if current.is_empty() { 0 } else { 1 }
            + sentence.chars().count();
        if candidate_len > max_len {
            chunks.push(std::mem::take(&mut current));
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&sentence);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Splits on `.`/`!`/`?` followed by whitespace (or end of text), keeping the
/// terminating punctuation attached to its sentence.
fn split_sentences(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut sentences = Vec::new();
    let mut current = String::new();

    for (i, &c) in chars.iter().enumerate() {
        current.push(c);
        let ends_sentence = matches!(c, '.' | '!' | '?')
            && chars.get(i + 1).map_or(true, |next| next.is_whitespace());
        if ends_sentence {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                sentences.push(trimmed.to_string());
            }
            current.clear();
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        sentences.push(trimmed.to_string());
    }

    sentences
}

/// Greedily packs whitespace-separated words into chunks no longer than
/// `max_len`, for sentences that exceed the limit on their own.
fn split_by_words(sentence: &str, max_len: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for word in sentence.split_whitespace() {
        if word.chars().count() > max_len {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            chunks.extend(split_by_chars(word, max_len));
            continue;
        }

        let candidate_len = current.chars().count()
            + if current.is_empty() { 0 } else { 1 }
            + word.chars().count();
        if candidate_len > max_len {
            chunks.push(std::mem::take(&mut current));
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Last-resort hard split for single words longer than `max_len` (e.g. URLs).
fn split_by_chars(word: &str, max_len: usize) -> Vec<String> {
    word.chars()
        .collect::<Vec<_>>()
        .chunks(max_len)
        .map(|c| c.iter().collect())
        .collect()
}

/// Groq streams the TTS response and doesn't know the final length up front,
/// so it writes placeholder `0xFFFFFFFF` sizes for the RIFF and `data` chunks.
/// rodio's WAV decoder (via hound) treats those as literal sizes and fails to
/// play the audio, so patch them to the real buffer length first.
fn fix_streamed_wav_header(bytes: &mut [u8]) {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return;
    }

    let riff_size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());

    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let declared_size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;

        if chunk_id == b"data" {
            let data_size = (bytes.len() - (pos + 8)) as u32;
            bytes[pos + 4..pos + 8].copy_from_slice(&data_size.to_le_bytes());
            return;
        }

        // Chunks are padded to an even number of bytes per the RIFF spec.
        pos += 8 + declared_size + (declared_size % 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_empty_vec_for_blank_input() {
        assert_eq!(chunk_text("   ", 200), Vec::<String>::new());
    }

    #[test]
    fn keeps_short_text_as_a_single_chunk() {
        let chunks = chunk_text("Hello there. How are you?", 200);
        assert_eq!(chunks, vec!["Hello there. How are you?".to_string()]);
    }

    #[test]
    fn splits_long_text_on_sentence_boundaries_within_limit() {
        let text = "Sentence one is here. Sentence two is here. Sentence three is here. Sentence four is here.";
        let chunks = chunk_text(text, 50);

        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.chars().count() <= 50, "chunk too long: {chunk:?}");
        }
        // Rejoining preserves every word in order.
        let rejoined = chunks.join(" ");
        assert_eq!(
            rejoined.split_whitespace().collect::<Vec<_>>(),
            text.split_whitespace().collect::<Vec<_>>()
        );
    }

    #[test]
    fn falls_back_to_word_boundaries_for_oversized_sentences() {
        let text = "This single sentence has no punctuation anywhere so it just keeps going and going past the limit";
        let chunks = chunk_text(text, 30);

        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.chars().count() <= 30, "chunk too long: {chunk:?}");
        }
    }

    #[test]
    fn falls_back_to_character_splitting_for_oversized_words() {
        let text = "https://example.com/a/very/long/url/that/has/no/spaces/in/it/whatsoever";
        let chunks = chunk_text(text, 20);

        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.chars().count() <= 20, "chunk too long: {chunk:?}");
        }
        assert_eq!(chunks.concat(), text);
    }

    #[test]
    fn patches_placeholder_riff_and_data_sizes_to_actual_length() {
        // RIFF/WAVE header + fmt chunk + LIST/INFO chunk + data chunk,
        // mirroring Groq's streamed response shape with placeholder sizes.
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&u32::MAX.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&[0u8; 16]);
        wav.extend_from_slice(b"LIST");
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(b"INFO");
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&u32::MAX.to_le_bytes());
        let pcm_samples = [1u8, 2, 3, 4, 5, 6, 7, 8];
        wav.extend_from_slice(&pcm_samples);

        fix_streamed_wav_header(&mut wav);

        let riff_size = u32::from_le_bytes(wav[4..8].try_into().unwrap());
        assert_eq!(riff_size as usize, wav.len() - 8);

        let data_chunk_pos = wav.len() - 8 - pcm_samples.len();
        let data_size = u32::from_le_bytes(
            wav[data_chunk_pos + 4..data_chunk_pos + 8].try_into().unwrap(),
        );
        assert_eq!(data_size as usize, pcm_samples.len());
    }

    #[test]
    fn leaves_non_wav_bytes_untouched() {
        let mut bytes = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let original = bytes.clone();
        fix_streamed_wav_header(&mut bytes);
        assert_eq!(bytes, original);
    }
}
