use std::collections::HashMap;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use crate::tools::{ToolContext, ToolResult};

// ── JSON-RPC 2.0 Types ─────────────────────────────────────────────────────────

#[derive(Serialize, Debug)]
#[allow(dead_code)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct RpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct RpcError {
    code: i32,
    message: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    capabilities: Value,
    #[serde(rename = "serverInfo")]
    server_info: ServerInfo,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ListToolsResult {
    tools: Vec<McpToolDecl>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct McpToolDecl {
    pub name: String,
    pub description: String,
    #[serde(default)]
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Serialize, Debug)]
#[allow(dead_code)]
struct CallToolParams {
    name: String,
    arguments: Value,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct CallToolResult {
    content: Vec<McpContent>,
    #[serde(default)]
    #[serde(rename = "isError")]
    is_error: bool,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "resource")]
    Resource { resource: Value },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

// ── Server Configuration ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub disabled: bool,
}

impl McpServerConfig {
    pub fn is_valid(&self) -> bool {
        !self.command.is_empty()
    }
}

// ── Active Server Instance ─────────────────────────────────────────────────────

/// Internal server state behind a mutex for thread-safe access.
struct McpServerInner {
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
    tools: Vec<McpToolDecl>,
}

impl McpServerInner {
    async fn exchange<T: for<'de> Deserialize<'de>>(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<T> {
        let id = self.next_id;
        self.next_id += 1;

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&request)?;

        // Write with 5s timeout
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            self.stdin.write_all(json.as_bytes()).await?;
            self.stdin.write_all(b"\n").await?;
            self.stdin.flush().await?;
            Ok::<_, anyhow::Error>(())
        })
        .await
        .context("MCP write timeout")?
        .context("MCP write failed")?;

        // Read until we get the matching response id
        let mut line_buf = String::new();

        let inner: Result<T, anyhow::Error> = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            loop {
                line_buf.clear();
                let n = self.reader.read_line(&mut line_buf).await?;
                if n == 0 {
                    anyhow::bail!("MCP server closed connection");
                }

                let trimmed = line_buf.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let resp: RpcResponse = match serde_json::from_str(trimmed) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                if resp.id != id {
                    continue;
                }

                if let Some(err) = resp.error {
                    anyhow::bail!("MCP error {}: {}", err.code, err.message);
                }

                let value = resp
                    .result
                    .ok_or_else(|| anyhow::anyhow!("MCP response missing result field"))?;

                let parsed: T = serde_json::from_value(value)?;
                break Ok::<_, anyhow::Error>(parsed);
            }
        })
        .await
        .context("MCP response timeout")?;

        inner
    }
}

/// A running MCP server instance. Methods are safe to call concurrently.
#[allow(dead_code)]
pub struct McpServer {
    pub name: String,
    pub config: McpServerConfig,
    inner: Mutex<McpServerInner>,
    process: Option<tokio::process::Child>,
}

impl McpServer {
    /// Spawn the MCP server process, perform initialize handshake, and discover tools.
    pub async fn start(name: &str, config: &McpServerConfig) -> Result<Self> {
        let mut cmd = tokio::process::Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());

        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn MCP server '{}'", name))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("MCP server '{}' stdin not available", name))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("MCP server '{}' stdout not available", name))?;

        let reader = BufReader::new(stdout);
        let mut inner = McpServerInner {
            stdin,
            reader,
            next_id: 1,
            tools: Vec::new(),
        };

