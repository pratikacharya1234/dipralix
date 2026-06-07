use serde_json::{json, Value};

use crate::integrations::IntegrationService;
use crate::integrations::SlackConfig;
use crate::tools::ToolResult;
use crate::types::FunctionDeclaration;

/// Slack integration — posts messages and lists channels via a bot token.
pub struct SlackIntegration {
    bot_token: String,
    client: reqwest::Client,
}

impl SlackIntegration {
    pub fn new(config: &SlackConfig) -> Self {
        SlackIntegration {
            bot_token: config.bot_token.clone(),
            client: reqwest::Client::new(),
        }
    }

    async fn call(&self, method: &str, body: Option<&Value>) -> Result<Value, String> {
        let url = format!("https://slack.com/api/{method}");
        let req = match body {
            Some(b) => self.client.post(url).json(b),
            None => self.client.get(url),
        };
        let resp = req
            .bearer_auth(&self.bot_token)
            .send()
            .await
            .map_err(|e| format!("Slack API request failed: {e}"))?;
        let v: Value = resp
            .json()
            .await
            .map_err(|e| format!("Slack response parse failed: {e}"))?;
        if v.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(format!(
                "Slack API error: {}",
                v.get("error").and_then(Value::as_str).unwrap_or("unknown")
            ));
        }
        Ok(v)
    }
}

impl IntegrationService for SlackIntegration {
    fn name(&self) -> &str {
        "slack"
    }

    fn tool_declarations(&self) -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "send_message".to_string(),
                description: "Post a message to a Slack channel (by channel ID).".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "channel": { "type": "STRING", "description": "Channel ID" },
                        "text":    { "type": "STRING", "description": "Message text" }
                    },
                    "required": ["channel", "text"]
                }),
            },
            FunctionDeclaration {
                name: "list_channels".to_string(),
                description: "List Slack channels the bot can see.".to_string(),
                parameters: json!({ "type": "OBJECT", "properties": {} }),
            },
        ]
    }

    fn call_tool(&self, tool_name: &str, args: Value) -> ToolResult {
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => return ToolResult::err("No async runtime available for Slack API call"),
        };

        match tool_name {
            "send_message" => {
                let channel = match args.get("channel").and_then(Value::as_str) {
                    Some(c) => c,
                    None => return ToolResult::err("Missing required argument: channel"),
                };
                let text = match args.get("text").and_then(Value::as_str) {
                    Some(t) => t,
                    None => return ToolResult::err("Missing required argument: text"),
                };
                let payload = json!({ "channel": channel, "text": text });
                match rt.block_on(self.call("chat.postMessage", Some(&payload))) {
                    Ok(_) => ToolResult::ok(format!("Message sent to {channel}")),
                    Err(e) => ToolResult::err(e),
                }
            }
            "list_channels" => match rt.block_on(self.call("conversations.list?limit=200", None)) {
                Ok(v) => {
                    let names: Vec<String> = v
                        .get("channels")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(|c| {
                                    c.get("name")
                                        .and_then(Value::as_str)
                                        .map(|s| format!("#{s}"))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    ToolResult::ok(if names.is_empty() {
                        "No channels found".to_string()
                    } else {
                        names.join(", ")
                    })
                }
                Err(e) => ToolResult::err(e),
            },
            other => ToolResult::err(format!("Unknown Slack tool: {other}")),
        }
    }
}
