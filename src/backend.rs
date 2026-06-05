/// Multi-provider backend abstraction.
///
/// Provides a unified interface across Gemini, Anthropic (Claude), and OpenAI (GPT).
/// Internally uses Gemini-native types (Content, Part, FunctionCall, FunctionResponse)
/// as the canonical representation, converting to/from each provider's native format.
use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::types::*;

// ── Provider enum ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Gemini,
    Anthropic,
    OpenAI,
    Ollama,
}

/// Detect provider from model name.
pub fn detect_provider(model: &str) -> Provider {
    let lower = model.to_lowercase();
    if lower.starts_with("gemini") || lower.contains("gemini") {
        Provider::Gemini
    } else if lower.starts_with("claude") || lower.contains("claude") {
        Provider::Anthropic
    } else if lower.starts_with("gpt")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
    {
        Provider::OpenAI
    } else if lower.contains("llama")
        || lower.contains("mistral")
        || lower.contains("codellama")
        || lower.contains("phi")
        || lower.contains("qwen")
        || lower.contains("deepseek")
        || lower.contains("gemma")
        || lower.contains("mixtral")
        || lower.contains("dolphin")
        || lower.contains("openhermes")
        || lower.contains("orca")
        || lower.contains("neural")
        || lower.contains("yi-")
        || lower.contains("falcon")
        || lower.contains("command-r")
    {
        Provider::Ollama
    } else {
        // Default: treat as Gemini (common models like gemini-2.5-flash)
        Provider::Gemini
    }
}

// ── BackendClient ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum BackendClient {
    Gemini(GeminiBackend),
    Anthropic(AnthropicBackend),
    OpenAI(OpenAIBackend),
    Ollama(OllamaBackend),
}

impl BackendClient {
    pub fn new(config: &Config) -> Result<Self> {
        match detect_provider(&config.model) {
            Provider::Gemini => Ok(Self::Gemini(GeminiBackend::new(config))),
            Provider::Anthropic => {
                let key = config.anthropic_api_key.as_deref().unwrap_or("");
                if key.is_empty() {
                    anyhow::bail!("Anthropic API key required for {}. Set ANTHROPIC_API_KEY or --anthropic-api-key", config.model);
                }
                Ok(Self::Anthropic(AnthropicBackend::new(key, &config.model)))
            }
            Provider::OpenAI => {
                let key = config.openai_api_key.as_deref().unwrap_or("");
                if key.is_empty() {
                    anyhow::bail!(
                        "OpenAI API key required for {}. Set OPENAI_API_KEY or --openai-api-key",
                        config.model
                    );
                }
                Ok(Self::OpenAI(OpenAIBackend::new(key, &config.model)))
            }
            Provider::Ollama => Ok(Self::Ollama(OllamaBackend::new(&config.model))),
        }
    }

    #[allow(dead_code)]
    pub fn provider(&self) -> Provider {
        match self {
            Self::Gemini(_) => Provider::Gemini,
            Self::Anthropic(_) => Provider::Anthropic,
            Self::OpenAI(_) => Provider::OpenAI,
            Self::Ollama(_) => Provider::Ollama,
        }
    }

    #[allow(dead_code)]
    pub fn model_name(&self) -> &str {
        match self {
            Self::Gemini(b) => &b.model,
            Self::Anthropic(b) => &b.model,
            Self::OpenAI(b) => &b.model,
            Self::Ollama(b) => &b.model,
        }
    }

    #[allow(dead_code)]
    pub fn supports_thinking(&self) -> bool {
        match self {
            Self::Gemini(_) => true,
            Self::Anthropic(b) => b.model.contains("opus") || b.model.contains("sonnet"),
            Self::OpenAI(_) => false, // GPT doesn't stream thinking tokens
            Self::Ollama(_) => false,
        }
    }

    pub async fn generate(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        match self {
            Self::Gemini(b) => b.generate(request).await,
            Self::Anthropic(b) => b.generate(request).await,
            Self::OpenAI(b) => b.generate(request).await,
            Self::Ollama(b) => b.generate(request).await,
        }
    }

    pub async fn generate_streaming(
        &self,
        request: &GenerateContentRequest,
        on_text: &mut impl FnMut(&str),
        on_thought: &mut impl FnMut(&str),
    ) -> Result<GenerateContentResponse> {
        match self {
            Self::Gemini(b) => b.generate_streaming(request, on_text, on_thought).await,
            Self::Anthropic(b) => b.generate_streaming(request, on_text, on_thought).await,
            Self::OpenAI(b) => b.generate_streaming(request, on_text, on_thought).await,
            Self::Ollama(b) => b.generate_streaming(request, on_text, on_thought).await,
        }
    }
}

// ── Gemini backend ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GeminiBackend {
    http: reqwest::Client,
    pub model: String,
    api_key: String,
    api_base: Option<String>,
}

