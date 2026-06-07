use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::tools::{ToolContext, ToolResult};
use crate::types::FunctionDeclaration;

pub mod discord;
pub mod github;
pub mod google;
pub mod notion;
pub mod slack;

// ── Integration Registry ─────────────────────────────────────────────────────

/// Holds all active integrations and routes tool calls.
pub struct IntegrationRegistry {
    services: HashMap<String, Box<dyn IntegrationService>>,
    tool_map: HashMap<String, String>, // tool_name -> service_name
}

pub trait IntegrationService: Send + Sync {
    fn name(&self) -> &str;
    fn tool_declarations(&self) -> Vec<FunctionDeclaration>;
    fn call_tool(&self, tool_name: &str, args: Value) -> ToolResult;
}

impl IntegrationRegistry {
    pub fn new() -> Self {
        IntegrationRegistry {
            services: HashMap::new(),
            tool_map: HashMap::new(),
        }
    }

    pub fn register(&mut self, service: Box<dyn IntegrationService>) {
        let name = service.name().to_string();
        for decl in service.tool_declarations() {
            let tool_name = format!("{}__{}", name, decl.name);
            self.tool_map.insert(tool_name, name.clone());
        }
        self.services.insert(name, service);
    }

    pub fn function_declarations(&self) -> Vec<FunctionDeclaration> {
        let mut out = Vec::new();
        for svc in self.services.values() {
            let prefix = svc.name().to_string();
            for decl in svc.tool_declarations() {
                out.push(FunctionDeclaration {
                    name: format!("{}__{}", prefix, decl.name),
                    description: decl.description,
                    parameters: decl.parameters,
                });
            }
        }
        out
    }

    pub fn call_tool(&self, full_name: &str, args: Value, _ctx: &ToolContext) -> ToolResult {
        let service_name = match self.tool_map.get(full_name) {
            Some(s) => s,
            None => return ToolResult::err(format!("Integration tool not found: {}", full_name)),
        };

        let service = match self.services.get(service_name) {
            Some(s) => s,
            None => {
                return ToolResult::err(format!("Integration service not found: {}", service_name))
            }
        };

        let prefix = format!("{}__", service_name);
        let tool_name = full_name.strip_prefix(&prefix).unwrap_or(full_name);
        service.call_tool(tool_name, args)
    }

    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    #[allow(dead_code)]
    pub fn tool_count(&self) -> usize {
        self.tool_map.len()
    }

    pub fn print_status(&self) {
        use colored::Colorize;
        for svc in self.services.values() {
            let count = svc.tool_declarations().len();
            println!("  + integration '{}' — {} tools", svc.name().cyan(), count);
        }
    }

    pub fn from_config(config: &IntegrationsConfig) -> Self {
        let mut reg = IntegrationRegistry::new();

        if let Some(ref gh_cfg) = config.github {
            if !gh_cfg.token.is_empty() {
                reg.register(Box::new(github::GithubIntegration::new(gh_cfg)));
            }
        }

        if let Some(ref dc_cfg) = config.discord {
            if !dc_cfg.bot_token.is_empty() {
                reg.register(Box::new(discord::DiscordIntegration::new(dc_cfg)));
            }
        }

        if let Some(ref g_cfg) = config.google {
            if !g_cfg.client_id.is_empty() || !g_cfg.access_token.is_empty() {
                if g_cfg.gdrive_enabled {
                    reg.register(Box::new(google::GDriveIntegration::new(g_cfg)));
                }
                if g_cfg.gmail_enabled {
                    reg.register(Box::new(google::GmailIntegration::new(g_cfg)));
                }
            }
        }

        if let Some(ref s_cfg) = config.slack {
            if !s_cfg.bot_token.is_empty() {
                reg.register(Box::new(slack::SlackIntegration::new(s_cfg)));
            }
        }

        if let Some(ref n_cfg) = config.notion {
            if !n_cfg.token.is_empty() {
                reg.register(Box::new(notion::NotionIntegration::new(n_cfg)));
            }
        }

        reg
    }
}

// ── Integration Config ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, Deserialize)]
pub struct IntegrationsConfig {
    #[serde(default)]
    pub github: Option<GithubConfig>,
    #[serde(default)]
    pub discord: Option<DiscordConfig>,
    #[serde(default)]
    pub google: Option<GoogleConfig>,
    #[serde(default)]
    pub slack: Option<SlackConfig>,
    #[serde(default)]
    pub notion: Option<NotionConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct GithubConfig {
    pub token: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SlackConfig {
    pub bot_token: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct NotionConfig {
    pub token: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub gdrive_enabled: bool,
    #[serde(default)]
    pub gmail_enabled: bool,
}

impl IntegrationsConfig {
    #[allow(dead_code)]
    pub fn has_any(&self) -> bool {
        self.github.is_some()
            || self.discord.is_some()
            || self.google.is_some()
            || self.slack.is_some()
            || self.notion.is_some()
    }
}
