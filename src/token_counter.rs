#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;

// ── Token counting via Gemini API ────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Serialize)]
struct CountTokensRequest {
    contents: Vec<CountContent>,
}

#[derive(Serialize)]
struct CountContent {
    parts: Vec<CountPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

#[derive(Serialize)]
struct CountPart {
    text: String,
}

#[derive(Deserialize)]
struct CountTokensResponse {
    #[serde(rename = "totalTokens")]
    total_tokens: Option<u32>,
    #[serde(rename = "totalBillableCharacters")]
    total_billable_chars: Option<u32>,
}

pub async fn count_tokens(text: &str, config: &Config) -> Result<u32> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:countTokens?key={}",
        config.model, config.api_key
    );

    let request = CountTokensRequest {
        contents: vec![CountContent {
            role: Some("user".to_string()),
            parts: vec![CountPart {
                text: text.to_string(),
            }],
        }],
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .context("Token count API request failed")?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .context("Failed to read token count response")?;

    if !status.is_success() {
        anyhow::bail!(
            "Token count API HTTP {}: {}",
            status.as_u16(),
            trunc_str(&body, 300)
        );
    }

    let parsed: CountTokensResponse =
        serde_json::from_str(&body).context("Failed to parse token count response")?;

    Ok(parsed.total_tokens.unwrap_or(0))
}

pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as f64 * 0.25) as u32
}

// ── Cost Tracking ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ModelPricing {
    pub input_price_per_mtok: f64,
    pub output_price_per_mtok: f64,
    pub thinking_price_per_mtok: f64,
    pub context_window: u32,
}

pub fn pricing_for_model(model: &str) -> ModelPricing {
    let m = model.to_lowercase();

    // ── Claude (Anthropic) ────────────────────────────────────────────────────
    if m.contains("claude-4-opus") || m.contains("claude-opus-4") {
        return ModelPricing {
            input_price_per_mtok: 15.0,
            output_price_per_mtok: 75.0,
            thinking_price_per_mtok: 15.0,
            context_window: 200_000,
        };
    }
    if m.contains("claude-4-sonnet") || m.contains("claude-sonnet-4") {
        return ModelPricing {
            input_price_per_mtok: 3.0,
            output_price_per_mtok: 15.0,
            thinking_price_per_mtok: 3.0,
            context_window: 200_000,
        };
    }
    if m.contains("claude-3.5-sonnet") || m.contains("claude-3-5-sonnet") {
        return ModelPricing {
            input_price_per_mtok: 3.0,
            output_price_per_mtok: 15.0,
            thinking_price_per_mtok: 0.0,
            context_window: 200_000,
        };
    }
    if m.contains("claude-3.5-haiku") || m.contains("claude-3-5-haiku") {
        return ModelPricing {
            input_price_per_mtok: 0.80,
            output_price_per_mtok: 4.0,
            thinking_price_per_mtok: 0.0,
            context_window: 200_000,
        };
    }
    if m.contains("claude") {
        return ModelPricing {
            input_price_per_mtok: 3.0,
            output_price_per_mtok: 15.0,
            thinking_price_per_mtok: 3.0,
            context_window: 200_000,
        };
    }

    // ── OpenAI ────────────────────────────────────────────────────────────────
    if m.contains("gpt-4.1-mini") {
        return ModelPricing {
            input_price_per_mtok: 0.40,
            output_price_per_mtok: 1.60,
            thinking_price_per_mtok: 0.0,
            context_window: 1_000_000,
        };
    }
    if m.contains("gpt-4.1") {
        return ModelPricing {
            input_price_per_mtok: 2.0,
            output_price_per_mtok: 8.0,
            thinking_price_per_mtok: 0.0,
            context_window: 1_000_000,
        };
    }
    if m.starts_with("o4-mini") {
        return ModelPricing {
            input_price_per_mtok: 1.10,
            output_price_per_mtok: 4.40,
            thinking_price_per_mtok: 1.10,
            context_window: 200_000,
        };
    }
    if m.starts_with("o3-mini") {
        return ModelPricing {
            input_price_per_mtok: 1.10,
            output_price_per_mtok: 4.40,
            thinking_price_per_mtok: 1.10,
            context_window: 200_000,
        };
    }
    if m.starts_with("o3") {
        return ModelPricing {
            input_price_per_mtok: 10.0,
            output_price_per_mtok: 40.0,
            thinking_price_per_mtok: 10.0,
            context_window: 200_000,
        };
    }
    if m.starts_with("o4") {
        return ModelPricing {
            input_price_per_mtok: 10.0,
            output_price_per_mtok: 40.0,
            thinking_price_per_mtok: 10.0,
            context_window: 200_000,
        };
    }
    if m.contains("gpt-4o-mini") {
        return ModelPricing {
            input_price_per_mtok: 0.15,
            output_price_per_mtok: 0.60,
            thinking_price_per_mtok: 0.0,
            context_window: 128_000,
        };
    }
    if m.contains("gpt-4o") {
        return ModelPricing {
            input_price_per_mtok: 2.50,
            output_price_per_mtok: 10.0,
            thinking_price_per_mtok: 0.0,
            context_window: 128_000,
        };
    }
    if m.contains("gpt-4") {
        return ModelPricing {
            input_price_per_mtok: 30.0,
            output_price_per_mtok: 60.0,
            thinking_price_per_mtok: 0.0,
            context_window: 128_000,
        };
    }

    // ── Gemini ────────────────────────────────────────────────────────────────
    if m.contains("2.5-pro") {
        return ModelPricing {
            input_price_per_mtok: 1.25,
            output_price_per_mtok: 10.0,
            thinking_price_per_mtok: 1.25,
            context_window: 1_000_000,
        };
    }
    if m.contains("2.5-flash-lite") {
        return ModelPricing {
            input_price_per_mtok: 0.10,
            output_price_per_mtok: 0.40,
            thinking_price_per_mtok: 0.0,
            context_window: 1_000_000,
        };
    }
    if m.contains("2.5-flash") || m.contains("2.5") {
        return ModelPricing {
            input_price_per_mtok: 0.15,
            output_price_per_mtok: 0.60,
            thinking_price_per_mtok: 0.15,
            context_window: 1_000_000,
        };
    }
    if m.contains("2.0-flash-lite") {
        return ModelPricing {
            input_price_per_mtok: 0.075,
            output_price_per_mtok: 0.30,
            thinking_price_per_mtok: 0.0,
            context_window: 1_000_000,
        };
    }
    if m.contains("2.0-flash") || m.contains("2.0") {
        return ModelPricing {
            input_price_per_mtok: 0.10,
            output_price_per_mtok: 0.40,
            thinking_price_per_mtok: 0.0,
            context_window: 1_000_000,
        };
    }

    // Default (Gemini-family fallback)
    ModelPricing {
        input_price_per_mtok: 0.15,
        output_price_per_mtok: 0.60,
        thinking_price_per_mtok: 0.15,
        context_window: 1_000_000,
    }
}

