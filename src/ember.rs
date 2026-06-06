#![allow(dead_code)]
// EMBER — Lightweight real-time voice AI. Pure Rust.
// Google Gemini for transcription + Google Cloud TTS for speech.
// Local memory, zero keyboard, hands-free operation.
//
// No banner. No emoji. NULLVOID-clean.
// Caller (main.rs) handles mode selection before we fire.

use anyhow::Result;
use base64::Engine;

/// Run EMBER voice assistant loop.
/// Assumes caller already confirmed voice mode — no mode selector here.
pub async fn ember_loop(config: &crate::config::Config) -> Result<()> {
    let has_mic = crate::voice::check_audio();
    if !has_mic {
        crate::ui::nullvoid::print_warning("No mic detected. Voice mode unavailable.");
        return Ok(());
    }

    // Greet — nullvoid style, no emoji
    let proj = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default();
    let greeting = format!("EMBER online. Working on {}. What do you need?", proj);
    speak(&greeting, &config.api_key).await;
    crate::ui::nullvoid::thin_rule_stdout();
    crate::ui::nullvoid::print_info(&greeting);
    crate::ui::nullvoid::print_info("Speak now — listening. Say \"quit\" to stop.");
    crate::ui::nullvoid::thin_rule_stdout();
    println!();

    // Memory
    let mem_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".forge")
        .join("ember-memory.md");
    let mut memory: Vec<String> = if mem_path.exists() {
        std::fs::read_to_string(&mem_path)
            .unwrap_or_default()
            .lines()
            .map(|l| l.to_string())
            .filter(|l| !l.trim().is_empty())
            .collect()
    } else {
        Vec::new()
    };

    loop {
        let user_msg = match crate::voice::listen_and_transcribe(&config.api_key, 3).await {
            Ok(t) => t,
            Err(_) => continue,
        };
        if user_msg.is_empty() {
            continue;
        }

        println!(
            " {}{} {}{} {}",
            crate::ui::nullvoid::PLASMA,
            crate::ui::nullvoid::I_PROMPT,
            crate::ui::nullvoid::BRIGHT,
            user_msg,
            crate::ui::nullvoid::RESET
        );

        let lower = user_msg.to_lowercase();
        if lower.contains("quit") || lower.contains("exit") || lower.contains("goodbye") {
            let bye = "Shutting down. Goodbye.";
            speak(bye, &config.api_key).await;
            crate::ui::nullvoid::print_info(bye);
            save_mem(&mem_path, &memory);
            return Ok(());
        }

        memory.push(format!("You: {}", user_msg));
        let ctx = if memory.len() > 2 {
            let r: String = memory
                .iter()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .iter()
                .rev()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            format!("You are EMBER, concise voice AI for FORGE. 1-2 sentences max. Recent:\n{}\n\nUser: {}", r, user_msg)
        } else {
            format!("You are EMBER. Concise. 1-2 sentences.\nUser: {}", user_msg)
        };

        let response = match crate::agent::run_jarvis_query(config, &ctx).await {
            Ok(t) => t,
            Err(e) => format!("Error: {}", e),
        };
        memory.push(format!("EMBER: {}", response));

        speak(&response, &config.api_key).await;
        crate::ui::nullvoid::print_info(&response);
        println!();
    }
}

/// Speak via Google Cloud TTS (same API key as Gemini). Lightweight, always works.
async fn speak(text: &str, api_key: &str) {
    let clean: String = text.chars().filter(|c| c.is_ascii()).collect();
    if clean.len() < 2 {
        return;
    }

    let body = serde_json::json!({
        "input": { "text": clean },
        "voice": { "languageCode": "en-US", "name": "en-US-Journey-O" },
        "audioConfig": { "audioEncoding": "LINEAR16", "speakingRate": 1.1 }
    });

    let url = format!(
        "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
        api_key
    );

    if let Ok(resp) = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        if let Ok(parsed) = resp.json::<serde_json::Value>().await {
            if let Some(b64) = parsed["audioContent"].as_str() {
                if let Ok(audio) = base64::engine::general_purpose::STANDARD.decode(b64) {
                    // Write WAV and play via paplay
                    let tmp = std::env::temp_dir().join("ember_tts.wav");
                    let _ = std::fs::write(&tmp, &audio);
                    let _ = std::process::Command::new("paplay")
                        .arg(&tmp)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                }
            }
        }
    }
}

fn save_mem(path: &std::path::Path, memory: &[String]) {
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::fs::write(path, memory.join("\n"));
}

/// Check if mic is available for EMBER.
pub fn mic_available() -> bool {
    crate::voice::check_audio()
}
