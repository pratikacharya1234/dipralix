use std::sync::Mutex;

use serde_json::{json, Value};

use crate::integrations::GoogleConfig;
use crate::integrations::IntegrationService;
use crate::tools::ToolResult;
use crate::types::FunctionDeclaration;

// ── Shared Google OAuth2 handler ─────────────────────────────────────────────

struct GoogleClient {
    client_id: String,
    client_secret: String,
    access_token: Mutex<String>,
    refresh_token: String,
    http: reqwest::Client,
}

impl GoogleClient {
    fn new(config: &GoogleConfig) -> Self {
        GoogleClient {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            access_token: Mutex::new(config.access_token.clone()),
            refresh_token: config.refresh_token.clone(),
            http: reqwest::Client::new(),
        }
    }

    async fn refresh_access_token(&self) -> Result<String, String> {
        let resp = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("refresh_token", self.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| format!("Token refresh request failed: {}", e))?;

        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read token response: {}", e))?;

        let data: Value = serde_json::from_str(&body).map_err(|e| {
            format!(
                "Failed to parse token response: {} - body: {}",
                e,
                truncate_g(&body, 200)
            )
        })?;

        if let Some(error) = data.get("error").and_then(|v| v.as_str()) {
            let desc = data
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            return Err(format!("OAuth2 error: {} - {}", error, desc));
        }

        let new_token = data["access_token"]
            .as_str()
            .ok_or_else(|| format!("No access_token in response: {}", truncate_g(&body, 200)))?
            .to_string();

        // Update stored token
        if let Ok(mut token) = self.access_token.lock() {
            *token = new_token.clone();
        }

        Ok(new_token)
    }

    async fn get_token(&self) -> Result<String, String> {
        let token = self
            .access_token
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        if !token.is_empty() {
            return Ok(token.clone());
        }
        drop(token);

        if self.refresh_token.is_empty() {
            return Err("No access token or refresh token configured.\n\
                 Set [integrations.google] in ~/.forge/config.toml with:\n\
                 - client_id, client_secret, refresh_token\n\
                 See: https://console.cloud.google.com/apis/credentials"
                .to_string());
        }

        self.refresh_access_token().await
    }

