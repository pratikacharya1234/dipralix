#![allow(dead_code)]

use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputTokenLimit")]
    pub input_token_limit: Option<u32>,
    #[serde(rename = "outputTokenLimit")]
    pub output_token_limit: Option<u32>,
    #[serde(rename = "supportedGenerationMethods")]
    pub supported_methods: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct ModelListResponse {
    models: Vec<ModelInfo>,
}

/// Query the Gemini API for available models. Returns the full list.
pub async fn fetch_available_models(api_key: &str) -> Result<Vec<ModelInfo>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let body = resp.text().await?;

    let parsed: ModelListResponse = serde_json::from_str(&body)?;

    Ok(parsed.models)
}

/// Filter to coding-relevant models only.
pub fn filter_coding_models(models: &[ModelInfo]) -> Vec<ModelInfo> {
    let code_relevant: Vec<&str> = vec![
        "gemini-3.1",
        "gemini-3",
        "gemini-2.5-pro",
        "gemini-2.5-flash",
        "gemini-2.0-flash",
    ];

    // Junk display names from Gemini API (internal codenames)
    let skip_displays: Vec<&str> = vec!["nano banana", "nano", "banana"];

    // Non-coding model types that may advertise generateContent but don't work for chat
    let skip_keywords: Vec<&str> = vec![
        "-tts",
        "tts-",
        "-embedding",
        "-imagen",
        "-veo",
        "-lite-preview",
    ];

    let mut filtered: Vec<ModelInfo> = models
        .iter()
        .filter(|m| {
            let name = m.name.to_lowercase();
            let display = m.display_name.as_deref().unwrap_or("").to_lowercase();
            let has_gemini = name.contains("gemini");
            let is_chat_model = m
                .supported_methods
                .as_ref()
                .map(|m| m.iter().any(|s| s == "generateContent"))
                .unwrap_or(false);
            let is_relevant = code_relevant.iter().any(|p| name.contains(p));
            let is_latest = !name.contains("1.0") && !name.contains("1.5");
            let has_junk_display = skip_displays.iter().any(|j| display.contains(*j));
            let is_non_coding = skip_keywords.iter().any(|k| name.contains(*k));
            has_gemini
                && is_chat_model
                && is_relevant
                && is_latest
                && !has_junk_display
                && !is_non_coding
        })
        .cloned()
        .collect();

    if filtered.is_empty() {
        filtered = models
            .iter()
            .filter(|m| {
                let display = m.display_name.as_deref().unwrap_or("").to_lowercase();
                let has_junk = skip_displays.iter().any(|j| display.contains(*j));
                m.supported_methods
                    .as_ref()
                    .map(|m| m.iter().any(|s| s == "generateContent"))
                    .unwrap_or(false)
                    && !has_junk
            })
            .cloned()
            .collect();
    }

    filtered.sort_by(|a, b| b.name.cmp(&a.name));
    filtered.dedup_by(|a, b| a.name == b.name);
    filtered
}