        // Initialize: negotiate protocol version
        let init_result: InitializeResult = inner
            .exchange(
                "initialize",
                Some(serde_json::json!({
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "dipralix",
                        "version": "0.0.1"
                    }
                })),
            )
            .await
            .with_context(|| format!("MCP initialize handshake failed for '{}'", name))?;

        // Ensure the server supports tools
        if init_result.capabilities.get("tools").and_then(|t| t.as_object()).is_none() {
            eprintln!(
                "  ~ MCP server '{}' has no tools capability",
                name
            );
        }

        // List tools
        let list_result: ListToolsResult = inner
            .exchange("tools/list", None::<Value>)
            .await
            .with_context(|| format!("MCP tools/list failed for '{}'", name))?;

        inner.tools = list_result.tools.clone();

        eprintln!(
            "  {} MCP server '{}' ready — {} tools",
            "+".green(),
            name.cyan(),
            list_result.tools.len()
        );

        Ok(McpServer {
            name: name.to_string(),
            config: config.clone(),
            inner: Mutex::new(inner),
            process: Some(child),
        })
    }

    /// Call a tool on this MCP server.
    pub async fn call_tool(&self, tool_name: &str, args: Value) -> ToolResult {
        let mut inner = self.inner.lock().await;

        let result: CallToolResult = match inner
            .exchange(
                "tools/call",
                Some(serde_json::json!({
                    "name": tool_name,
                    "arguments": args
                })),
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return ToolResult::err(format!(
                    "MCP tool '{}/{}' call failed: {}",
                    self.name, tool_name, e
                ));
            }
        };

        if result.is_error {
            let msg = result
                .content
                .iter()
                .filter_map(|c| {
                    if let McpContent::Text { text } = c {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            return ToolResult::err(format!(
                "MCP tool '{}/{}' error: {}",
                self.name, tool_name, msg
            ));
        }

        let output = result
            .content
            .iter()
            .filter_map(|c| match c {
                McpContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        ToolResult::ok(if output.is_empty() {
            format!(
                "MCP tool '{}/{}' completed (no text output)",
                self.name, tool_name
            )
        } else {
            output
        })
    }

    /// Returns the list of tool declarations for this server.
    pub fn tool_names(&self) -> Vec<(String, McpToolDecl)> {
        let tools: Vec<_> = self
            .inner
            .try_lock()
            .map(|inner| inner.tools.clone())
            .unwrap_or_default();
        tools
            .into_iter()
            .map(|t| (format!("{}__{}", self.name, t.name), t))
            .collect()
    }

    /// Gracefully shut down the server.
    #[allow(dead_code)]
    pub async fn shutdown(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.start_kill();
        }
    }
}

// ── Registry ────────────────────────────────────────────────────────────────────

/// Manages all active MCP server connections.
pub struct McpRegistry {
    servers: Vec<McpServer>,
    /// Maps "server_name__tool_name" -> (server_index, tool_decl)
    tool_map: HashMap<String, (usize, McpToolDecl)>,
}

impl McpRegistry {
    /// Start all configured MCP servers and index their tools.
    pub async fn startup(configs: &HashMap<String, McpServerConfig>) -> Self {
        let mut servers = Vec::new();
        let mut concurrency = Vec::new();

        for (name, config) in configs {
            if config.disabled || !config.is_valid() {
                continue;
            }

            let name_clone = name.clone();
            let config_clone = config.clone();
            concurrency.push(async move {
                match McpServer::start(&name_clone, &config_clone).await {
                    Ok(server) => Some(server),
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to start MCP server '{}': {}",
                            "x".red(),
                            name_clone.red(),
                            e
                        );
                        None
                    }
                }
            });
        }

        // Start servers in parallel
        let results: Vec<Option<McpServer>> = futures_util::future::join_all(concurrency).await;
        for server in results.into_iter().flatten() {
            servers.push(server);
        }

        // Index all tools with server-name prefixed keys
        let mut tool_map = HashMap::new();
        for (idx, server) in servers.iter().enumerate() {
            for (prefixed_name, decl) in server.tool_names() {
                tool_map.insert(prefixed_name, (idx, decl));
            }
        }

        McpRegistry { servers, tool_map }
    }

    /// Convert all MCP tools into Gemini function declarations.
    pub fn function_declarations(&self) -> Vec<crate::types::FunctionDeclaration> {
        let mut decls = Vec::new();
        for (prefixed_name, (_idx, tool)) in &self.tool_map {
            let schema = if tool.input_schema.is_object() {
                let obj = tool.input_schema.as_object().unwrap();
                if obj.contains_key("properties") {
                    // Standard JSON Schema — wrap if needed
                    let mut wrapped = obj.clone();
                    let required = wrapped
                        .remove("required")
                        .unwrap_or(serde_json::json!([]));
                    let properties = wrapped
                        .remove("properties")
                        .unwrap_or(serde_json::json!({}));

                    serde_json::json!({
                        "type": "OBJECT",
                        "properties": properties,
                        "required": required
                    })
                } else {
                    serde_json::json!({
                        "type": "OBJECT",
                        "properties": obj,
                        "required": []
                    })
                }
            } else {
                serde_json::json!({
                    "type": "OBJECT",
                    "properties": {},
                    "required": []
                })
            };

            decls.push(crate::types::FunctionDeclaration {
                name: prefixed_name.clone(),
                description: format!(
                    "[MCP {}] {}",
                    self.server_name(prefixed_name),
                    tool.description
                ),
                parameters: schema,
            });
        }
        decls
    }

    /// Execute an MCP tool call by its prefixed name.
    pub async fn call_tool(
        &self,
        prefixed_name: &str,
        args: Value,
        _ctx: &ToolContext,
    ) -> ToolResult {
        let (idx, _decl) = match self.tool_map.get(prefixed_name) {
            Some(v) => v,
            None => {
                return ToolResult::err(format!(
                    "MCP tool not found: {}",
                    prefixed_name
                ))
            }
        };

        let server = &self.servers[*idx];

        // Extract the original tool name (strip server prefix)
        let server_prefix = format!("{}__", server.name);
        let tool_name = prefixed_name
            .strip_prefix(&server_prefix)
            .unwrap_or(prefixed_name);

        server.call_tool(tool_name, args).await
    }

    /// Extract the server name from a prefixed tool name.
    fn server_name(&self, prefixed: &str) -> &str {
        for server in &self.servers {
            let prefix = format!("{}__", server.name);
            if prefixed.starts_with(&prefix) {
                return &server.name;
            }
        }
        "unknown"
    }

    /// Number of active servers.
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Number of available MCP tools.
    #[allow(dead_code)]
    pub fn tool_count(&self) -> usize {
        self.tool_map.len()
    }

    /// Print MCP status to terminal.
    pub fn print_status(&self) {
        if self.servers.is_empty() {
            return;
        }
        println!("  {} MCP Servers:", "*".dimmed());
        for server in &self.servers {
            let count = server.tool_names().len();
            println!(
                "    {} {} — {} tools",
                "+".green(),
                server.name.cyan(),
                count
            );
        }
    }

    /// Shutdown all servers.
    #[allow(dead_code)]
    pub async fn shutdown_all(&mut self) {
        for server in &mut self.servers {
            server.shutdown().await;
        }
    }
}
