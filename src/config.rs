use std::collections::HashMap;

use serde::Deserialize;

use crate::integrations::IntegrationsConfig;
use crate::mcp::McpServerConfig;

/// Runtime configuration, assembled from CLI flags + config file + defaults.
#[derive(Clone, Debug)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    pub grounding: bool,
    pub thinking: bool,
    pub thinking_budget: i32,
    pub auto_apply: bool,
    pub max_iterations: u32,
    pub context_warn: f32,
    pub context_compact: f32,
    pub mcp_servers: HashMap<String, McpServerConfig>,
    pub integrations: IntegrationsConfig,
    pub daily_budget_usd: Option<f64>,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub explain_before_execute: bool,
    #[allow(dead_code)]
    pub api_base: Option<String>,
    /// Domain preset — skips interactive selector when set.
    pub domain: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "gemini-2.5-flash".to_string(),
            grounding: false,
            thinking: false,
            thinking_budget: 8000,
            auto_apply: false,
            max_iterations: 50,
            context_warn: 0.75,
            context_compact: 0.90,
            mcp_servers: HashMap::new(),
            integrations: IntegrationsConfig::default(),
            daily_budget_usd: None,
            anthropic_api_key: None,
            openai_api_key: None,
            explain_before_execute: false,
            api_base: None,
            domain: None,
        }
    }
}

/// Context window sizes by model family (tokens).
pub fn context_window(model: &str) -> u32 {
    let m = model.to_lowercase();
    // Claude models
    if m.contains("claude") {
        return 200_000;
    }
    // OpenAI models
    if m.contains("gpt-4.1") {
        return 1_000_000;
    }
    if m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4") {
        return 200_000;
    }
    if m.contains("gpt-4o") || m.contains("gpt-4") {
        return 128_000;
    }
    if m.contains("gpt-3.5") {
        return 16_385;
    }
    // Gemini models (default 1M for 2.0+)
    1_000_000
}

// ── ~/.dipralix/config.toml ─────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
#[allow(dead_code)]
struct FileConfig {
    api_key: Option<String>,
    model: Option<String>,
    grounding: Option<bool>,
    auto_apply: Option<bool>,
    max_iterations: Option<u32>,
    context_warn: Option<f32>,
    context_compact: Option<f32>,

    #[serde(default)]
    thinking: ThinkingSection,

    #[serde(default)]
    mcp_servers: HashMap<String, McpServerConfig>,

    #[serde(default)]
    integrations: IntegrationsConfig,

    #[serde(default)]
    daily_budget_usd: Option<f64>,

    anthropic_api_key: Option<String>,
    openai_api_key: Option<String>,
    explain_before_execute: Option<bool>,
    api_base: Option<String>,

    #[serde(default)]
    profiles: HashMap<String, ProfileConfig>,
}

#[derive(Deserialize, Default)]
struct ThinkingSection {
    enabled: Option<bool>,
    budget: Option<i32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ProfileConfig {
    pub model: Option<String>,
    pub grounding: Option<bool>,
    pub thinking: Option<bool>,
    pub thinking_budget: Option<i32>,
    pub auto_apply: Option<bool>,
    pub daily_budget_usd: Option<f64>,
}

impl Config {
    /// Load default values from `~/.dipralix/config.toml`.
    /// Returns a partial Config — API key and model will be overridden by CLI flags.
    pub fn file_defaults() -> FileDefaults {
        let path = dirs::home_dir().map(|h| h.join(".dipralix").join("config.toml"));

        let Some(path) = path else {
            return FileDefaults::default();
        };
        let Ok(content) = std::fs::read_to_string(path) else {
            return FileDefaults::default();
        };
        let Ok(cfg): Result<FileConfig, _> = toml::from_str(&content) else {
            return FileDefaults::default();
        };

        FileDefaults {
            api_key: cfg.api_key,
            model: cfg.model,
            grounding: cfg.grounding.unwrap_or(false),
            thinking: cfg.thinking.enabled.unwrap_or(false),
            thinking_budget: cfg.thinking.budget.unwrap_or(8000),
            auto_apply: cfg.auto_apply.unwrap_or(false),
            max_iterations: cfg.max_iterations.unwrap_or(50),
            context_warn: cfg.context_warn.unwrap_or(0.75),
            context_compact: cfg.context_compact.unwrap_or(0.90),
            mcp_servers: cfg.mcp_servers,
            integrations: cfg.integrations,
            daily_budget_usd: cfg.daily_budget_usd,
            anthropic_api_key: cfg.anthropic_api_key,
            openai_api_key: cfg.openai_api_key,
            explain_before_execute: cfg.explain_before_execute.unwrap_or(false),
            api_base: cfg.api_base.clone(),
        }
    }
}

#[derive(Default)]
pub struct FileDefaults {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub grounding: bool,
    pub thinking: bool,
    pub thinking_budget: i32,
    pub auto_apply: bool,
    pub max_iterations: u32,
    pub context_warn: f32,
    pub context_compact: f32,
    pub mcp_servers: HashMap<String, McpServerConfig>,
    pub integrations: IntegrationsConfig,
    pub daily_budget_usd: Option<f64>,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub explain_before_execute: bool,
    #[allow(dead_code)]
    pub api_base: Option<String>,
}

pub fn load_profile(name: &str) -> Option<ProfileConfig> {
    let path = dirs::home_dir()?.join(".dipralix").join("config.toml");
    let content = std::fs::read_to_string(path).ok()?;
    #[derive(serde::Deserialize, Default)]
    struct ProfilesWrapper {
        #[serde(default)]
        profiles: std::collections::HashMap<String, ProfileConfig>,
    }
    let cfg: ProfilesWrapper = toml::from_str(&content).ok()?;
    cfg.profiles.get(name).cloned()
}