impl GeminiBackend {
    pub fn new(config: &Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            api_base: config.api_base.clone(),
        }
    }

    fn base_url(&self) -> &str {
        self.api_base
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com")
    }

    pub async fn generate(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url(),
            self.model,
            self.api_key
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

    pub async fn generate_streaming(
        &self,
        request: &GenerateContentRequest,
        on_text: &mut impl FnMut(&str),
        on_thought: &mut impl FnMut(&str),
    ) -> Result<GenerateContentResponse> {
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url(),
            self.model,
            self.api_key
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

// ── Anthropic (Claude) backend ────────────────────────────────────────────────

#[derive(Clone)]
pub struct AnthropicBackend {
    http: reqwest::Client,
    pub model: String,
    api_key: String,
}

// Anthropic message role types
#[derive(Serialize, Deserialize, Debug)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    signature: Option<String>,
    #[serde(rename = "tool_use_id", skip_serializing_if = "Option::is_none")]
    tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Vec<AnthropicContent>>,
}

#[derive(Serialize, Debug)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Serialize, Debug)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Serialize, Debug)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<AnthropicTool>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<AnthropicThinking>,
}

#[derive(Serialize, Debug)]
struct AnthropicThinking {
    #[serde(rename = "type")]
    thinking_type: String,
    budget_tokens: u32,
}

#[derive(Deserialize, Debug)]
struct AnthropicUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct AnthropicResponse {
    #[allow(dead_code)]
    id: String,
    content: Vec<AnthropicContent>,
    #[allow(dead_code)]
    role: String,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

// Streaming event types
#[derive(Deserialize, Debug)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    content_block: Option<AnthropicContent>,
    #[serde(default)]
    delta: Option<AnthropicDelta>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
    #[serde(default)]
    #[allow(dead_code)]
    index: Option<usize>,
}

#[derive(Deserialize, Debug)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "partial_json")]
    partial_json: Option<String>,
    #[serde(default, rename = "thinking")]
    thinking_delta: Option<String>,
    #[allow(dead_code)]
    #[serde(default, rename = "signature")]
    signature_delta: Option<String>,
    #[serde(default, rename = "stop_reason")]
    stop_reason: Option<String>,
}