    async fn api_get(&self, url: &str) -> Result<Value, String> {
        let token = self.get_token().await?;
        let resp = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Google API request failed: {}", e))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Read failed: {}", e))?;

        if status.as_u16() == 401 {
            // Token expired, refresh and retry once
            let new_token = self.refresh_access_token().await?;
            let resp2 = self
                .http
                .get(url)
                .header("Authorization", format!("Bearer {}", new_token))
                .send()
                .await
                .map_err(|e| format!("Retry failed: {}", e))?;
            let status2 = resp2.status();
            let body2 = resp2
                .text()
                .await
                .map_err(|e| format!("Read failed: {}", e))?;
            if !status2.is_success() {
                return Err(format!(
                    "Google API HTTP {} (after refresh): {}",
                    status2.as_u16(),
                    truncate_g(&body2, 400)
                ));
            }
            return serde_json::from_str(&body2)
                .map_err(|e| format!("Parse error after refresh: {}", e));
        }

        if !status.is_success() {
            return Err(format!(
                "Google API HTTP {}: {}",
                status.as_u16(),
                truncate_g(&body, 400)
            ));
        }
        serde_json::from_str(&body).map_err(|e| format!("Parse error: {}", e))
    }

    async fn api_post(&self, url: &str, body: &Value) -> Result<Value, String> {
        let token = self.get_token().await?;
        let resp = self
            .http
            .post(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Google API request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Read failed: {}", e))?;

        if status.as_u16() == 401 {
            let new_token = self.refresh_access_token().await?;
            let resp2 = self
                .http
                .post(url)
                .header("Authorization", format!("Bearer {}", new_token))
                .header("Content-Type", "application/json")
                .json(body)
                .send()
                .await
                .map_err(|e| format!("Retry failed: {}", e))?;
            let status2 = resp2.status();
            let text2 = resp2
                .text()
                .await
                .map_err(|e| format!("Read failed: {}", e))?;
            if !status2.is_success() {
                return Err(format!(
                    "Google API HTTP {} (after refresh): {}",
                    status2.as_u16(),
                    truncate_g(&text2, 400)
                ));
            }
            return serde_json::from_str(&text2)
                .map_err(|e| format!("Parse error after refresh: {}", e));
        }

        if !status.is_success() {
            return Err(format!(
                "Google API HTTP {}: {}",
                status.as_u16(),
                truncate_g(&text, 400)
            ));
        }
        if text.is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_str(&text).map_err(|e| format!("Parse error: {}", e))
    }

    async fn api_delete(&self, url: &str) -> Result<Value, String> {
        let token = self.get_token().await?;
        let resp = self
            .http
            .delete(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Google API request failed: {}", e))?;

        let status = resp.status();
        if status.as_u16() == 204 {
            return Ok(json!({"deleted": true}));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Read failed: {}", e))?;

        if status.as_u16() == 401 {
            let new_token = self.refresh_access_token().await?;
            let resp2 = self
                .http
                .delete(url)
                .header("Authorization", format!("Bearer {}", new_token))
                .send()
                .await
                .map_err(|e| format!("Retry failed: {}", e))?;
            let status2 = resp2.status();
            if status2.as_u16() == 204 {
                return Ok(json!({"deleted": true}));
            }
            let body2 = resp2
                .text()
                .await
                .map_err(|e| format!("Read failed: {}", e))?;
            if !status2.is_success() {
                return Err(format!(
                    "Google API HTTP {} (after refresh): {}",
                    status2.as_u16(),
                    truncate_g(&body2, 400)
                ));
            }
            return Ok(json!({"deleted": true}));
        }

        if !status.is_success() {
            return Err(format!(
                "Google API HTTP {}: {}",
                status.as_u16(),
                truncate_g(&body, 400)
            ));
        }
        Ok(json!({"deleted": true}))
    }
}

// ── Google Drive Integration ─────────────────────────────────────────────────

pub struct GDriveIntegration {
    client: GoogleClient,
}

impl GDriveIntegration {
    pub fn new(config: &GoogleConfig) -> Self {
        GDriveIntegration {
            client: GoogleClient::new(config),
        }
    }

    fn format_file(file: &Value) -> String {
        let name = file["name"].as_str().unwrap_or("(unnamed)");
        let id = file["id"].as_str().unwrap_or("?");
        let mime = file["mimeType"].as_str().unwrap_or("?");
        let size = file["size"]
            .as_str()
            .map(format_size)
            .unwrap_or_else(|| "?".to_string());
        let modified = file["modifiedTime"].as_str().unwrap_or("?");
        let is_folder = mime == "application/vnd.google-apps.folder";
        let kind = if is_folder { "[DIR]" } else { "[FILE]" };
        format!(
            "{}  {}  {}  {}  {}  {}",
            kind, name, id, size, mime, modified
        )
    }
}

impl IntegrationService for GDriveIntegration {
    fn name(&self) -> &str {
        "gdrive"
    }

    fn tool_declarations(&self) -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "list_files".to_string(),
                description: "List files and folders in Google Drive. Supports search queries and pagination.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "query": { "type": "STRING", "description": "Search query: 'name contains \"report\"', 'mimeType=\"application/pdf\"', 'trashed=false', etc." },
                        "page_size": { "type": "INTEGER", "description": "Number of results (default: 20, max: 100)" },
                        "order_by": { "type": "STRING", "description": "Sort: 'name', 'modifiedTime desc', 'createdTime' (default: modifiedTime desc)" }
                    },
                    "required": []
                }),
            },
            FunctionDeclaration {
                name: "get_file".to_string(),
                description: "Get metadata for a specific file or folder by ID.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "file_id": { "type": "STRING", "description": "Drive file ID" }
                    },
                    "required": ["file_id"]
                }),
            },
            FunctionDeclaration {
                name: "download_file".to_string(),
                description: "Download the content of a Google Drive file. Works for text files, documents, spreadsheets (exports as text/csv), and more. Returns file content as text.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "file_id": { "type": "STRING", "description": "Drive file ID" },
                        "mime_type": { "type": "STRING", "description": "Export MIME type for Google Docs/Sheets (e.g. 'text/plain', 'text/csv'). Not needed for regular files." }
                    },
                    "required": ["file_id"]
                }),
            },
            FunctionDeclaration {
                name: "create_folder".to_string(),
                description: "Create a new folder in Google Drive. Returns the folder ID.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "name": { "type": "STRING", "description": "Folder name" },
                        "parent_id": { "type": "STRING", "description": "Parent folder ID (default: root)" }
                    },
                    "required": ["name"]
                }),
            },
            FunctionDeclaration {
                name: "upload_file".to_string(),
                description: "Upload a file to Google Drive. Content can be plain text or base64-encoded binary.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "name": { "type": "STRING", "description": "File name (e.g. 'report.txt')" },
                        "content": { "type": "STRING", "description": "File content as text or base64" },
                        "mime_type": { "type": "STRING", "description": "MIME type (default: text/plain). Use 'application/json', 'text/csv', etc." },
                        "parent_id": { "type": "STRING", "description": "Parent folder ID (default: root)" }
                    },
                    "required": ["name", "content"]
                }),
            },
            FunctionDeclaration {
                name: "search_files".to_string(),
                description: "Full-text search across all files in Google Drive. Finds files whose content or name matches the query.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "query": { "type": "STRING", "description": "Search terms (matches file name and content)" },
                        "page_size": { "type": "INTEGER", "description": "Number of results (default: 10)" }
                    },
                    "required": ["query"]
                }),
            },
            FunctionDeclaration {
                name: "delete_file".to_string(),
                description: "Move a file or folder to trash. Use file_id from list_files or search_files.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "file_id": { "type": "STRING", "description": "Drive file ID" }
                    },
                    "required": ["file_id"]
                }),
            },
        ]
    }

    fn call_tool(&self, tool_name: &str, args: Value) -> ToolResult {
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => {
                return ToolResult::err("No async runtime available for Google Drive API call")
            }
        };

        match tool_name {
            "list_files" => {
                let page_size = args
                    .get("page_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20)
                    .min(100);
                let order_by = args
                    .get("order_by")
                    .and_then(|v| v.as_str())
                    .unwrap_or("modifiedTime desc");
                let fields = "files(id,name,mimeType,size,modifiedTime,webViewLink,trashed)";
                let mut url =
                    format!(
                    "https://www.googleapis.com/drive/v3/files?pageSize={}&orderBy={}&fields={}",
                    page_size, url_encode_g(order_by), fields
                );
                if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
                    url.push_str(&format!("&q={}", url_encode_g(query)));
                } else {
                    url.push_str("&q=trashed=false");
                }

                let result = rt.block_on(self.client.api_get(&url));
                match result {
                    Ok(data) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let files = data["files"].as_array().unwrap_or(&_empty);
                        if files.is_empty() {
                            return ToolResult::ok("No files found.");
                        }
                        let lines: Vec<String> = files.iter().map(Self::format_file).collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "get_file" => {
                let file_id = match args.get("file_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ToolResult::err("Missing required argument: file_id"),
                };
                let url = format!(
                    "https://www.googleapis.com/drive/v3/files/{}?fields=id,name,mimeType,size,createdTime,modifiedTime,webViewLink,description,owners,trashed",
                    file_id
                );
                let result = rt.block_on(self.client.api_get(&url));
                match result {
                    Ok(file) => {
                        let name = file["name"].as_str().unwrap_or("?");
                        let mime = file["mimeType"].as_str().unwrap_or("?");
                        let size = file["size"]
                            .as_str()
                            .map(format_size)
                            .unwrap_or_else(|| "unknown".to_string());
                        let created = file["createdTime"].as_str().unwrap_or("?");
                        let modified = file["modifiedTime"].as_str().unwrap_or("?");
                        let desc = file["description"].as_str().unwrap_or("");
                        let trashed = file["trashed"].as_bool().unwrap_or(false);
                        let owners: Vec<&str> = file["owners"]
                            .as_array()
                            .map(|a| a.iter().filter_map(|o| o["displayName"].as_str()).collect())
                            .unwrap_or_default();
                        let link = file["webViewLink"].as_str().unwrap_or("");
                        ToolResult::ok(format!(
                            "Name: {}\nID: {}\nMIME: {}\nSize: {}\nCreated: {}\nModified: {}\nTrashed: {}\nOwners: {}\nDescription: {}\nLink: {}",
                            name, file_id, mime, size, created, modified, trashed,
                            owners.join(", "),
                            if desc.is_empty() { "(none)" } else { desc },
                            link
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "download_file" => {
                let file_id = match args.get("file_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ToolResult::err("Missing required argument: file_id"),
                };
                let export_mime = args.get("mime_type").and_then(|v| v.as_str());

                let url = if let Some(mime) = export_mime {
                    format!(
                        "https://www.googleapis.com/drive/v3/files/{}/export?mimeType={}",
                        file_id, mime
                    )
                } else {
                    format!(
                        "https://www.googleapis.com/drive/v3/files/{}?alt=media",
                        file_id
                    )
                };

                let result = rt.block_on(self.client.api_get(&url));
                match result {
                    Ok(data) => {
                        if let Some(text) = data.as_str() {
                            let lines = text.lines().count();
                            let chars = text.len();
                            ToolResult::ok(format!(
                                "File content ({} lines, {} chars):\n\n{}",
                                lines, chars, text
                            ))
                        } else {
                            // For structured JSON responses instead of raw text
                            ToolResult::ok(format!(
                                "File content:\n{}",
                                serde_json::to_string_pretty(&data).unwrap_or_default()
                            ))
                        }
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "create_folder" => {
                let name = match args.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => return ToolResult::err("Missing required argument: name"),
                };
                let parent = args
                    .get("parent_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("root");
                let payload = json!({
                    "name": name,
                    "mimeType": "application/vnd.google-apps.folder",
                    "parents": [parent]
                });
                let url = "https://www.googleapis.com/drive/v3/files?fields=id,name,webViewLink";
                let result = rt.block_on(self.client.api_post(url, &payload));
                match result {
                    Ok(folder) => {
                        let fid = folder["id"].as_str().unwrap_or("?");
                        let fname = folder["name"].as_str().unwrap_or("?");
                        let link = folder["webViewLink"].as_str().unwrap_or("");
                        ToolResult::ok(format!(
                            "Created folder '{}' (ID: {})\nLink: {}",
                            fname, fid, link
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "upload_file" => {
                let fname = match args.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => return ToolResult::err("Missing required argument: name"),
                };
                let content = match args.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ToolResult::err("Missing required argument: content"),
                };
                let mime = args
                    .get("mime_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("text/plain");
                let parent = args
                    .get("parent_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("root");

                let boundary = "forge_upload_boundary";
                let mut body = String::new();
                body.push_str(&format!("--{}\r\n", boundary));
                body.push_str("Content-Type: application/json\r\n\r\n");
                body.push_str(
                    &json!({
                        "name": fname,
                        "parents": [parent]
                    })
                    .to_string(),
                );
                body.push_str(&format!("\r\n--{}\r\n", boundary));
                body.push_str(&format!("Content-Type: {}\r\n\r\n", mime));
                body.push_str(content);
                body.push_str(&format!("\r\n--{}--\r\n", boundary));

                let result = rt.block_on(async {
                    let token = self.client.get_token().await?;
                    let resp = self.client.http
                        .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,name,webViewLink")
                        .header("Authorization", format!("Bearer {}", token))
                        .header("Content-Type", format!("multipart/related; boundary={}", boundary))
                        .body(body)
                        .send()
                        .await
                        .map_err(|e| format!("Upload failed: {}", e))?;

                    let status = resp.status();
                    let resp_body = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
                    if !status.is_success() {
                        return Err(format!("Upload HTTP {}: {}", status.as_u16(), truncate_g(&resp_body, 400)));
                    }
                    serde_json::from_str::<Value>(&resp_body).map_err(|e| format!("Parse error: {}", e))
                });

                match result {
                    Ok(file) => {
                        let fid = file["id"].as_str().unwrap_or("?");
                        let link = file["webViewLink"].as_str().unwrap_or("");
                        ToolResult::ok(format!(
                            "Uploaded '{}' (ID: {})\nLink: {}",
                            fname, fid, link
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "search_files" => {
                let query = match args.get("query").and_then(|v| v.as_str()) {
                    Some(q) => q,
                    None => return ToolResult::err("Missing required argument: query"),
                };
                let page_size = args
                    .get("page_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10)
                    .min(100);
                let url = format!(
                    "https://www.googleapis.com/drive/v3/files?q=fullText contains '{}' and trashed=false&pageSize={}&fields=files(id,name,mimeType,size,modifiedTime)",
                    query.replace('\'', "\\'"), page_size
                );
                let result = rt.block_on(self.client.api_get(&url));
                match result {
                    Ok(data) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let files = data["files"].as_array().unwrap_or(&_empty);
                        if files.is_empty() {
                            return ToolResult::ok("No files matching that query.");
                        }
                        let lines: Vec<String> = files.iter().map(Self::format_file).collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "delete_file" => {
                let file_id = match args.get("file_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ToolResult::err("Missing required argument: file_id"),
                };
                let url = format!("https://www.googleapis.com/drive/v3/files/{}", file_id);
                let result = rt.block_on(self.client.api_delete(&url));
                match result {
                    Ok(_) => ToolResult::ok(format!("File {} moved to trash.", file_id)),
                    Err(e) => ToolResult::err(e),
                }
            }
            _ => ToolResult::err(format!("Unknown GDrive tool: {}", tool_name)),
        }
    }
}

// ── Gmail Integration ────────────────────────────────────────────────────────

pub struct GmailIntegration {
    client: GoogleClient,
}

impl GmailIntegration {
    pub fn new(config: &GoogleConfig) -> Self {
        GmailIntegration {
            client: GoogleClient::new(config),
        }
    }

    fn decode_base64url(data: &str) -> String {
        let mut b64 = data.replace('-', "+").replace('_', "/");
        match b64.len() % 4 {
            1 => b64.push_str("==="),
            2 => b64.push_str("=="),
            3 => b64.push('='),
            _ => {}
        }
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(&b64)
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .unwrap_or_else(|_| "[base64 decode failed]".to_string())
    }

    fn format_header(headers: &[Value], name: &str) -> String {
        headers
            .iter()
            .find(|h| h["name"].as_str().unwrap_or("").eq_ignore_ascii_case(name))
            .and_then(|h| h["value"].as_str())
            .unwrap_or("?")
            .to_string()
    }

    fn format_email_preview(msg: &Value) -> String {
        let id = msg["id"].as_str().unwrap_or("?");
        let _empty: Vec<serde_json::Value> = Vec::new();
        let headers = msg["payload"]["headers"].as_array().unwrap_or(&_empty);
        let subject = Self::format_header(headers, "Subject");
        let from = Self::format_header(headers, "From");
        let date = Self::format_header(headers, "Date");
        let _snippet = msg["snippet"].as_str().unwrap_or("");
        let unread = msg["labelIds"]
            .as_array()
            .map(|a| a.iter().any(|l| l.as_str() == Some("UNREAD")))
            .unwrap_or(false);
        let prefix = if unread { "[UNREAD]" } else { "[READ]  " };
        format!(
            "{} {}  {}  From: {}  {}",
            prefix,
            id,
            date,
            truncate_g(&from, 30),
            subject
        )
    }
}

impl IntegrationService for GmailIntegration {
    fn name(&self) -> &str {
        "gmail"
    }

    fn tool_declarations(&self) -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "send_email".to_string(),
                description: "Send an email from your Gmail account. Supports plain text and HTML body. To, subject, and body are required.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "to": { "type": "STRING", "description": "Recipient email address" },
                        "subject": { "type": "STRING", "description": "Email subject line" },
                        "body": { "type": "STRING", "description": "Email body text (plain text)" },
                        "cc": { "type": "STRING", "description": "CC recipients (comma-separated)" },
                        "bcc": { "type": "STRING", "description": "BCC recipients (comma-separated)" }
                    },
                    "required": ["to", "subject", "body"]
                }),
            },
            FunctionDeclaration {
                name: "list_emails".to_string(),
                description: "List recent emails from your Gmail inbox. Returns sender, subject, date, snippet, and read/unread status.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "max_results": { "type": "INTEGER", "description": "Number of emails (default: 10, max: 50)" },
                        "label": { "type": "STRING", "description": "Gmail label: INBOX, SENT, DRAFT, UNREAD, STARRED, etc. (default: INBOX)" },
                        "query": { "type": "STRING", "description": "Search query (from:alice@example.com, subject:report, newer_than:2d)" }
                    },
                    "required": []
                }),
            },
            FunctionDeclaration {
                name: "get_email".to_string(),
                description: "Get the full content of a specific email by ID. Returns headers, body text, and attachments info.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "email_id": { "type": "STRING", "description": "Email message ID from list_emails" }
                    },
                    "required": ["email_id"]
                }),
            },
            FunctionDeclaration {
                name: "search_emails".to_string(),
                description: "Search Gmail with full Gmail search syntax. Returns matching emails with metadata.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "query": { "type": "STRING", "description": "Gmail search: 'from:boss@company.com newer_than:7d', 'has:attachment subject:invoice', etc." },
                        "max_results": { "type": "INTEGER", "description": "Max results (default: 20)" }
                    },
                    "required": ["query"]
                }),
            },
            FunctionDeclaration {
                name: "list_labels".to_string(),
                description: "List all Gmail labels and their message counts.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {},
                    "required": []
                }),
            },
            FunctionDeclaration {
                name: "mark_read".to_string(),
                description: "Mark an email as read. Removes the UNREAD label.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "email_id": { "type": "STRING", "description": "Email message ID" }
                    },
                    "required": ["email_id"]
                }),
            },
            FunctionDeclaration {
                name: "delete_email".to_string(),
                description: "Move an email to trash.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "email_id": { "type": "STRING", "description": "Email message ID" }
                    },
                    "required": ["email_id"]
                }),
            },
        ]
    }

    fn call_tool(&self, tool_name: &str, args: Value) -> ToolResult {
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => return ToolResult::err("No async runtime available for Gmail API call"),
        };

        match tool_name {
            "send_email" => {
                let to = match args.get("to").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ToolResult::err("Missing required argument: to"),
                };
                let subject = match args.get("subject").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ToolResult::err("Missing required argument: subject"),
                };
                let body_text = match args.get("body").and_then(|v| v.as_str()) {
                    Some(b) => b,
                    None => return ToolResult::err("Missing required argument: body"),
                };

                let mut raw_headers = format!("From: me\r\nTo: {}\r\nSubject: {}\r\n", to, subject);
                if let Some(cc) = args.get("cc").and_then(|v| v.as_str()) {
                    raw_headers.push_str(&format!("Cc: {}\r\n", cc));
                }
                if let Some(bcc) = args.get("bcc").and_then(|v| v.as_str()) {
                    raw_headers.push_str(&format!("Bcc: {}\r\n", bcc));
                }
                raw_headers.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
                raw_headers.push_str("MIME-Version: 1.0\r\n\r\n");
                raw_headers.push_str(body_text);

                use base64::Engine as _;
                let raw_b64 = base64::engine::general_purpose::STANDARD
                    .encode(raw_headers.as_bytes())
                    .replace('+', "-")
                    .replace('/', "_")
                    .trim_end_matches('=')
                    .to_string();

                let payload = json!({ "raw": raw_b64 });
                let result = rt.block_on(self.client.api_post(
                    "https://gmail.googleapis.com/gmail/v1/users/me/messages/send",
                    &payload,
                ));
                match result {
                    Ok(msg) => {
                        let msg_id = msg["id"].as_str().unwrap_or("?");
                        let thread_id = msg["threadId"].as_str().unwrap_or("?");
                        ToolResult::ok(format!(
                            "Email sent to {} ({})\n  Message ID: {}\n  Thread ID: {}",
                            to, subject, msg_id, thread_id
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "list_emails" => {
                let max = args
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10)
                    .min(50);
                let label = args
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("INBOX");
                let mut url = format!(
                    "https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults={}&labelIds={}",
                    max, label
                );
                if let Some(q) = args.get("query").and_then(|v| v.as_str()) {
                    url.push_str(&format!("&q={}", url_encode_g(q)));
                }

                let result = rt.block_on(self.client.api_get(&url));
                match result {
                    Ok(data) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let messages = data["messages"].as_array().unwrap_or(&_empty);
                        if messages.is_empty() {
                            return ToolResult::ok("No emails found.");
                        }
                        let mut lines = Vec::new();
                        for msg in messages {
                            if let Some(mid) = msg["id"].as_str() {
                                // Fetch full message to get headers
                                let detail_result = rt.block_on(self.client.api_get(
                                    &format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=metadata&metadataHeaders=Subject&metadataHeaders=From&metadataHeaders=Date",
                                    mid)
                                ));
                                if let Ok(detail) = detail_result {
                                    lines.push(Self::format_email_preview(&detail));
                                }
                            }
                        }
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "get_email" => {
                let email_id = match args.get("email_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ToolResult::err("Missing required argument: email_id"),
                };
                let url = format!(
                    "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=full",
                    email_id
                );
                let result = rt.block_on(self.client.api_get(&url));
                match result {
                    Ok(msg) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let headers = msg["payload"]["headers"].as_array().unwrap_or(&_empty);
                        let subject = Self::format_header(headers, "Subject");
                        let from = Self::format_header(headers, "From");
                        let to = Self::format_header(headers, "To");
                        let date = Self::format_header(headers, "Date");
                        let snippet = msg["snippet"].as_str().unwrap_or("");

                        // Decode body
                        let body = Self::decode_gmail_body(&msg["payload"]);
                        let labels: Vec<&str> = msg["labelIds"]
                            .as_array()
                            .map(|a| a.iter().filter_map(|l| l.as_str()).collect())
                            .unwrap_or_default();

                        let parts = msg["payload"]["parts"].as_array();
                        let attachment_count = parts
                            .map(|p| {
                                p.iter()
                                    .filter(|part| {
                                        part["filename"]
                                            .as_str()
                                            .map(|f| !f.is_empty())
                                            .unwrap_or(false)
                                    })
                                    .count()
                            })
                            .unwrap_or(0);

                        ToolResult::ok(format!(
                            "Subject: {}\nFrom: {}\nTo: {}\nDate: {}\nLabels: {}\nAttachments: {}\nSnippet: {}\n\nBody:\n{}",
                            subject, from, to, date, labels.join(", "), attachment_count, snippet, body
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "search_emails" => {
                let query = match args.get("query").and_then(|v| v.as_str()) {
                    Some(q) => q,
                    None => return ToolResult::err("Missing required argument: query"),
                };
                let max = args
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20)
                    .min(100);
                let url = format!(
                    "https://gmail.googleapis.com/gmail/v1/users/me/messages?q={}&maxResults={}",
                    url_encode_g(query),
                    max
                );
                let result = rt.block_on(self.client.api_get(&url));
                match result {
                    Ok(data) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let messages = data["messages"].as_array().unwrap_or(&_empty);
                        if messages.is_empty() {
                            return ToolResult::ok("No emails match that query.");
                        }
                        let result_count = data["resultSizeEstimate"]
                            .as_u64()
                            .unwrap_or(messages.len() as u64);
                        let mut lines = Vec::new();
                        for msg in messages.iter().take(20) {
                            if let Some(mid) = msg["id"].as_str() {
                                let detail_result = rt.block_on(self.client.api_get(
                                    &format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=metadata&metadataHeaders=Subject&metadataHeaders=From&metadataHeaders=Date", mid)
                                ));
                                if let Ok(detail) = detail_result {
                                    lines.push(Self::format_email_preview(&detail));
                                }
                            }
                        }
                        let header = if result_count > 20 {
                            format!("{} results (showing 20):\n", result_count)
                        } else {
                            format!("{} results:\n", result_count)
                        };
                        ToolResult::ok(format!("{}{}", header, lines.join("\n")))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "list_labels" => {
                let result = rt.block_on(
                    self.client
                        .api_get("https://gmail.googleapis.com/gmail/v1/users/me/labels"),
                );
                match result {
                    Ok(data) => {
                        let _empty: Vec<serde_json::Value> = Vec::new();
                        let labels = data["labels"].as_array().unwrap_or(&_empty);
                        if labels.is_empty() {
                            return ToolResult::ok("No labels found.");
                        }
                        let lines: Vec<String> = labels
                            .iter()
                            .map(|l| {
                                let name = l["name"].as_str().unwrap_or("?");
                                let id = l["id"].as_str().unwrap_or("?");
                                let msg_total = l["messagesTotal"].as_u64().unwrap_or(0);
                                let msg_unread = l["messagesUnread"].as_u64().unwrap_or(0);
                                let ltype = l["type"].as_str().unwrap_or("user");
                                format!(
                                    "{} ({})  {} total / {} unread  [{}]",
                                    name, id, msg_total, msg_unread, ltype
                                )
                            })
                            .collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "mark_read" => {
                let email_id = match args.get("email_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ToolResult::err("Missing required argument: email_id"),
                };
                let payload = json!({ "removeLabelIds": ["UNREAD"] });
                let result = rt.block_on(self.client.api_post(
                    &format!(
                        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/modify",
                        email_id
                    ),
                    &payload,
                ));
                match result {
                    Ok(_) => ToolResult::ok(format!("Marked email {} as read.", email_id)),
                    Err(e) => ToolResult::err(e),
                }
            }
            "delete_email" => {
                let email_id = match args.get("email_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ToolResult::err("Missing required argument: email_id"),
                };
                let result = rt.block_on(self.client.api_delete(&format!(
                    "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
                    email_id
                )));
                match result {
                    Ok(_) => ToolResult::ok(format!("Email {} moved to trash.", email_id)),
                    Err(e) => ToolResult::err(e),
                }
            }
            _ => ToolResult::err(format!("Unknown Gmail tool: {}", tool_name)),
        }
    }
}

impl GmailIntegration {
    fn decode_gmail_body(payload: &Value) -> String {
        // Try parts first (multipart messages)
        if let Some(parts) = payload["parts"].as_array() {
            for part in parts {
                let mime = part["mimeType"].as_str().unwrap_or("");
                if mime == "text/plain" || mime.starts_with("text/plain") {
                    if let Some(data) = part["body"]["data"].as_str() {
                        let decoded = Self::decode_base64url(data);
                        if !decoded.is_empty() {
                            return decoded;
                        }
                    }
                }
            }
            // Fallback: first text/html part
            for part in parts {
                let mime = part["mimeType"].as_str().unwrap_or("");
                if mime == "text/html" || mime.starts_with("text/html") {
                    if let Some(data) = part["body"]["data"].as_str() {
                        let decoded = Self::decode_base64url(data);
                        if !decoded.is_empty() {
                            return format!("[HTML content - {} chars]", decoded.len());
                        }
                    }
                }
            }
        }

        // Try top-level body
        if let Some(data) = payload["body"]["data"].as_str() {
            let decoded = Self::decode_base64url(data);
            if !decoded.is_empty() {
                return decoded;
            }
        }

        "(no readable body)".to_string()
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn format_size(s: &str) -> String {
    let bytes: u64 = s.parse().unwrap_or(0);
    if bytes >= 1_073_741_824 {
        format!("{:.1}GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1}MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

fn truncate_g(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        None => s.to_string(),
        Some((idx, _)) => format!("{}...", &s[..idx]),
    }
}

fn url_encode_g(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('\'', "%27")
        .replace('#', "%23")
        .replace('&', "%26")
        .replace('?', "%3F")
        .replace('=', "%3D")
        .replace('+', "%2B")
}
