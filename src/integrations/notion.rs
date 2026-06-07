use serde_json::{json, Value};

use crate::integrations::IntegrationService;
use crate::integrations::NotionConfig;
use crate::tools::ToolResult;
use crate::types::FunctionDeclaration;

/// Notion integration — searches the workspace and creates pages via an
/// internal integration token.
pub struct NotionIntegration {
    token: String,
    client: reqwest::Client,
}

impl NotionIntegration {
    pub fn new(config: &NotionConfig) -> Self {
        NotionIntegration {
            token: config.token.clone(),
            client: reqwest::Client::new(),
        }
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        let resp = self
            .client
            .post(format!("https://api.notion.com/v1/{path}"))
            .bearer_auth(&self.token)
            .header("Notion-Version", "2022-06-28")
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Notion API request failed: {e}"))?;
        let status = resp.status();
        let v: Value = resp
            .json()
            .await
            .map_err(|e| format!("Notion response parse failed: {e}"))?;
        if !status.is_success() {
            return Err(format!(
                "Notion API HTTP {}: {}",
                status.as_u16(),
                v.get("message").and_then(Value::as_str).unwrap_or("error")
            ));
        }
        Ok(v)
    }
}

impl IntegrationService for NotionIntegration {
    fn name(&self) -> &str {
        "notion"
    }

    fn tool_declarations(&self) -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "search".to_string(),
                description: "Search Notion pages and databases by text query.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "query": { "type": "STRING", "description": "Search text" }
                    },
                    "required": ["query"]
                }),
            },
            FunctionDeclaration {
                name: "create_page".to_string(),
                description:
                    "Create a Notion page under a parent page with a title and optional body text."
                        .to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "parent_page_id": { "type": "STRING", "description": "Parent page ID" },
                        "title":          { "type": "STRING", "description": "Page title" },
                        "content":        { "type": "STRING", "description": "Optional paragraph text" }
                    },
                    "required": ["parent_page_id", "title"]
                }),
            },
        ]
    }

    fn call_tool(&self, tool_name: &str, args: Value) -> ToolResult {
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => return ToolResult::err("No async runtime available for Notion API call"),
        };

        match tool_name {
            "search" => {
                let query = match args.get("query").and_then(Value::as_str) {
                    Some(q) => q,
                    None => return ToolResult::err("Missing required argument: query"),
                };
                let body = json!({ "query": query, "page_size": 10 });
                match rt.block_on(self.post("search", &body)) {
                    Ok(v) => {
                        let n = v
                            .get("results")
                            .and_then(Value::as_array)
                            .map(|a| a.len())
                            .unwrap_or(0);
                        ToolResult::ok(format!("Notion search '{query}' — {n} result(s)"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "create_page" => {
                let parent = match args.get("parent_page_id").and_then(Value::as_str) {
                    Some(p) => p,
                    None => return ToolResult::err("Missing required argument: parent_page_id"),
                };
                let title = match args.get("title").and_then(Value::as_str) {
                    Some(t) => t,
                    None => return ToolResult::err("Missing required argument: title"),
                };
                let mut body = json!({
                    "parent": { "page_id": parent },
                    "properties": {
                        "title": { "title": [ { "text": { "content": title } } ] }
                    }
                });
                if let Some(content) = args.get("content").and_then(Value::as_str) {
                    body["children"] = json!([{
                        "object": "block",
                        "type": "paragraph",
                        "paragraph": { "rich_text": [ { "type": "text", "text": { "content": content } } ] }
                    }]);
                }
                match rt.block_on(self.post("pages", &body)) {
                    Ok(_) => ToolResult::ok(format!("Created Notion page: {title}")),
                    Err(e) => ToolResult::err(e),
                }
            }
            other => ToolResult::err(format!("Unknown Notion tool: {other}")),
        }
    }
}
