#![allow(dead_code)]
// Pure Rust voice capture via cpal. No external commands needed.
// Cross-platform mic recording → WAV encoding → Gemini transcription.
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// Check if a microphone is available.
pub fn check_audio() -> bool {
    cpal::default_host().default_input_device().is_some()
}

/// Record audio from default mic for `duration_secs`. Returns WAV bytes.
pub fn record_audio(duration_secs: u32) -> Result<Vec<u8>> {
    let host = cpal::default_host();
    let device = host.default_input_device().context("No microphone found")?;
    let config = device.default_input_config().context("No input config")?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as u16;
    let sample_format = config.sample_format();

    let samples_needed = (sample_rate * duration_secs) as usize;
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let recorded_clone = recorded.clone();
    let running = Arc::new(Mutex::new(true));
    let running_clone = running.clone();

    let stream = match sample_format {
        cpal::SampleFormat::I16 => {
            let err_fn = |e| eprintln!("  audio err: {}", e);
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    let mut buf = recorded_clone.lock().unwrap();
                    for s in data {
                        buf.extend_from_slice(&s.to_le_bytes());
                    }
                    if buf.len() / 2 >= samples_needed {
                        *running_clone.lock().unwrap() = false;
                    }
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::F32 => {
            let err_fn = |e| eprintln!("  audio err: {}", e);
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    let mut buf = recorded_clone.lock().unwrap();
                    for s in data {
                        let sample = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                        buf.extend_from_slice(&sample.to_le_bytes());
                    }
                    if buf.len() / 2 >= samples_needed {
                        *running_clone.lock().unwrap() = false;
                    }
                },
                err_fn,
                None,
            )?
        }
        _ => anyhow::bail!("Unsupported sample format: {:?}", sample_format),
    };

    stream.play()?;

    // Wait until enough samples
    while *running.lock().unwrap() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(stream);

    let pcm = recorded.lock().unwrap().clone();
    if pcm.is_empty() {
        anyhow::bail!("No audio captured — check microphone");
    }

    // Encode as WAV
    let mut wav_buf = Vec::new();
    {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(std::io::Cursor::new(&mut wav_buf), spec)?;
        for chunk in pcm.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }
    Ok(wav_buf)
}

/// Transcribe audio bytes using Gemini multimodal API. Returns text.
pub async fn transcribe_audio(audio_bytes: &[u8], api_key: &str) -> Result<String> {
    let audio_b64 = BASE64.encode(audio_bytes);
    let body = serde_json::json!({
        "contents": [{
            "parts": [
                { "text": "Transcribe this audio to text. Return ONLY the transcribed words. No explanation. No analysis. Just the words." },
                { "inlineData": { "mimeType": "audio/wav", "data": audio_b64 } }
            ]
        }],
        "generationConfig": {
            "temperature": 0.0,
            "maxOutputTokens": 256
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Transcription request failed")?;
    let status = resp.status();
    let body_text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!(
            "Transcription HTTP {}: {}",
            status,
            &body_text[..body_text.len().min(300)]
        );
    }
    let parsed: serde_json::Value = serde_json::from_str(&body_text)?;
    let text = parsed["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    // Filter out model rambling — if response is way too long or contains "thinking" patterns
    if text.len() > 500 || text.contains("1.") && text.contains("2.") && text.contains("3.") {
        // Model output reasoning instead of transcribing — try to extract just the words
        anyhow::bail!("Model rambled — retrying");
    }

    if text.is_empty() {
        anyhow::bail!("No transcription — speak more clearly");
    }
    Ok(text)
}

/// Record + transcribe, return text. For JARVIS loop.
pub async fn listen_and_transcribe(api_key: &str, duration_secs: u32) -> Result<String> {
    let audio = record_audio(duration_secs)?;
    transcribe_audio(&audio, api_key).await
}

/// One-shot voice prompt for --voice flag.
pub async fn voice_prompt(api_key: &str, duration_secs: u32) -> Result<String> {
    use colored::Colorize;
    println!();
    println!(
        "  {} Recording... speak now ({}s)",
        "◉".bright_red(),
        duration_secs
    );
    println!("  {}", "  (Press Ctrl+C to cancel)".dimmed());
    let audio = record_audio(duration_secs)?;
    println!(
        "  {} Recorded {:.1}KB — transcribing...",
        "⊕".green(),
        audio.len() as f64 / 1024.0
    );
    let text = transcribe_audio(&audio, api_key).await?;
    println!("  {} You said: {}", "⊢".cyan(), text.bright_white().bold());
    println!();
    Ok(text)
}