/// Fetch Claude models from the Anthropic API.
/// Returns `(model_id, context_description)` pairs filtered to generation models.
pub async fn fetch_anthropic_models(api_key: &str) -> Result<Vec<(String, String)>> {
    #[derive(serde::Deserialize)]
    struct AnthropicModel {
        id: String,
        display_name: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct AnthropicListResponse {
        data: Vec<AnthropicModel>,
    }

    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?;

    let body = resp.text().await?;
    let parsed: AnthropicListResponse = serde_json::from_str(&body)?;

    let skip = ["claude-2", "claude-instant", "claude-1"];
    let mut out: Vec<(String, String)> = parsed
        .data
        .into_iter()
        .filter(|m| !skip.iter().any(|s| m.id.contains(s)))
        .map(|m| {
            let ctx = if m.id.contains("claude-4") || m.id.contains("claude-3") {
                "200K ctx".to_string()
            } else {
                m.display_name.unwrap_or_default()
            };
            (m.id, ctx)
        })
        .collect();

    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.dedup_by(|a, b| a.0 == b.0);
    Ok(out)
}

/// Fetch GPT/O-series models from the OpenAI API.
/// Returns `(model_id, context_description)` pairs filtered to relevant chat models.
pub async fn fetch_openai_models(api_key: &str) -> Result<Vec<(String, String)>> {
    #[derive(serde::Deserialize)]
    struct OpenAIModel {
        id: String,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIListResponse {
        data: Vec<OpenAIModel>,
    }

    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let body = resp.text().await?;
    let parsed: OpenAIListResponse = serde_json::from_str(&body)?;

    let relevant = ["gpt-4", "gpt-4o", "gpt-4.1", "o1", "o3", "o4"];
    let skip = [
        "instruct",
        "vision",
        "realtime",
        "audio",
        "search",
        "mini-2024",
        "preview-2024",
        "0125",
        "0613",
        "1106",
    ];
    let mut out: Vec<(String, String)> = parsed
        .data
        .into_iter()
        .filter(|m| {
            relevant.iter().any(|p| m.id.starts_with(p)) && !skip.iter().any(|s| m.id.contains(s))
        })
        .map(|m| {
            let ctx = if m.id.contains("gpt-4.1") {
                "1M ctx".to_string()
            } else if m.id.contains("o3") || m.id.contains("o4") || m.id.contains("o1") {
                "200K ctx".to_string()
            } else {
                "128K ctx".to_string()
            };
            (m.id, ctx)
        })
        .collect();

    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.dedup_by(|a, b| a.0 == b.0);
    Ok(out)
}

/// Get the latest available model matching a preference.
pub fn pick_best_model(models: &[ModelInfo], preferred: &str) -> String {
    let find = |name: &str| -> Option<String> {
        models
            .iter()
            .find(|m| m.name.to_lowercase().contains(&name.to_lowercase()))
            .map(|m| m.name.clone())
    };

    if !preferred.is_empty() {
        if let Some(m) = find(preferred) {
            return m;
        }
    }

    find("gemini-2.5-flash-latest")
        .or_else(|| find("gemini-2.5-flash"))
        .or_else(|| find("gemini-2.5-pro"))
        .or_else(|| find("gemini-2.0-flash"))
        .unwrap_or_else(|| "gemini-2.5-flash".to_string())
}

/// Detect the actual model version from a model name prefix.
/// E.g., "gemini-2.5-flash" might resolve to "models/gemini-2.5-flash-preview-05-20"
pub fn resolve_model_name(fetched: &[ModelInfo], requested: &str) -> String {
    let normalized = requested
        .trim_start_matches("models/")
        .trim_start_matches("tunedModels/");

    let exact = fetched
        .iter()
        .find(|m| m.name == normalized || m.name == format!("models/{}", normalized));

    if let Some(m) = exact {
        return m.name.clone();
    }

    let prefix_match = fetched
        .iter()
        .filter(|m| m.name.contains(normalized))
        .max_by_key(|m| m.name.len());

    if let Some(m) = prefix_match {
        return m.name.clone();
    }

    requested.to_string()
}

/// Auto-detect the best available Gemini model from the API response.
/// Prefers gemini-2.5-flash variants, falls back to latest available.
pub fn resolve_best_model(fetched: &[ModelInfo]) -> String {
    let coding = filter_coding_models(fetched);

    // Priority order for default model selection
    let preferences = [
        "gemini-3.1-flash",
        "gemini-3-flash",
        "gemini-2.5-flash",
        "gemini-3.1-pro",
        "gemini-3-pro",
        "gemini-2.5-pro",
        "gemini-2.0-flash",
    ];

    for pref in &preferences {
        if let Some(m) = coding.iter().find(|m| m.name.contains(pref)) {
            return m.name.trim_start_matches("models/").to_string();
        }
    }

    // No preferred model found — use the first coding model available
    if let Some(m) = coding.first() {
        return m.name.trim_start_matches("models/").to_string();
    }

    // Nothing found at all — return the API default
    "gemini-2.5-flash".to_string()
}

/// Pick the best Claude model from the API response.
pub fn resolve_best_anthropic(models: &[(String, String)]) -> String {
    let preferences = [
        "claude-sonnet-4",
        "claude-4-sonnet",
        "claude-4-opus",
        "claude-3-5-sonnet",
        "claude-3-opus",
    ];
    for pref in &preferences {
        for (id, _) in models {
            if id.contains(pref) {
                return id.clone();
            }
        }
    }
    models
        .first()
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string())
}

/// Pick the best OpenAI model from the API response.
pub fn resolve_best_openai(models: &[(String, String)]) -> String {
    let preferences = ["gpt-4.1", "gpt-4o", "o3", "o4-mini", "gpt-4"];
    for pref in &preferences {
        for (id, _) in models {
            if id.starts_with(pref) {
                return id.clone();
            }
        }
    }
    models
        .first()
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| "gpt-4o".to_string())
}

/// Pick a fallback model when the current one fails (rate limit, auth error, etc).
/// Returns (model_name, api_key). Tries: same-provider alternative → different provider.
pub fn pick_fallback_model(current: &str, config: &crate::config::Config) -> (String, String) {
    let gemini_key = std::env::var("GEMINI_API_KEY")
        .or_else(|_| std::env::var("DIPRALIX_API_KEY"))
        .unwrap_or_else(|_| config.api_key.clone());
    let anthropic_key: String = std::env::var("ANTHROPIC_API_KEY")
        .unwrap_or_else(|_| config.anthropic_api_key.clone().unwrap_or_default());
    let openai_key: String = std::env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| config.openai_api_key.clone().unwrap_or_default());

    let current_lower = current.to_lowercase();
    let is_gemini = current_lower.contains("gemini");
    let is_claude = current_lower.contains("claude");
    let is_gpt = current_lower.contains("gpt")
        || current_lower.contains("o3")
        || current_lower.contains("o4");

    // Same-provider fallback first
    if is_gemini && !gemini_key.is_empty() {
        if current_lower.contains("pro") {
            return ("gemini-2.5-flash".into(), gemini_key);
        } else {
            return ("gemini-2.0-flash".into(), gemini_key);
        }
    }

    if is_claude && !anthropic_key.is_empty() {
        return ("claude-sonnet-4-20250514".into(), anthropic_key);
    }

    if is_gpt && !openai_key.is_empty() {
        return ("gpt-4.1".into(), openai_key);
    }

    // Cross-provider fallback: prefer Gemini (free) > Claude > GPT
    if !gemini_key.is_empty() && !is_gemini {
        return ("gemini-2.5-flash".into(), gemini_key);
    }
    if !anthropic_key.is_empty() && !is_claude {
        return ("claude-sonnet-4-20250514".into(), anthropic_key);
    }
    if !openai_key.is_empty() && !is_gpt {
        return ("gpt-4.1".into(), openai_key);
    }

    // No fallback available — return current (will fail)
    (current.to_string(), gemini_key)
}
