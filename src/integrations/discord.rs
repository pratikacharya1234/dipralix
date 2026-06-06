use serde_json::{json, Value};

use crate::integrations::DiscordConfig;
use crate::integrations::IntegrationService;
use crate::tools::ToolResult;
use crate::types::FunctionDeclaration;

pub struct DiscordIntegration {
    bot_token: String,
    client: reqwest::Client,
}

impl DiscordIntegration {
    pub fn new(config: &DiscordConfig) -> Self {
        DiscordIntegration {
            bot_token: config.bot_token.clone(),
            client: reqwest::Client::new(),
        }
    }

    fn api_url(path: &str) -> String {
        format!("https://discord.com/api/v10{}", path)
    }

    fn auth_header(token: &str) -> String {
        format!("Bot {}", token)
    }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let resp = self
            .client
            .get(Self::api_url(path))
            .header("Authorization", Self::auth_header(&self.bot_token))
            .header("User-Agent", "dipralix/1.0")
            .send()
            .await
            .map_err(|e| format!("Discord API request failed: {}", e))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "Discord API HTTP {}: {}",
                status.as_u16(),
                truncate_str(&body, 400)
            ));
        }

        serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        let resp = self
            .client
            .post(Self::api_url(path))
            .header("Authorization", Self::auth_header(&self.bot_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "dipralix/1.0")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Discord API request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "Discord API HTTP {}: {}",
                status.as_u16(),
                truncate_str(&text, 400)
            ));
        }

        if text.is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse response: {}", e))
    }

    async fn delete(&self, path: &str) -> Result<Value, String> {
        let resp = self
            .client
            .delete(Self::api_url(path))
            .header("Authorization", Self::auth_header(&self.bot_token))
            .header("User-Agent", "dipralix/1.0")
            .send()
            .await
            .map_err(|e| format!("Discord API request failed: {}", e))?;

        let status = resp.status();
        if status.as_u16() == 204 {
            return Ok(json!({"deleted": true}));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read: {}", e))?;
        if !status.is_success() {
            return Err(format!(
                "Discord API HTTP {}: {}",
                status.as_u16(),
                truncate_str(&body, 400)
            ));
        }
        Ok(json!({"deleted": true}))
    }
}

impl IntegrationService for DiscordIntegration {
    fn name(&self) -> &str {
        "discord"
    }