#[derive(Clone, Debug)]
pub struct CostTracker {
    pub session_input_tokens: u64,
    pub session_output_tokens: u64,
    pub session_thinking_tokens: u64,
    pub turn_count: u64,
    pub model: String,
    budget_usd: Option<f64>,
    budget_used: f64,
    pricing: ModelPricing,
    last_estimate_tick: u32,
}

impl CostTracker {
    pub fn new(model: &str, budget_usd: Option<f64>) -> Self {
        CostTracker {
            session_input_tokens: 0,
            session_output_tokens: 0,
            session_thinking_tokens: 0,
            turn_count: 0,
            model: model.to_string(),
            budget_usd,
            budget_used: 0.0,
            pricing: pricing_for_model(model),
            last_estimate_tick: 0,
        }
    }

    pub fn record_usage(
        &mut self,
        prompt_tokens: u32,
        completion_tokens: u32,
        thinking_tokens: u32,
    ) {
        self.session_input_tokens += prompt_tokens as u64;
        self.session_output_tokens += completion_tokens as u64;
        self.session_thinking_tokens += thinking_tokens as u64;
        self.turn_count += 1;

        let cost = (prompt_tokens as f64 / 1_000_000.0) * self.pricing.input_price_per_mtok
            + (completion_tokens as f64 / 1_000_000.0) * self.pricing.output_price_per_mtok
            + (thinking_tokens as f64 / 1_000_000.0) * self.pricing.thinking_price_per_mtok;

        self.budget_used += cost;
    }

    pub fn estimate_context_tokens(&self) -> u32 {
        self.last_estimate_tick
    }

    pub fn set_estimate_tick(&mut self, tokens: u32) {
        self.last_estimate_tick = tokens;
    }

    pub fn session_cost(&self) -> f64 {
        self.budget_used
    }

    pub fn total_tokens(&self) -> u64 {
        self.session_input_tokens + self.session_output_tokens + self.session_thinking_tokens
    }

    pub fn format_status(&self) -> String {
        let is_gemini_free = self.model.to_lowercase().contains("gemini");
        let pct = if let Some(budget) = self.budget_usd {
            if budget > 0.0 {
                format!(" ({:.0}%)", (self.budget_used / budget) * 100.0)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let cost_str = if is_gemini_free && self.budget_used < 0.0001 {
            "FREE (Gemini tier)".to_string()
        } else {
            format!("${:.4}", self.budget_used)
        };

        format!(
            "Session: {}{}  Input: {}K  Output: {}K  Thinking: {}K  Turns: {}",
            cost_str,
            pct,
            self.session_input_tokens / 1000,
            self.session_output_tokens / 1000,
            self.session_thinking_tokens / 1000,
            self.turn_count
        )
    }

    pub fn budget_warning(&self) -> Option<String> {
        if let Some(budget) = self.budget_usd {
            if budget > 0.0 && self.budget_used >= budget * 0.80 {
                Some(format!(
                    "Budget alert: ${:.4} of ${:.2} used ({:.0}%)",
                    self.budget_used,
                    budget,
                    (self.budget_used / budget) * 100.0
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn trunc_str(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        None => s.to_string(),
        Some((idx, _)) => format!("{}...", &s[..idx]),
    }
}