impl AnthropicBackend {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        }
    }

    fn convert_tools(tools: &[serde_json::Value]) -> Vec<AnthropicTool> {
        tools
            .iter()
            .filter_map(|tool| {
                let decls = tool.get("functionDeclarations")?.as_array()?;
                Some(
                    decls
                        .iter()
                        .filter_map(|decl| {
                            Some(AnthropicTool {
                                name: decl.get("name")?.as_str()?.to_string(),
                                description: decl.get("description")?.as_str()?.to_string(),
                                input_schema: decl.get("parameters")?.clone(),
                            })
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect()
    }

    fn extract_system(contents: &[Content]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system = None;
        let mut messages = Vec::new();
        for content in contents {
            let text: String = content
                .parts
                .iter()
                .filter_map(|p| {
                    if let Part::Text {
                        text,
                        thought: None | Some(false),
                        ..
                    } = p
                    {
                        if !text.trim().is_empty() {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            if content.role == "system"
                || (content.role == "user" && system.is_none() && messages.is_empty())
            {
                // First user message could be system
            }

            match content.role.as_str() {
                "system" => {
                    system = Some(text);
                }
                "user" => {
                    let mut anthro_content: Vec<AnthropicContent> = Vec::new();
                    // Text parts
                    let text_content: String = content
                        .parts
                        .iter()
                        .filter_map(|p| {
                            if let Part::Text {
                                text,
                                thought: None | Some(false),
                                ..
                            } = p
                            {
                                if !text.trim().is_empty() {
                                    Some(text.as_str())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !text_content.is_empty() {
                        anthro_content.push(AnthropicContent {
                            content_type: "text".into(),
                            text: Some(text_content),
                            name: None,
                            input: None,
                            id: None,
                            thinking: None,
                            signature: None,
                            tool_use_id: None,
                            content: None,
                        });
                    }
                    // Tool result parts
                    for part in &content.parts {
                        if let Part::FunctionResponse { function_response } = part {
                            let result_text = function_response
                                .response
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let tool_id = function_response
                                .id
                                .as_deref()
                                .unwrap_or(&function_response.name);
                            anthro_content.push(AnthropicContent {
                                content_type: "tool_result".into(),
                                tool_use_id: Some(tool_id.to_string()),
                                content: Some(vec![AnthropicContent {
                                    content_type: "text".into(),
                                    text: Some(result_text.to_string()),
                                    name: None,
                                    input: None,
                                    id: None,
                                    thinking: None,
                                    signature: None,
                                    tool_use_id: None,
                                    content: None,
                                }]),
                                text: None,
                                name: None,
                                input: None,
                                id: None,
                                thinking: None,
                                signature: None,
                            });
                        }
                    }
                    messages.push(AnthropicMessage {
                        role: "user".into(),
                        content: anthro_content,
                    });
                }
                "model" | "assistant" => {
                    let mut anthro_content: Vec<AnthropicContent> = Vec::new();
                    for part in &content.parts {
                        match part {
                            Part::Text {
                                text,
                                thought: Some(true),
                                ..
                            } => {
                                anthro_content.push(AnthropicContent {
                                    content_type: "thinking".into(),
                                    thinking: Some(text.clone()),
                                    signature: Some(String::new()),
                                    text: None,
                                    name: None,
                                    input: None,
                                    id: None,
                                    tool_use_id: None,
                                    content: None,
                                });
                            }
                            Part::Text { text, .. } if !text.trim().is_empty() => {
                                anthro_content.push(AnthropicContent {
                                    content_type: "text".into(),
                                    text: Some(text.clone()),
                                    name: None,
                                    input: None,
                                    id: None,
                                    thinking: None,
                                    signature: None,
                                    tool_use_id: None,
                                    content: None,
                                });
                            }
                            Part::FunctionCall { function_call, .. } => {
                                anthro_content.push(AnthropicContent {
                                    content_type: "tool_use".into(),
                                    id: Some(format!("toolu_{}", uuid::Uuid::new_v4())),
                                    name: Some(function_call.name.clone()),
                                    input: Some(function_call.args.clone()),
                                    text: None,
                                    thinking: None,
                                    signature: None,
                                    tool_use_id: None,
                                    content: None,
                                });
                            }
                            _ => {}
                        }
                    }
                    messages.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: anthro_content,
                    });
                }
                _ => {}
            }
        }
        (system, messages)
    }

    pub async fn generate(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let (system, messages) = Self::extract_system(&request.contents);
        let tools = Self::convert_tools(&request.tools);
        let max_tokens = request
            .generation_config
            .as_ref()
            .and_then(|g| g.max_output_tokens)
            .unwrap_or(8192);

        let mut anthro_request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens,
            messages,
            system,
            tools,
            stream: false,
            thinking: None,
        };

        // Enable extended thinking for models that support it
        if self.model.contains("opus") || self.model.contains("sonnet") {
            if let Some(ref gen_cfg) = request.generation_config {
                if let Some(ref tc) = gen_cfg.thinking_config {
                    if tc.thinking_budget > 0 {
                        anthro_request.thinking = Some(AnthropicThinking {
                            thinking_type: "enabled".into(),
                            budget_tokens: tc.thinking_budget.max(1024) as u32,
                        });
                        // Bump max_tokens to accommodate thinking
                        anthro_request.max_tokens = max_tokens.max(4096);
                    }
                }
            }
        }

        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&anthro_request)
            .send()
            .await
            .context("Network request to Anthropic API failed")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("Failed to read Anthropic response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Anthropic API HTTP {}: {}",
                status,
                &body[..body.len().min(400)]
            );
        }

        let parsed: AnthropicResponse =
            serde_json::from_str(&body).context("Failed to parse Anthropic response")?;

        Ok(Self::response_to_gemini(&parsed))
    }

    fn response_to_gemini(resp: &AnthropicResponse) -> GenerateContentResponse {
        let mut parts: Vec<Part> = Vec::new();
        for block in &resp.content {
            match block.content_type.as_str() {
                "text" => {
                    if let Some(ref t) = block.text {
                        if !t.trim().is_empty() {
                            parts.push(Part::text(t));
                        }
                    }
                }
                "thinking" => {
                    if let Some(ref t) = block.thinking {
                        parts.push(Part::Text {
                            text: t.clone(),
                            thought: Some(true),
                            thought_signature: None,
                        });
                    }
                }
                "tool_use" => {
                    parts.push(Part::FunctionCall {
                        function_call: FunctionCall {
                            name: block.name.clone().unwrap_or_default(),
                            args: block.input.clone().unwrap_or(serde_json::json!({})),
                            thought_signature: None,
                        },
                        thought_signature: None,
                    });
                }
                _ => {}
            }
        }

        let usage = resp.usage.as_ref().map(|u| UsageMetadata {
            prompt_token_count: u.input_tokens,
            candidates_token_count: u.output_tokens,
            total_token_count: Some(u.input_tokens.unwrap_or(0) + u.output_tokens.unwrap_or(0)),
            thoughts_token_count: None,
        });

        GenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    role: "assistant".into(),
                    parts,
                }),
                finish_reason: resp.stop_reason.clone(),
            }]),
            usage_metadata: usage,
            error: None,
        }
    }

    pub async fn generate_streaming(
        &self,
        request: &GenerateContentRequest,
        on_text: &mut impl FnMut(&str),
        on_thought: &mut impl FnMut(&str),
    ) -> Result<GenerateContentResponse> {
        let (system, messages) = Self::extract_system(&request.contents);
        let tools = Self::convert_tools(&request.tools);
        let max_tokens = request
            .generation_config
            .as_ref()
            .and_then(|g| g.max_output_tokens)
            .unwrap_or(8192);

        let mut anthro_request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens,
            messages,
            system,
            tools,
            stream: true,
            thinking: None,
        };

        if self.model.contains("opus") || self.model.contains("sonnet") {
            if let Some(ref gen_cfg) = request.generation_config {
                if let Some(ref tc) = gen_cfg.thinking_config {
                    if tc.thinking_budget > 0 {
                        anthro_request.thinking = Some(AnthropicThinking {
                            thinking_type: "enabled".into(),
                            budget_tokens: tc.thinking_budget.max(1024) as u32,
                        });
                        anthro_request.max_tokens = max_tokens.max(4096);
                    }
                }
            }
        }

        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&anthro_request)
            .send()
            .await
            .context("Streaming request to Anthropic API failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "Anthropic API HTTP {}: {}",
                status,
                &body[..body.len().min(400)]
            );
        }

        let mut stream = resp.bytes_stream();
        let mut line_buf = String::new();

        // Accumulators for streamed blocks
        let mut blocks: Vec<AnthropicContent> = Vec::new();
        let mut usage: Option<AnthropicUsage> = None;
        let mut stop_reason: Option<String> = None;
        let mut current_block_type = String::new();
        let mut current_text = String::new();
        let mut current_thinking = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_id = String::new();
        let mut current_tool_json = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(_) => break,
            };
            line_buf.push_str(&String::from_utf8_lossy(&bytes));

            loop {
                let event_end = match line_buf.find("\n\n") {
                    Some(pos) => pos,
                    None => break,
                };
                let event_text = line_buf[..event_end].to_string();
                line_buf = line_buf[event_end + 2..].to_string();

                // Parse SSE event lines
                let mut data_str = String::new();
                for line in event_text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        data_str = data.trim().to_string();
                    }
                }
                if data_str.is_empty() {
                    continue;
                }

                let event: AnthropicStreamEvent = match serde_json::from_str(&data_str) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                match event.event_type.as_str() {
                    "content_block_start" => {
                        if let Some(block) = event.content_block {
                            current_block_type = block.content_type.clone();
                            match current_block_type.as_str() {
                                "text" => {
                                    current_text.clear();
                                }
                                "thinking" => {
                                    current_thinking.clear();
                                }
                                "tool_use" => {
                                    current_tool_name = block.name.unwrap_or_default();
                                    current_tool_id = block.id.unwrap_or_default();
                                    current_tool_json.clear();
                                }
                                _ => {}
                            }
                        }
                    }
                    "content_block_delta" => {
                        if let Some(delta) = event.delta {
                            match delta.delta_type.as_deref() {
                                Some("text_delta") => {
                                    if let Some(ref t) = delta.text {
                                        on_text(t);
                                        current_text.push_str(t);
                                    }
                                }
                                Some("thinking_delta") => {
                                    if let Some(ref t) = delta.thinking_delta {
                                        on_thought(t);
                                        current_thinking.push_str(t);
                                    }
                                }
                                Some("input_json_delta") => {
                                    if let Some(ref j) = delta.partial_json {
                                        current_tool_json.push_str(j);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "content_block_stop" => match current_block_type.as_str() {
                        "text" => {
                            if !current_text.is_empty() {
                                blocks.push(AnthropicContent {
                                    content_type: "text".into(),
                                    text: Some(std::mem::take(&mut current_text)),
                                    name: None,
                                    input: None,
                                    id: None,
                                    thinking: None,
                                    signature: None,
                                    tool_use_id: None,
                                    content: None,
                                });
                            }
                        }
                        "thinking" => {
                            blocks.push(AnthropicContent {
                                content_type: "thinking".into(),
                                thinking: Some(std::mem::take(&mut current_thinking)),
                                signature: Some(String::new()),
                                text: None,
                                name: None,
                                input: None,
                                id: None,
                                tool_use_id: None,
                                content: None,
                            });
                        }
                        "tool_use" => {
                            let input: serde_json::Value = serde_json::from_str(&current_tool_json)
                                .unwrap_or(serde_json::json!({}));
                            blocks.push(AnthropicContent {
                                content_type: "tool_use".into(),
                                id: Some(std::mem::take(&mut current_tool_id)),
                                name: Some(std::mem::take(&mut current_tool_name)),
                                input: Some(input),
                                text: None,
                                thinking: None,
                                signature: None,
                                tool_use_id: None,
                                content: None,
                            });
                        }
                        _ => {}
                    },
                    "message_delta" => {
                        if let Some(delta) = event.delta {
                            stop_reason = delta.stop_reason;
                        }
                        if let Some(u) = event.usage {
                            usage = Some(u);
                        }
                    }
                    "message_stop" => {}
                    _ => {}
                }
            }
        }

        let mut parts: Vec<Part> = Vec::new();
        for block in &blocks {
            match block.content_type.as_str() {
                "text" => {
                    if let Some(ref t) = block.text {
                        if !t.trim().is_empty() {
                            parts.push(Part::text(t));
                        }
                    }
                }
                "thinking" => {
                    if let Some(ref t) = block.thinking {
                        parts.push(Part::Text {
                            text: t.clone(),
                            thought: Some(true),
                            thought_signature: None,
                        });
                    }
                }
                "tool_use" => {
                    let fc = FunctionCall {
                        name: block.name.clone().unwrap_or_default(),
                        args: block.input.clone().unwrap_or(serde_json::json!({})),
                        thought_signature: None,
                    };
                    parts.push(Part::FunctionCall {
                        function_call: fc,
                        thought_signature: None,
                    });
                }
                _ => {}
            }
        }

        let usage_meta = usage.map(|u| UsageMetadata {
            prompt_token_count: u.input_tokens,
            candidates_token_count: u.output_tokens,
            total_token_count: Some(u.input_tokens.unwrap_or(0) + u.output_tokens.unwrap_or(0)),
            thoughts_token_count: None,
        });

        Ok(GenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    role: "assistant".into(),
                    parts,
                }),
                finish_reason: stop_reason,
            }]),
            usage_metadata: usage_meta,
            error: None,
        })
    }
}