    fn tool_declarations(&self) -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "send_message".to_string(),
                description: "Send a message to a Discord channel. Channel ID is required.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "channel_id": { "type": "STRING", "description": "Discord channel ID (numeric)" },
                        "content": { "type": "STRING", "description": "Message content (max 2000 chars)" },
                        "embed_title": { "type": "STRING", "description": "Optional embed title" },
                        "embed_description": { "type": "STRING", "description": "Optional embed description" },
                        "embed_color": { "type": "INTEGER", "description": "Optional embed color (decimal, e.g. 16711680 for red)" }
                    },
                    "required": ["channel_id", "content"]
                }),
            },
            FunctionDeclaration {
                name: "read_messages".to_string(),
                description: "Read recent messages from a Discord channel. Returns message author, content, timestamp, and IDs.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "channel_id": { "type": "STRING", "description": "Discord channel ID (numeric)" },
                        "limit": { "type": "INTEGER", "description": "Number of messages to fetch (default: 20, max: 100)" },
                        "before": { "type": "STRING", "description": "Get messages before this message ID (for pagination)" }
                    },
                    "required": ["channel_id"]
                }),
            },
            FunctionDeclaration {
                name: "list_channels".to_string(),
                description: "List all text channels in a Discord guild (server).".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "guild_id": { "type": "STRING", "description": "Discord guild/server ID (numeric)" }
                    },
                    "required": ["guild_id"]
                }),
            },
            FunctionDeclaration {
                name: "list_guilds".to_string(),
                description: "List all Discord guilds (servers) the bot has access to.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {},
                    "required": []
                }),
            },
            FunctionDeclaration {
                name: "create_channel".to_string(),
                description: "Create a new text channel in a Discord guild.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "guild_id": { "type": "STRING", "description": "Discord guild/server ID (numeric)" },
                        "name": { "type": "STRING", "description": "Channel name (lowercase, hyphens)" },
                        "topic": { "type": "STRING", "description": "Channel topic/description" },
                        "category_id": { "type": "STRING", "description": "Parent category ID" }
                    },
                    "required": ["guild_id", "name"]
                }),
            },
            FunctionDeclaration {
                name: "delete_message".to_string(),
                description: "Delete a message from a Discord channel.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "channel_id": { "type": "STRING", "description": "Discord channel ID" },
                        "message_id": { "type": "STRING", "description": "Message ID to delete" }
                    },
                    "required": ["channel_id", "message_id"]
                }),
            },
            FunctionDeclaration {
                name: "get_channel_info".to_string(),
                description: "Get information about a specific Discord channel.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "channel_id": { "type": "STRING", "description": "Discord channel ID (numeric)" }
                    },
                    "required": ["channel_id"]
                }),
            },
        ]
    }

    fn call_tool(&self, tool_name: &str, args: Value) -> ToolResult {
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => return ToolResult::err("No async runtime available for Discord API call"),
        };

        match tool_name {
            "send_message" => {
                let channel_id = match args.get("channel_id").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ToolResult::err("Missing required argument: channel_id"),
                };
                let content = match args.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ToolResult::err("Missing required argument: content"),
                };

                let mut payload = json!({ "content": content });

                if let Some(title) = args.get("embed_title").and_then(|v| v.as_str()) {
                    let mut embed = json!({
                        "title": title,
                        "type": "rich"
                    });
                    if let Some(desc) = args.get("embed_description").and_then(|v| v.as_str()) {
                        embed["description"] = json!(desc);
                    }
                    if let Some(color) = args.get("embed_color").and_then(|v| v.as_u64()) {
                        embed["color"] = json!(color);
                    }
                    payload["embeds"] = json!([embed]);
                }

                if content.len() > 2000 {
                    return ToolResult::err("Message content exceeds 2000 character limit.");
                }

                let result =
                    rt.block_on(self.post(&format!("/channels/{}/messages", channel_id), &payload));
                match result {
                    Ok(msg) => {
                        let id = msg["id"].as_str().unwrap_or("?");
                        let channel = msg["channel_id"].as_str().unwrap_or(channel_id);
                        ToolResult::ok(format!(
                            "Message sent: {} (ID: {}, channel: {})",
                            truncate_str(content, 60),
                            id,
                            channel
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "read_messages" => {
                let channel_id = match args.get("channel_id").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ToolResult::err("Missing required argument: channel_id"),
                };
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20)
                    .min(100);
                let mut path = format!("/channels/{}/messages?limit={}", channel_id, limit);
                if let Some(before) = args.get("before").and_then(|v| v.as_str()) {
                    path.push_str(&format!("&before={}", before));
                }

                let result = rt.block_on(self.get(&path));
                match result {
                    Ok(messages) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let arr = messages.as_array().unwrap_or(&_empty);
                        if arr.is_empty() {
                            return ToolResult::ok("No messages found.");
                        }
                        let lines: Vec<String> = arr
                            .iter()
                            .map(|m| {
                                let author = m["author"]["username"].as_str().unwrap_or("?");
                                let content = m["content"].as_str().unwrap_or("");
                                let id = m["id"].as_str().unwrap_or("?");
                                let ts = m["timestamp"].as_str().unwrap_or("?");
                                let attachments =
                                    m["attachments"].as_array().map(|a| a.len()).unwrap_or(0);
                                let attach_str = if attachments > 0 {
                                    format!(" [{} attachments]", attachments)
                                } else {
                                    String::new()
                                };
                                let edited = if m["edited_timestamp"].is_null() {
                                    ""
                                } else {
                                    " (edited)"
                                };
                                format!(
                                    "[{}] {} {}: {}{}{}",
                                    ts,
                                    id,
                                    author,
                                    truncate_str(content, 100),
                                    attach_str,
                                    edited
                                )
                            })
                            .collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "list_channels" => {
                let guild_id = match args.get("guild_id").and_then(|v| v.as_str()) {
                    Some(g) => g,
                    None => return ToolResult::err("Missing required argument: guild_id"),
                };
                let result = rt.block_on(self.get(&format!("/guilds/{}/channels", guild_id)));
                match result {
                    Ok(channels) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let arr = channels.as_array().unwrap_or(&_empty);
                        if arr.is_empty() {
                            return ToolResult::ok("No channels found.");
                        }
                        let lines: Vec<String> = arr
                            .iter()
                            .map(|c| {
                                let name = c["name"].as_str().unwrap_or("?");
                                let id = c["id"].as_str().unwrap_or("?");
                                let ctype = match c["type"].as_u64().unwrap_or(0) {
                                    0 => "text",
                                    2 => "voice",
                                    4 => "category",
                                    5 => "announcement",
                                    15 => "forum",
                                    _ => "other",
                                };
                                let topic = c["topic"].as_str().unwrap_or("");
                                let topic_str = if topic.is_empty() {
                                    String::new()
                                } else {
                                    format!(" - {}", truncate_str(topic, 40))
                                };
                                format!("#{} ({}) [{}{}]", name, id, ctype, topic_str)
                            })
                            .collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "list_guilds" => {
                let result = rt.block_on(self.get("/users/@me/guilds"));
                match result {
                    Ok(guilds) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let arr = guilds.as_array().unwrap_or(&_empty);
                        if arr.is_empty() {
                            return ToolResult::ok("Bot is not in any guilds.");
                        }
                        let lines: Vec<String> = arr
                            .iter()
                            .map(|g| {
                                let name = g["name"].as_str().unwrap_or("?");
                                let id = g["id"].as_str().unwrap_or("?");
                                let owner = g["owner"].as_bool().unwrap_or(false);
                                let approx_members =
                                    g["approximate_member_count"].as_u64().unwrap_or(0);
                                format!(
                                    "{} ({})  members: {}  {}",
                                    name,
                                    id,
                                    approx_members,
                                    if owner { "[owner]" } else { "" }
                                )
                            })
                            .collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "create_channel" => {
                let guild_id = match args.get("guild_id").and_then(|v| v.as_str()) {
                    Some(g) => g,
                    None => return ToolResult::err("Missing required argument: guild_id"),
                };
                let name = match args.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => return ToolResult::err("Missing required argument: name"),
                };
                let mut payload = json!({ "name": name, "type": 0 });
                if let Some(topic) = args.get("topic").and_then(|v| v.as_str()) {
                    payload["topic"] = json!(topic);
                }
                if let Some(cat_id) = args.get("category_id").and_then(|v| v.as_str()) {
                    payload["parent_id"] = json!(cat_id);
                }
                let result =
                    rt.block_on(self.post(&format!("/guilds/{}/channels", guild_id), &payload));
                match result {
                    Ok(channel) => {
                        let ch_name = channel["name"].as_str().unwrap_or("?");
                        let ch_id = channel["id"].as_str().unwrap_or("?");
                        ToolResult::ok(format!("Created channel #{} (ID: {})", ch_name, ch_id))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "delete_message" => {
                let channel_id = match args.get("channel_id").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ToolResult::err("Missing required argument: channel_id"),
                };
                let message_id = match args.get("message_id").and_then(|v| v.as_str()) {
                    Some(m) => m,
                    None => return ToolResult::err("Missing required argument: message_id"),
                };
                let result = rt.block_on(
                    self.delete(&format!("/channels/{}/messages/{}", channel_id, message_id)),
                );
                match result {
                    Ok(_) => ToolResult::ok(format!(
                        "Deleted message {} from channel {}",
                        message_id, channel_id
                    )),
                    Err(e) => ToolResult::err(e),
                }
            }
            "get_channel_info" => {
                let channel_id = match args.get("channel_id").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ToolResult::err("Missing required argument: channel_id"),
                };
                let result = rt.block_on(self.get(&format!("/channels/{}", channel_id)));
                match result {
                    Ok(channel) => {
                        let name = channel["name"].as_str().unwrap_or("?");
                        let id = channel["id"].as_str().unwrap_or("?");
                        let ctype = match channel["type"].as_u64().unwrap_or(0) {
                            0 => "text",
                            2 => "voice",
                            4 => "category",
                            5 => "announcement",
                            15 => "forum",
                            _ => "other",
                        };
                        let topic = channel["topic"].as_str().unwrap_or("");
                        let position = channel["position"].as_u64().unwrap_or(0);
                        let nsfw = channel["nsfw"].as_bool().unwrap_or(false);
                        ToolResult::ok(format!(
                            "#{}\n  ID: {}\n  Type: {}\n  Position: {}\n  NSFW: {}\n  Topic: {}",
                            name,
                            id,
                            ctype,
                            position,
                            nsfw,
                            if topic.is_empty() { "(none)" } else { topic }
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            _ => ToolResult::err(format!("Unknown Discord tool: {}", tool_name)),
        }
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        None => s.to_string(),
        Some((idx, _)) => format!("{}...", &s[..idx]),
    }
}
