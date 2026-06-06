use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::config::Config;

// ── Request types ──────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<serde_json::Value>,
    #[serde(rename = "toolConfig", skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<SystemContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

/// Gemini API parts.
///
/// Untagged — serde tries variants top-to-bottom:
///   FunctionCall      (has `functionCall`)
///   FunctionResponse  (has `functionResponse`)
///   InlineData        (has `inlineData`)
///   Text              (has `text` + optional `thought`)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum Part {
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
        /// Gemini 3+ thought signature on the part itself (sibling of functionCall).
        #[serde(
            default,
            rename = "thoughtSignature",
            skip_serializing_if = "Option::is_none"
        )]
        thought_signature: Option<String>,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
    Text {
        text: String,
        /// Present and `true` on Gemini thinking-mode reasoning chunks.
        #[serde(default)]
        thought: Option<bool>,
        /// Gemini 3+ thought signature on the part itself.
        #[serde(
            default,
            rename = "thoughtSignature",
            skip_serializing_if = "Option::is_none"
        )]
        thought_signature: Option<String>,
    },
}

impl Part {
    pub fn text(s: impl Into<String>) -> Self {
        Part::Text {
            text: s.into(),
            thought: None,
            thought_signature: None,
        }
    }

    pub fn image(mime_type: impl Into<String>, base64_data: impl Into<String>) -> Self {
        Part::InlineData {
            inline_data: InlineData {
                mime_type: mime_type.into(),
                data: base64_data.into(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
    /// Gemini 3 requires thought_signature on function call parts for tool use.
    /// Captured from model response and echoed back in conversation history.
    #[serde(
        default,
        rename = "thought_signature",
        skip_serializing_if = "Option::is_none"
    )]
    pub thought_signature: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
    /// Tool call ID for Anthropic (tool_use_id) / OpenAI (tool_call_id) round-trips.
    /// Populated when the backend provides an ID; used when building tool-result messages.
    #[serde(skip)]
    pub id: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolConfig {
    #[serde(rename = "functionCallingConfig")]
    pub function_calling_config: FunctionCallingConfig,
}

#[derive(Serialize, Clone, Debug)]
pub struct FunctionCallingConfig {
    pub mode: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct SystemContent {
    pub parts: Vec<Part>,
}

#[derive(Serialize, Clone, Debug)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(rename = "thinkingConfig", skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

/// ThinkMode configuration — gemini-2.5+ only.
#[derive(Serialize, Clone, Debug)]
pub struct ThinkingConfig {
    /// Token budget for reasoning: -1 = auto, 0 = disabled, 1-24576 = explicit.
    #[serde(rename = "thinkingBudget")]
    pub thinking_budget: i32,
    /// Whether to include reasoning chunks in the streaming response.
    #[serde(rename = "includeThoughts")]
    pub include_thoughts: bool,
}

// ── Response types ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct GenerateContentResponse {
    pub candidates: Option<Vec<Candidate>>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
    pub error: Option<ApiError>,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: Option<Content>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: Option<u32>,
    #[serde(rename = "thoughtsTokenCount")]
    pub thoughts_token_count: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct ApiError {
    pub code: Option<u32>,
    pub message: Option<String>,
}

// ── HTTP client ────────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct GeminiClient {
    http: reqwest::Client,
    config: Config,
}

#[allow(dead_code)]
impl GeminiClient {
    pub fn new(config: Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    /// Non-streaming — used for /compact and SecuritySweep.
    pub async fn generate(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.config.model, self.config.api_key
        );

        let resp = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Network request to Gemini API failed")?;

        let status = resp.status();
        let body = resp.text().await.context("Failed to read response body")?;

        let parsed: GenerateContentResponse = serde_json::from_str(&body).with_context(|| {
            format!(
                "Failed to parse Gemini response (HTTP {}): {}",
                status,
                &body[..body.len().min(400)]
            )
        })?;

        if let Some(ref err) = parsed.error {
            anyhow::bail!(
                "Gemini API error {}: {}",
                err.code.unwrap_or(0),
                err.message.as_deref().unwrap_or("unknown")
            );
        }
        if !status.is_success() {
            anyhow::bail!(
                "Gemini API HTTP {}: {}",
                status,
                &body[..body.len().min(400)]
            );
        }
        Ok(parsed)
    }

    /// Streaming generate.
    ///
    /// - `on_text`    — called with each regular text chunk as it arrives
    /// - `on_thought` — called with each ThinkMode reasoning chunk
    ///
    /// Returns the fully-accumulated `GenerateContentResponse` (including any
    /// function calls) after the stream ends.
    pub async fn generate_streaming(
        &self,
        request: &GenerateContentRequest,
        on_text: &mut impl FnMut(&str),
        on_thought: &mut impl FnMut(&str),
    ) -> Result<GenerateContentResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.config.model, self.config.api_key
        );

        let resp = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await
            .context("Streaming request to Gemini API failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "Gemini API HTTP {}: {}",
                status,
                &body[..body.len().min(400)]
            );
        }

        let mut stream = resp.bytes_stream();
        let mut line_buf = String::new();

        let mut all_text = String::new();
        let mut all_thought = String::new();
        let mut extra_parts: Vec<Part> = Vec::new();
        let mut usage: Option<UsageMetadata> = None;
        let mut finish_reason: Option<String> = None;
        let mut api_error: Option<ApiError> = None;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("Error reading streaming response")?;
            line_buf.push_str(&String::from_utf8_lossy(&bytes));

            loop {
                match line_buf.find('\n') {
                    None => break,
                    Some(pos) => {
                        let line = line_buf[..pos].trim_end_matches('\r').to_string();
                        line_buf = line_buf[pos + 1..].to_string();

                        if !line.starts_with("data: ") {
                            continue;
                        }
                        let json_str = &line[6..];
                        if json_str == "[DONE]" {
                            continue;
                        }

                        let chunk_resp: GenerateContentResponse =
                            match serde_json::from_str(json_str) {
                                Ok(r) => r,
                                Err(_) => continue,
                            };

                        if let Some(err) = chunk_resp.error {
                            api_error = Some(err);
                        }

                        if let Some(candidates) = chunk_resp.candidates {
                            for candidate in candidates {
                                if let Some(ref fr) = candidate.finish_reason {
                                    finish_reason = Some(fr.clone());
                                }
                                if let Some(content) = candidate.content {
                                    for part in content.parts {
                                        match part {
                                            Part::Text {
                                                ref text,
                                                thought: Some(true),
                                                ..
                                            } => {
                                                on_thought(text);
                                                all_thought.push_str(text);
                                            }
                                            Part::Text { ref text, .. } => {
                                                on_text(text);
                                                all_text.push_str(text);
                                            }
                                            other => extra_parts.push(other),
                                        }
                                    }
                                }
                            }
                        }

                        if chunk_resp.usage_metadata.is_some() {
                            usage = chunk_resp.usage_metadata;
                        }
                    }
                }
            }
        }

        if let Some(err) = api_error {
            anyhow::bail!(
                "Gemini API error {}: {}",
                err.code.unwrap_or(0),
                err.message.as_deref().unwrap_or("unknown")
            );
        }

        let mut final_parts: Vec<Part> = Vec::new();
        if !all_text.is_empty() {
            final_parts.push(Part::text(all_text));
        }
        final_parts.extend(extra_parts);

        Ok(GenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    role: "model".to_string(),
                    parts: final_parts,
                }),
                finish_reason,
            }]),
            usage_metadata: usage,
            error: None,
        })
    }
}