// ── OpenAI (GPT) backend ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct OpenAIBackend {
    http: reqwest::Client,
    pub model: String,
    api_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(rename = "tool_calls", skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(rename = "tool_call_id", skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct OpenAIToolCall {
    #[serde(default)]
    index: usize,
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAIFunction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Serialize, Debug)]
struct OpenAITool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIToolFunction,
}

#[derive(Serialize, Debug)]
struct OpenAIToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Debug)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OpenAITool>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(
        rename = "max_completion_tokens",
        skip_serializing_if = "Option::is_none"
    )]
    max_completion_tokens: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize, Debug)]
struct OpenAIChoice {
    message: OpenAIMessage,
    #[serde(rename = "finish_reason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// Streaming types
#[derive(Deserialize, Debug)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize, Debug)]
struct OpenAIStreamChoice {
    delta: OpenAIMessage,
    #[serde(rename = "finish_reason")]
    finish_reason: Option<String>,
}

impl OpenAIBackend {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        }
    }

    fn convert_tools(tools: &[serde_json::Value]) -> Vec<OpenAITool> {
        tools
            .iter()
            .filter_map(|tool| {
                let decls = tool.get("functionDeclarations")?.as_array()?;
                Some(
                    decls
                        .iter()
                        .map(|decl| OpenAITool {
                            tool_type: "function".into(),
                            function: OpenAIToolFunction {
                                name: decl
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .into(),
                                description: decl
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .into(),
                                parameters: decl
                                    .get("parameters")
                                    .cloned()
                                    .unwrap_or(serde_json::json!({})),
                            },
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect()
    }

    fn convert_messages(contents: &[Content]) -> Vec<OpenAIMessage> {
        let mut messages = Vec::new();
        for content in contents {
            match content.role.as_str() {
                "system" => {
                    let text: String = content
                        .parts
                        .iter()
                        .filter_map(|p| {
                            if let Part::Text {
                                text,
                                thought: None | Some(false),
                                ..
                            } = p
                            {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !text.is_empty() {
                        messages.push(OpenAIMessage {
                            role: "system".into(),
                            content: Some(text),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                }
                "user" => {
                    let text: String = content
                        .parts
                        .iter()
                        .filter_map(|p| {
                            match p {
                                Part::Text {
                                    text,
                                    thought: None | Some(false),
                                    ..
                                } if !text.trim().is_empty() => Some(text.as_str()),
                                Part::FunctionResponse { .. } => {
                                    // Tool results handled separately below as tool messages
                                    Some("__TOOL_RESULT__")
                                }
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    // For function responses, create separate tool messages
                    let mut has_tool_results = false;
                    for part in &content.parts {
                        if let Part::FunctionResponse { function_response } = part {
                            let result_text = function_response
                                .response
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let tool_id = function_response
                                .id
                                .as_deref()
                                .unwrap_or(&function_response.name);
                            messages.push(OpenAIMessage {
                                role: "tool".into(),
                                content: Some(result_text.to_string()),
                                tool_call_id: Some(tool_id.to_string()),
                                tool_calls: None,
                            });
                            has_tool_results = true;
                        }
                    }
                    if !has_tool_results && !text.is_empty() && text != "__TOOL_RESULT__" {
                        messages.push(OpenAIMessage {
                            role: "user".into(),
                            content: Some(text),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    } else if has_tool_results && !text.is_empty() && text != "__TOOL_RESULT__" {
                        messages.push(OpenAIMessage {
                            role: "user".into(),
                            content: Some(text),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                }
                "model" | "assistant" => {
                    let text: String = content
                        .parts
                        .iter()
                        .filter_map(|p| {
                            if let Part::Text {
                                text,
                                thought: None | Some(false),
                                ..
                            } = p
                            {
                                if !text.trim().is_empty() {
                                    Some(text.as_str())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    let tool_calls: Vec<OpenAIToolCall> = content
                        .parts
                        .iter()
                        .enumerate()
                        .filter_map(|(i, p)| {
                            if let Part::FunctionCall { function_call, .. } = p {
                                Some(OpenAIToolCall {
                                    index: i,
                                    id: format!("call_{}", uuid::Uuid::new_v4()),
                                    call_type: "function".into(),
                                    function: OpenAIFunction {
                                        name: function_call.name.clone(),
                                        arguments: function_call.args.to_string(),
                                    },
                                })
                            } else {
                                None
                            }
                        })
                        .collect();

                    messages.push(OpenAIMessage {
                        role: "assistant".into(),
                        content: if text.is_empty() { None } else { Some(text) },
                        tool_calls: if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        },
                        tool_call_id: None,
                    });
                }
                _ => {}
            }
        }
        messages
    }

    pub async fn generate(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let messages = Self::convert_messages(&request.contents);
        let tools = Self::convert_tools(&request.tools);
        let max_tokens = request
            .generation_config
            .as_ref()
            .and_then(|g| g.max_output_tokens)
            .unwrap_or(4096);
        let temperature = request
            .generation_config
            .as_ref()
            .and_then(|g| g.temperature);

        let oai_request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            tools,
            stream: false,
            temperature,
            max_completion_tokens: Some(max_tokens),
        };

        let resp = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&oai_request)
            .send()
            .await
            .context("Network request to OpenAI API failed")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("Failed to read OpenAI response")?;

        if !status.is_success() {
            anyhow::bail!(
                "OpenAI API HTTP {}: {}",
                status,
                &body[..body.len().min(400)]
            );
        }

        let parsed: OpenAIResponse =
            serde_json::from_str(&body).context("Failed to parse OpenAI response")?;

        Ok(Self::response_to_gemini(&parsed))
    }

    fn response_to_gemini(resp: &OpenAIResponse) -> GenerateContentResponse {
        let mut parts: Vec<Part> = Vec::new();

        if let Some(choice) = resp.choices.first() {
            if let Some(ref text) = choice.message.content {
                if !text.trim().is_empty() {
                    parts.push(Part::text(text));
                }
            }
            if let Some(ref tool_calls) = choice.message.tool_calls {
                for tc in tool_calls {
                    let args: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::json!({}));
                    parts.push(Part::FunctionCall {
                        function_call: FunctionCall {
                            name: tc.function.name.clone(),
                            args,
                            thought_signature: None,
                        },
                        thought_signature: None,
                    });
                }
            }
        }

        let usage = resp.usage.as_ref().map(|u| UsageMetadata {
            prompt_token_count: Some(u.prompt_tokens),
            candidates_token_count: Some(u.completion_tokens),
            total_token_count: Some(u.total_tokens),
            thoughts_token_count: None,
        });

        GenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    role: "assistant".into(),
                    parts,
                }),
                finish_reason: resp.choices.first().and_then(|c| c.finish_reason.clone()),
            }]),
            usage_metadata: usage,
            error: None,
        }
    }

    pub async fn generate_streaming(
        &self,
        request: &GenerateContentRequest,
        on_text: &mut impl FnMut(&str),
        on_thought: &mut impl FnMut(&str),
    ) -> Result<GenerateContentResponse> {
        let _ = on_thought; // OpenAI doesn't stream thinking tokens

        let messages = Self::convert_messages(&request.contents);
        let tools = Self::convert_tools(&request.tools);
        let max_tokens = request
            .generation_config
            .as_ref()
            .and_then(|g| g.max_output_tokens)
            .unwrap_or(4096);
        let temperature = request
            .generation_config
            .as_ref()
            .and_then(|g| g.temperature);

        let oai_request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            tools,
            stream: true,
            temperature,
            max_completion_tokens: Some(max_tokens),
        };

        let resp = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&oai_request)
            .send()
            .await
            .context("Streaming request to OpenAI API failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "OpenAI API HTTP {}: {}",
                status,
                &body[..body.len().min(400)]
            );
        }

        let mut stream = resp.bytes_stream();
        let mut line_buf = String::new();
        let mut all_text = String::new();
        let mut finish_reason: Option<String> = None;
        let mut usage: Option<OpenAIUsage> = None;

        // Accumulate tool calls across chunks
        let mut tool_calls: std::collections::BTreeMap<usize, OpenAIToolCall> =
            std::collections::BTreeMap::new();

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(_) => break,
            };
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

                        let chunk: OpenAIStreamChunk = match serde_json::from_str(json_str) {
                            Ok(c) => c,
                            Err(_) => continue,
                        };

                        if let Some(u) = chunk.usage {
                            usage = Some(u);
                        }

                        for choice in chunk.choices {
                            if let Some(fr) = choice.finish_reason {
                                finish_reason = Some(fr);
                            }
                            if let Some(ref text) = choice.delta.content {
                                on_text(text);
                                all_text.push_str(text);
                            }
                            if let Some(ref tcs) = choice.delta.tool_calls {
                                for tc in tcs {
                                    let entry = tool_calls.entry(tc.index).or_insert_with(|| {
                                        OpenAIToolCall {
                                            index: tc.index,
                                            id: tc.id.clone(),
                                            call_type: "function".into(),
                                            function: OpenAIFunction {
                                                name: String::new(),
                                                arguments: String::new(),
                                            },
                                        }
                                    });
                                    if !tc.id.is_empty() {
                                        entry.id = tc.id.clone();
                                    }
                                    if !tc.function.name.is_empty() {
                                        entry.function.name.push_str(&tc.function.name);
                                    }
                                    entry.function.arguments.push_str(&tc.function.arguments);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut parts: Vec<Part> = Vec::new();
        if !all_text.is_empty() {
            parts.push(Part::text(all_text));
        }
        for (_, tc) in tool_calls {
            let args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));
            parts.push(Part::FunctionCall {
                function_call: FunctionCall {
                    name: tc.function.name,
                    args,
                    thought_signature: None,
                },
                thought_signature: None,
            });
        }

        let usage_meta = usage.map(|u| UsageMetadata {
            prompt_token_count: Some(u.prompt_tokens),
            candidates_token_count: Some(u.completion_tokens),
            total_token_count: Some(u.total_tokens),
            thoughts_token_count: None,
        });

        Ok(GenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    role: "assistant".into(),
                    parts,
                }),
                finish_reason,
            }]),
            usage_metadata: usage_meta,
            error: None,
        })
    }
}

// ── Ollama backend (localhost:11434 OpenAI-compatible API) ────────────────────

#[derive(Clone)]
pub struct OllamaBackend {
    http: reqwest::Client,
    pub model: String,
    base_url: String,
}

impl OllamaBackend {
    pub fn new(model: &str) -> Self {
        let base_url =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
        Self {
            http: reqwest::Client::new(),
            model: model.to_string(),
            base_url,
        }
    }

    pub async fn generate(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        // Non-streaming fallback — just collects the stream
        let mut text = String::new();
        let mut on_text = |s: &str| {
            text.push_str(s);
        };
        let mut on_thought = |_: &str| {};
        self.generate_streaming(&request, &mut on_text, &mut on_thought)
            .await
    }

    fn convert_request(&self, request: &GenerateContentRequest) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // System instruction as first message
        if let Some(ref sys) = request.system_instruction {
            let sys_text: String = sys
                .parts
                .iter()
                .filter_map(|p| {
                    if let Part::Text { text, .. } = p {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !sys_text.is_empty() {
                messages.push(serde_json::json!({
                    "role": "system",
                    "content": sys_text
                }));
            }
        }

        for content in &request.contents {
            let role = match content.role.as_str() {
                "user" => "user",
                "model" | "assistant" => "assistant",
                _ => "user",
            };

            let mut text_parts: Vec<String> = Vec::new();
            let mut tool_calls: Vec<serde_json::Value> = Vec::new();
            let mut tool_results: Vec<serde_json::Value> = Vec::new();

            for part in &content.parts {
                match part {
                    Part::Text { text, .. } => text_parts.push(text.clone()),
                    Part::FunctionCall { function_call, .. } => {
                        tool_calls.push(serde_json::json!({
                            "id": format!("call_{}", tool_calls.len()),
                            "type": "function",
                            "function": {
                                "name": function_call.name,
                                "arguments": serde_json::to_string(&function_call.args).unwrap_or_default()
                            }
                        }));
                    }
                    Part::FunctionResponse { function_response } => {
                        let tc_id = function_response
                            .id
                            .as_deref()
                            .unwrap_or(&function_response.name);
                        let result_text = function_response
                            .response
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        tool_results.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tc_id,
                            "content": result_text
                        }));
                    }
                    _ => {}
                }
            }

            // Push text message
            if !text_parts.is_empty() || (tool_calls.is_empty() && tool_results.is_empty()) {
                let text = if text_parts.is_empty() {
                    String::new()
                } else {
                    text_parts.join("\n")
                };
                messages.push(serde_json::json!({
                    "role": role,
                    "content": text
                }));
            }

            // Push tool calls (as assistant message)
            if !tool_calls.is_empty() {
                messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": null,
                    "tool_calls": tool_calls
                }));
            }

            // Push tool results
            for tr in tool_results {
                messages.push(tr);
            }
        }

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true
        });

        // Add tools if present
        let mut openai_tools: Vec<serde_json::Value> = Vec::new();
        for t in &request.tools {
            if let Some(fds) = t
                .get("functionDeclarations")
                .and_then(|f: &serde_json::Value| f.as_array())
            {
                for fd in fds {
                    let name = fd.get("name").and_then(|v: &serde_json::Value| v.as_str());
                    let desc = fd
                        .get("description")
                        .and_then(|v: &serde_json::Value| v.as_str());
                    if let (Some(name), Some(desc)) = (name, desc) {
                        openai_tools.push(serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": name,
                                "description": desc,
                                "parameters": fd.get("parameters").cloned().unwrap_or(serde_json::json!({}))
                            }
                        }));
                    }
                }
            }
        }
        if !openai_tools.is_empty() {
            body["tools"] = serde_json::json!(openai_tools);
        }

        body
    }

    pub async fn generate_streaming(
        &self,
        request: &GenerateContentRequest,
        on_text: &mut impl FnMut(&str),
        _on_thought: &mut impl FnMut(&str),
    ) -> Result<GenerateContentResponse> {
        let body = self.convert_request(request);
        let url = format!("{}/v1/chat/completions", self.base_url);

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Ollama streaming request failed — is Ollama running? (ollama serve)")?;

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "Ollama API HTTP {}: {}",
                status,
                &err_body[..err_body.len().min(400)]
            );
        }

        let mut stream = resp.bytes_stream();
        let mut full_text = String::new();
        let mut tool_calls_acc: Vec<serde_json::Value> = Vec::new();
        let mut finish_reason: Option<String> = None;
        let mut prompt_tokens: u32 = 0;
        let mut completion_tokens: u32 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }
                let json_str = line.strip_prefix("data: ").unwrap_or(line);

                if let Ok(event) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(choices) = event.get("choices").and_then(|c| c.as_array()) {
                        for choice in choices {
                            // Text delta
                            if let Some(delta) = choice.get("delta") {
                                if let Some(content) = delta.get("content").and_then(|c| c.as_str())
                                {
                                    if !content.is_empty() {
                                        on_text(content);
                                        full_text.push_str(content);
                                    }
                                }
                                // Tool call delta
                                if let Some(tc_deltas) =
                                    delta.get("tool_calls").and_then(|t| t.as_array())
                                {
                                    for tc in tc_deltas {
                                        let idx =
                                            tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0)
                                                as usize;
                                        while tool_calls_acc.len() <= idx {
                                            tool_calls_acc.push(serde_json::json!({
                                                "id": "", "type": "function",
                                                "function": { "name": "", "arguments": "" }
                                            }));
                                        }
                                        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                            tool_calls_acc[idx]["id"] = serde_json::json!(id);
                                        }
                                        if let Some(func) = tc.get("function") {
                                            if let Some(name) =
                                                func.get("name").and_then(|n| n.as_str())
                                            {
                                                if !name.is_empty() {
                                                    tool_calls_acc[idx]["function"]["name"] =
                                                        serde_json::json!(name);
                                                }
                                            }
                                            if let Some(args) =
                                                func.get("arguments").and_then(|a| a.as_str())
                                            {
                                                let current = tool_calls_acc[idx]["function"]
                                                    ["arguments"]
                                                    .as_str()
                                                    .unwrap_or("");
                                                tool_calls_acc[idx]["function"]["arguments"] = serde_json::json!(
                                                    format!("{}{}", current, args)
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(fr) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                                finish_reason = Some(fr.to_string());
                            }
                        }
                    }
                    // Usage
                    if let Some(usage) = event.get("usage") {
                        prompt_tokens = usage
                            .get("prompt_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0) as u32;
                        completion_tokens = usage
                            .get("completion_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0) as u32;
                    }
                }
            }
        }

        // Build parts from response
        let mut parts: Vec<Part> = Vec::new();

        if !full_text.is_empty() {
            parts.push(Part::text(full_text));
        }

        for tc in &tool_calls_acc {
            let name = tc["function"]["name"].as_str().unwrap_or("");
            let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
            let args: serde_json::Value =
                serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
            if !name.is_empty() {
                parts.push(Part::FunctionCall {
                    function_call: FunctionCall {
                        name: name.to_string(),
                        args,
                        thought_signature: None,
                    },
                    thought_signature: None,
                });
            }
        }

        let usage_meta = if prompt_tokens > 0 || completion_tokens > 0 {
            Some(UsageMetadata {
                prompt_token_count: Some(prompt_tokens),
                candidates_token_count: Some(completion_tokens),
                total_token_count: Some(prompt_tokens + completion_tokens),
                thoughts_token_count: None,
            })
        } else {
            None
        };

        Ok(GenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    role: "assistant".into(),
                    parts,
                }),
                finish_reason,
            }]),
            usage_metadata: usage_meta,
            error: None,
        })
    }
}
