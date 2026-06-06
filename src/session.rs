use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::types::{Content, Part};

// ── Session data structure ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct SessionFile {
    version: String,
    model: String,
    created: String,
    updated: String,
    grounding: bool,
    thinking: bool,
    thinking_budget: i32,
    history: Vec<SerializedContent>,
}

#[derive(Serialize, Deserialize)]
struct SerializedContent {
    role: String,
    parts: Vec<SerializedPart>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum SerializedPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "function_call")]
    FunctionCall { name: String, args: String },
    #[serde(rename = "function_response")]
    FunctionResponse {
        name: String,
        response: String,
        is_error: bool,
    },
}

fn sessions_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dipralix")
        .join("sessions")
}

fn serialize_history(history: &[Content]) -> Vec<SerializedContent> {
    history
        .iter()
        .map(|c| {
            let parts: Vec<SerializedPart> = c
                .parts
                .iter()
                .map(|p| match p {
                    Part::Text { text, .. } => SerializedPart::Text { text: text.clone() },
                    Part::FunctionCall { function_call, .. } => SerializedPart::FunctionCall {
                        name: function_call.name.clone(),
                        args: function_call.args.to_string(),
                    },
                    Part::FunctionResponse { function_response } => {
                        SerializedPart::FunctionResponse {
                            name: function_response.name.clone(),
                            response: function_response.response.to_string(),
                            is_error: function_response.response.get("error").is_some(),
                        }
                    }
                    _ => SerializedPart::Text {
                        text: String::new(),
                    },
                })
                .collect();
            SerializedContent {
                role: c.role.clone(),
                parts,
            }
        })
        .collect()
}

fn deserialize_history(contents: &[SerializedContent]) -> Vec<Content> {
    contents
        .iter()
        .map(|c| {
            let parts: Vec<Part> = c
                .parts
                .iter()
                .map(|p| match p {
                    SerializedPart::Text { text } => Part::text(text),
                    SerializedPart::FunctionCall { name, args } => {
                        let args_val: serde_json::Value =
                            serde_json::from_str(args).unwrap_or_default();
                        Part::FunctionCall {
                            function_call: crate::types::FunctionCall {
                                name: name.clone(),
                                args: args_val,
                                thought_signature: None,
                            },
                            thought_signature: None,
                        }
                    }
                    SerializedPart::FunctionResponse {
                        name,
                        response,
                        is_error: _,
                    } => Part::FunctionResponse {
                        function_response: crate::types::FunctionResponse {
                            name: name.clone(),
                            response: serde_json::Value::String(response.clone()),
                            id: None,
                        },
                    },
                })
                .collect();
            Content {
                role: c.role.clone(),
                parts,
            }
        })
        .collect()
}

fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

// ── Public API ──────────────────────────────────────────────────────────────

pub fn save_session(
    name: &str,
    history: &[Content],
    model: &str,
    grounding: bool,
    thinking: bool,
    thinking_budget: i32,
) -> Result<()> {
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir)?;

    let path = dir.join(format!("{}.json", sanitize_name(name)));
    let now = timestamp();

    let session = SessionFile {
        version: "1.0".to_string(),
        model: model.to_string(),
        created: now.clone(),
        updated: now,
        grounding,
        thinking,
        thinking_budget,
        history: serialize_history(history),
    };

    let json = serde_json::to_string_pretty(&session)?;
    std::fs::write(&path, json)?;

    Ok(())
}

pub fn load_session(name: &str) -> Result<LoadedSession> {
    let path = sessions_dir().join(format!("{}.json", sanitize_name(name)));
    let content = std::fs::read_to_string(&path)?;
    let session: SessionFile = serde_json::from_str(&content)?;

    Ok(LoadedSession {
        model: session.model,
        grounding: session.grounding,
        thinking: session.thinking,
        thinking_budget: session.thinking_budget,
        history: deserialize_history(&session.history),
        created: session.created,
    })
}

pub fn list_sessions() -> Vec<SessionInfo> {
    let dir = sessions_dir();
    let mut sessions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let mut info = SessionInfo {
                    name: name.clone(),
                    created: String::new(),
                    turns: 0,
                    size_bytes: 0,
                };

                if let Ok(content) = std::fs::read_to_string(&path) {
                    info.size_bytes = content.len();
                    if let Ok(session) = serde_json::from_str::<serde_json::Value>(&content) {
                        info.created = session["created"].as_str().unwrap_or("?").to_string();
                        info.turns = session["history"].as_array().map(|a| a.len()).unwrap_or(0);
                    }
                }

                sessions.push(info);
            }
        }
    }

    sessions.sort_by(|a, b| b.created.cmp(&a.created));
    sessions
}

pub fn delete_session(name: &str) -> Result<()> {
    let path = sessions_dir().join(format!("{}.json", sanitize_name(name)));
    std::fs::remove_file(path)?;
    Ok(())
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// ── Output types ────────────────────────────────────────────────────────────

pub struct LoadedSession {
    pub model: String,
    pub grounding: bool,
    pub thinking: bool,
    pub thinking_budget: i32,
    pub history: Vec<Content>,
    #[allow(dead_code)]
    pub created: String,
}

pub struct SessionInfo {
    pub name: String,
    pub created: String,
    pub turns: usize,
    pub size_bytes: usize,
}
