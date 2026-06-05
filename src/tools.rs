use std::collections::HashMap;
use std::io::Write as _;
use std::sync::OnceLock;

use colored::Colorize;
use serde_json::{json, Value};
use tokio::io::AsyncBufReadExt as _;
use walkdir::WalkDir;

use crate::{
    audit, diff_view, integrations::IntegrationRegistry, mcp::McpRegistry, safety, snapshot,
    types::FunctionDeclaration,
};

// ── Context passed to every tool invocation ────────────────────────────────────

#[derive(Clone)]
pub struct ToolContext {
    /// Stream bash output live to the terminal (disable for parallel batches).
    pub stream_output: bool,
    /// Skip diff preview and auto-accept all file changes.
    pub auto_apply: bool,
    /// MCP (Model Context Protocol) server registry for external tools.
    pub mcp: Option<std::sync::Arc<McpRegistry>>,
    /// Integration registry for native service integrations (GitHub, Discord, Gmail, Drive).
    pub integrations: Option<std::sync::Arc<IntegrationRegistry>>,
}

// ── Result type ────────────────────────────────────────────────────────────────

pub struct ToolResult {
    pub output: String,
    pub is_error: bool,
    /// True when bash already streamed output live — agent should not re-print it.
    pub was_streamed: bool,
}

impl ToolResult {
    pub fn ok(s: impl Into<String>) -> Self {
        Self {
            output: s.into(),
            is_error: false,
            was_streamed: false,
        }
    }
    pub fn err(s: impl Into<String>) -> Self {
        Self {
            output: s.into(),
            is_error: true,
            was_streamed: false,
        }
    }
}

// ── Dispatcher ─────────────────────────────────────────────────────────────────

pub async fn execute_tool(name: &str, args: &Value, ctx: &ToolContext) -> ToolResult {
    // Route integration tools: github__..., discord__..., gdrive__..., gmail__...
    if name.starts_with("github__")
        || name.starts_with("discord__")
        || name.starts_with("gdrive__")
        || name.starts_with("gmail__")
    {
        if let Some(ref ireg) = ctx.integrations {
            return ireg.call_tool(name, args.clone(), ctx);
        }
        return ToolResult::err(format!(
            "Integration tool '{}' called but integrations are not configured",
            name
        ));
    }

    // MCP tools have names like "server__toolname"
    if name.contains("__") {
        if let Some(ref mcp) = ctx.mcp {
            return mcp.call_tool(name, args.clone(), ctx).await;
        }
        return ToolResult::err(format!(
            "MCP tool '{}' called but MCP is not configured",
            name
        ));
    }

    match name {
        "read_file" => tool_read_file(args),
        "write_file" => tool_write_file(args, ctx),
        "edit_file" => tool_edit_file(args, ctx),
        "append_file" => tool_append_file(args),
        "bash" => tool_bash(args, ctx).await,
        "list_files" => tool_list_files(args),
        "list_symbols" => tool_list_symbols(args),
        "search_files" => tool_search_files(args),
        "glob" => tool_glob(args),
        "create_directory" => tool_create_directory(args),
        "delete_file" => tool_delete_file(args).await,
        "move_file" => tool_move_file(args),
        "copy_file" => tool_copy_file(args),
        "url_fetch" => tool_url_fetch(args).await,
        "git_snapshot" => tool_git_snapshot(args).await,
        "memorize_decision" => tool_memorize_decision(args).await,
        "memorize_pattern" => tool_memorize_pattern(args).await,
        other => ToolResult::err(format!("Unknown tool: {other}")),
    }
}

// ── read_file ──────────────────────────────────────────────────────────────────

fn tool_read_file(args: &Value) -> ToolResult {
    let path = match args.get("path").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing required argument: path"),
    };
    let start_line = args.get("start_line").and_then(Value::as_u64).unwrap_or(1) as usize;
    let end_line = args
        .get("end_line")
        .and_then(Value::as_u64)
        .map(|n| n as usize);

    match std::fs::read_to_string(path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let total = lines.len();
            let from = start_line.saturating_sub(1).min(total);
            let to = end_line
                .map(|e| e.min(total))
                .unwrap_or(total.min(from + 500));

            let chunk = lines[from..to]
                .iter()
                .enumerate()
                .map(|(i, l)| format!("{:>4} {}", from + i + 1, l))
                .collect::<Vec<_>>()
                .join("\n");

            if to < total {
                ToolResult::ok(format!(
                    "{}\n\n[... {} more lines. Use start_line/end_line to paginate ...]",
                    chunk,
                    total - to
                ))
            } else {
                ToolResult::ok(chunk)
            }
        }
        Err(e) => ToolResult::err(format!("Cannot read '{}': {}", path, e)),
    }
}

// ── write_file ─────────────────────────────────────────────────────────────────

fn tool_write_file(args: &Value, ctx: &ToolContext) -> ToolResult {
    let path = match args.get("path").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing required argument: path"),
    };
    let content = match args.get("content").and_then(Value::as_str) {
        Some(c) => c,
        None => return ToolResult::err("Missing required argument: content"),
    };

    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::err(format!("Cannot create parent directory: {}", e));
            }
        }
    }

    // Show diff if the file already exists
    let existing = std::fs::read_to_string(path).ok();
    if let Some(ref old) = existing {
        snapshot::capture(path, "write_file");
        if !diff_view::show_and_confirm(path, old, content, ctx.auto_apply) {
            audit::log("write_file", path, false);
            return ToolResult::err(format!("Change to '{}' rejected by user.", path));
        }
    } else {
        // New file — capture empty so undo deletes it
        snapshot::capture(path, "write_file (new)");
    }

    match std::fs::write(path, content) {
        Ok(_) => {
            audit::log("write_file", path, true);
            ToolResult::ok(format!("Wrote {} bytes to '{}'", content.len(), path))
        }
        Err(e) => {
            audit::log("write_file", path, false);
            ToolResult::err(format!("Cannot write '{}': {}", path, e))
        }
    }
}

// ── append_file ────────────────────────────────────────────────────────────────

fn tool_append_file(args: &Value) -> ToolResult {
    let path = match args.get("path").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing required argument: path"),
    };
    let content = match args.get("content").and_then(Value::as_str) {
        Some(c) => c,
        None => return ToolResult::err("Missing required argument: content"),
    };

    snapshot::capture(path, "append_file");

    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut f) => match f.write_all(content.as_bytes()) {
            Ok(_) => {
                audit::log("append_file", path, true);
                ToolResult::ok(format!("Appended {} bytes to '{}'", content.len(), path))
            }
            Err(e) => ToolResult::err(format!("Cannot write to '{}': {}", path, e)),
        },
        Err(e) => ToolResult::err(format!("Cannot open '{}' for appending: {}", path, e)),
    }
}

// ── edit_file ──────────────────────────────────────────────────────────────────

fn tool_edit_file(args: &Value, ctx: &ToolContext) -> ToolResult {
    let path = match args.get("path").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing: path"),
    };
    let old_str = match args.get("old_str").and_then(Value::as_str) {
        Some(s) => s,
        None => return ToolResult::err("Missing: old_str"),
    };
    let new_str = match args.get("new_str").and_then(Value::as_str) {
        Some(s) => s,
        None => return ToolResult::err("Missing: new_str"),
    };
    let occurrence = args.get("occurrence").and_then(Value::as_u64).unwrap_or(1) as usize;

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return ToolResult::err(format!("Cannot read '{}': {}", path, e)),
    };

    // Try exact match first
    let exact_count = content.matches(old_str).count();
    if exact_count > 0 {
        if occurrence == 0 || occurrence > exact_count {
            return ToolResult::err(format!(
                "Found {} occurrences in '{}'. occurrence={}, but only {} exist. Use 0 for all or 1-{} for a specific one.",
                exact_count, path, occurrence, exact_count, exact_count
            ));
        }

        let updated = if occurrence == 0 {
            content.replace(old_str, new_str)
        } else {
            // Replace the Nth occurrence
            let mut pos = 0usize;
            let mut found = 0usize;
            for _ in 0..occurrence {
                if let Some(p) = content[pos..].find(old_str) {
                    pos += p;
                    found += 1;
                    if found == occurrence {
                        break;
                    }
                    pos += old_str.len();
                } else {
                    break;
                }
            }
            if found != occurrence {
                return ToolResult::err(format!(
                    "Could not find occurrence {} of '{}' in '{}'",
                    occurrence, old_str, path
                ));
            }
            let mut updated = content[..pos].to_string();
            updated.push_str(new_str);
            updated.push_str(&content[pos + old_str.len()..]);
            updated
        };

        snapshot::capture(path, "edit_file");
        if !diff_view::show_and_confirm(path, &content, &updated, ctx.auto_apply) {
            audit::log("edit_file", path, false);
            return ToolResult::err(format!("Edit to '{}' rejected by user.", path));
        }

        match std::fs::write(path, &updated) {
            Ok(_) => {
                audit::log("edit_file", path, true);
                ToolResult::ok(format!("Edited '{}'", path))
            }
            Err(e) => ToolResult::err(format!("Cannot write '{}': {}", path, e)),
        }
    } else {
        // Fuzzy fallback: whitespace-normalized matching
        let normalize =
            |s: &str| -> String { s.lines().map(|l| l.trim()).collect::<Vec<_>>().join("\n") };
        let normalized_old = normalize(old_str);
        let normalized_content = normalize(&content);

        if let Some(_pos) = normalized_content.find(&normalized_old) {
            // Fall back to line-based fuzzy matching
            let old_lines: Vec<&str> = old_str.lines().map(|l| l.trim()).collect();
            let content_lines: Vec<&str> = content.lines().collect();
            let mut fuzzy_pos = None;

            'outer: for (i, window) in content_lines.windows(old_lines.len()).enumerate() {
                let window_trimmed: Vec<&str> = window.iter().map(|l| l.trim()).collect();
                if window_trimmed == old_lines {
                    fuzzy_pos = Some(i);
                    break 'outer;
                }
            }

            if let Some(start_line) = fuzzy_pos {
                let original_match: String =
                    content_lines[start_line..start_line + old_lines.len()].join("\n");

                let updated = content.replacen(&original_match, new_str, 1);

                snapshot::capture(path, "edit_file");
                if !diff_view::show_and_confirm(path, &content, &updated, ctx.auto_apply) {
                    audit::log("edit_file", path, false);
                    return ToolResult::err(format!("Edit to '{}' rejected by user.", path));
                }

                match std::fs::write(path, &updated) {
                    Ok(_) => {
                        audit::log("edit_file", path, true);
                        ToolResult::ok(format!(
                            "Edited '{}' (fuzzy: matched by lines ignoring leading whitespace)",
                            path
                        ))
                    }
                    Err(e) => ToolResult::err(format!("Cannot write '{}': {}", path, e)),
                }
            } else {
                ToolResult::err(format!(
                    "String not found in '{}' (exact or fuzzy). Ensure whitespace matches, or re-read the file first.",
                    path
                ))
            }
        } else {
            ToolResult::err(format!(
                "String not found in '{}' (exact or fuzzy). Ensure it matches, or re-read the file first.",
                path
            ))
        }
    }
}

// ── bash ───────────────────────────────────────────────────────────────────────

async fn tool_bash(args: &Value, ctx: &ToolContext) -> ToolResult {
    let command = match args.get("command").and_then(Value::as_str) {
        Some(c) => c,
        None => return ToolResult::err("Missing required argument: command"),
    };
    let timeout_secs = args
        .get("timeout")
        .and_then(Value::as_u64)
        .unwrap_or(120)
        .min(600);

    // Safety check — runs synchronous stdin via block_in_place
    let cmd_owned = command.to_string();
    let allowed = tokio::task::block_in_place(|| safety::check_bash(&cmd_owned));
    if !allowed {
        audit::log("bash", command, false);
        return ToolResult::err(format!("Command blocked by safety policy: {}", command));
    }

    audit::log("bash", command, true);

    let wrapped = format!("({}) 2>&1", command);

    let mut child = match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&wrapped)
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return ToolResult::err(format!("Failed to spawn command: {}", e)),
    };

    let stdout = child.stdout.take().expect("stdout piped");
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    let mut all_lines: Vec<String> = Vec::new();
    let mut displayed = 0usize;
    const MAX_DISPLAY: usize = 200;
    const MAX_COLLECT: usize = 2000;

    let timed_out = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async {
        while let Ok(Some(line)) = reader.next_line().await {
            if ctx.stream_output && displayed < MAX_DISPLAY {
                println!("  {} {}", "│".bright_black(), line.dimmed());
                let _ = std::io::stdout().flush();
                displayed += 1;
            }
            if all_lines.len() < MAX_COLLECT {
                all_lines.push(line);
            }
        }
    })
    .await
    .is_err();

    if timed_out {
        let _ = child.kill().await;
        return ToolResult::err(format!("Command timed out after {}s", timeout_secs));
    }

    let status = child.wait().await;
    let succeeded = status.map(|s| s.success()).unwrap_or(false);
    let output = if all_lines.is_empty() {
        format!("(exit {})", if succeeded { 0 } else { 1 })
    } else {
        all_lines.join("\n")
    };

    ToolResult {
        output,
        is_error: !succeeded,
        was_streamed: ctx.stream_output,
    }
}

// ── list_files ─────────────────────────────────────────────────────────────────

fn tool_list_files(args: &Value) -> ToolResult {
    let path = args.get("path").and_then(Value::as_str).unwrap_or(".");
    let recursive = args
        .get("recursive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_depth = if recursive { usize::MAX } else { 1 };

    let mut lines = Vec::new();

    for entry in WalkDir::new(path)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let depth = entry.depth();
        if depth == 0 {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if matches!(
            name.as_str(),
            "target" | "node_modules" | "__pycache__" | ".git"
        ) {
            continue;
        }

        let indent = "  ".repeat(depth - 1);
        let suffix = if entry.file_type().is_dir() { "/" } else { "" };
        lines.push(format!("{}{}{}", indent, name, suffix));
    }

    if lines.is_empty() {
        ToolResult::ok(format!("(empty: {})", path))
    } else {
        ToolResult::ok(lines.join("\n"))
    }
}

// ── list_symbols ───────────────────────────────────────────────────────────────

fn tool_list_symbols(args: &Value) -> ToolResult {
    let path = match args.get("path").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing required argument: path"),
    };

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return ToolResult::err(format!("Cannot read '{}': {}", path, e)),
    };

    // Use our new ast module
    match crate::ast::extract_symbols(&content) {
        Ok(symbols) => {
            if symbols.is_empty() {
                ToolResult::ok(format!("No symbols found in '{}'", path))
            } else {
                let mut output = Vec::new();
                for s in symbols {
                    output.push(format!(
                        "{:<15} {:<30} L{}-L{}",
                        s.kind, s.name, s.start_line, s.end_line
                    ));
                }
                ToolResult::ok(output.join("\n"))
            }
        }
        Err(e) => ToolResult::err(format!("Failed to extract symbols: {}", e)),
    }
}

// ── search_files ───────────────────────────────────────────────────────────────

fn tool_search_files(args: &Value) -> ToolResult {
    let pattern = match args.get("pattern").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing required argument: pattern"),
    };
    let path = args.get("path").and_then(Value::as_str).unwrap_or(".");
    let case_insensitive = args
        .get("case_insensitive")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let re = match regex::RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
    {
        Ok(r) => r,
        Err(e) => return ToolResult::err(format!("Invalid regex '{}': {}", pattern, e)),
    };

    let skip_exts = [
        "png", "jpg", "jpeg", "gif", "ico", "svg", "webp", "pdf", "zip", "tar", "gz", "exe", "bin",
        "so", "dll", "dylib", "wasm", "lock",
    ];

    let mut matches = Vec::new();
    let mut files_checked = 0usize;

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let fp = entry.path();
        let fp_str = fp.to_string_lossy();

        if fp_str.contains("/target/")
            || fp_str.contains("/node_modules/")
            || fp_str.contains("/.git/")
            || fp_str.contains("/__pycache__/")
        {
            continue;
        }

        let ext = fp.extension().and_then(|e| e.to_str()).unwrap_or("");
        if skip_exts.contains(&ext) {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(fp) {
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    matches.push(format!("{}:{}: {}", fp_str, i + 1, line.trim()));
                    if matches.len() >= 100 {
                        return ToolResult::ok(format!(
                            "{}\n[... truncated at 100 matches ...]",
                            matches.join("\n")
                        ));
                    }
                }
            }
        }

        files_checked += 1;
        if files_checked > 2000 {
            break;
        }
    }

    if matches.is_empty() {
        ToolResult::ok(format!("No matches for '{}' in '{}'", pattern, path))
    } else {
        ToolResult::ok(matches.join("\n"))
    }
}

// ── glob ───────────────────────────────────────────────────────────────────────

fn tool_glob(args: &Value) -> ToolResult {
    let pattern = match args.get("pattern").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing required argument: pattern"),
    };

    match glob::glob(pattern) {
        Err(e) => ToolResult::err(format!("Invalid glob pattern '{}': {}", pattern, e)),
        Ok(paths) => {
            let mut results: Vec<String> = paths
                .filter_map(|r| r.ok())
                .map(|p| p.display().to_string())
                .filter(|p| !p.contains("/target/") && !p.contains("/node_modules/"))
                .take(500)
                .collect();
            results.sort();
            if results.is_empty() {
                ToolResult::ok(format!("No files matched '{}'", pattern))
            } else {
                ToolResult::ok(results.join("\n"))
            }
        }
    }
}

// ── create_directory ───────────────────────────────────────────────────────────

fn tool_create_directory(args: &Value) -> ToolResult {
    let path = match args.get("path").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing required argument: path"),
    };
    match std::fs::create_dir_all(path) {
        Ok(_) => ToolResult::ok(format!("Created directory '{}'", path)),
        Err(e) => ToolResult::err(format!("Cannot create directory '{}': {}", path, e)),
    }
}

// ── delete_file ────────────────────────────────────────────────────────────────

async fn tool_delete_file(args: &Value) -> ToolResult {
    let path = match args.get("path").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing required argument: path"),
    };

    let path_owned = path.to_string();
    let allowed = tokio::task::block_in_place(|| safety::check_delete(&path_owned));
    if !allowed {
        audit::log("delete_file", path, false);
        return ToolResult::err(format!("Delete of '{}' cancelled.", path));
    }

    // Snapshot before delete so /undo can restore
    snapshot::capture(path, "delete_file");

    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return ToolResult::err(format!("Cannot access '{}': {}", path, e)),
    };

    let result = if meta.is_dir() {
        std::fs::remove_dir_all(path)
            .map(|_| format!("Deleted directory '{}'", path))
            .map_err(|e| format!("Cannot delete directory '{}': {}", path, e))
    } else {
        std::fs::remove_file(path)
            .map(|_| format!("Deleted file '{}'", path))
            .map_err(|e| format!("Cannot delete file '{}': {}", path, e))
    };

    match result {
        Ok(msg) => {
            audit::log("delete_file", path, true);
            ToolResult::ok(msg)
        }
        Err(msg) => {
            audit::log("delete_file", path, false);
            ToolResult::err(msg)
        }
    }
}

// ── move_file ──────────────────────────────────────────────────────────────────

fn tool_move_file(args: &Value) -> ToolResult {
    let src = match args.get("source").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing: source"),
    };
    let dst = match args.get("destination").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing: destination"),
    };

    if let Some(parent) = std::path::Path::new(dst).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    match std::fs::rename(src, dst) {
        Ok(_) => ToolResult::ok(format!("Moved '{}' → '{}'", src, dst)),
        Err(e) => ToolResult::err(format!("Cannot move '{}' to '{}': {}", src, dst, e)),
    }
}

// ── copy_file ──────────────────────────────────────────────────────────────────

fn tool_copy_file(args: &Value) -> ToolResult {
    let src = match args.get("source").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing: source"),
    };
    let dst = match args.get("destination").and_then(Value::as_str) {
        Some(p) => p,
        None => return ToolResult::err("Missing: destination"),
    };

    if let Some(parent) = std::path::Path::new(dst).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    match std::fs::copy(src, dst) {
        Ok(bytes) => ToolResult::ok(format!("Copied '{}' → '{}' ({} bytes)", src, dst, bytes)),
        Err(e) => ToolResult::err(format!("Cannot copy '{}' to '{}': {}", src, dst, e)),
    }
}

// ── url_fetch ──────────────────────────────────────────────────────────────────

// Simple in-memory URL cache: url → (content, fetched_at)
static URL_CACHE: OnceLock<tokio::sync::Mutex<HashMap<String, (String, std::time::Instant)>>> =
    OnceLock::new();
fn url_cache() -> &'static tokio::sync::Mutex<HashMap<String, (String, std::time::Instant)>> {
    URL_CACHE.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

async fn tool_url_fetch(args: &Value) -> ToolResult {
    let url = match args.get("url").and_then(Value::as_str) {
        Some(u) => u,
        None => return ToolResult::err("Missing required argument: url"),
    };
    let max_len = args
        .get("max_length")
        .and_then(Value::as_u64)
        .unwrap_or(32_000) as usize;
    let max_len = max_len.min(128_000);

    // Check cache
    {
        let cache = url_cache().lock().await;
        if let Some((cached, at)) = cache.get(url) {
            if at.elapsed() < CACHE_TTL {
                let truncated = truncate_chars(cached, max_len);
                return ToolResult::ok(format!("[cached]\n{}", truncated));
            }
        }
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (compatible; dipralix/1.0)")
        .build()
    {
        Ok(c) => c,
        Err(e) => return ToolResult::err(format!("HTTP client error: {}", e)),
    };

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => return ToolResult::err(format!("Request failed for '{}': {}", url, e)),
    };

    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = match resp.text().await {
        Ok(t) => t,
        Err(e) => return ToolResult::err(format!("Failed to read body: {}", e)),
    };

    let text = if content_type.contains("html") {
        strip_html(&body)
    } else {
        body
    };

    // Store in cache
    {
        let mut cache = url_cache().lock().await;
        cache.insert(url.to_string(), (text.clone(), std::time::Instant::now()));
    }

    let truncated = truncate_chars(&text, max_len);

    if status.is_success() {
        ToolResult::ok(truncated)
    } else {
        ToolResult::err(format!("HTTP {} from '{}'\n{}", status, url, truncated))
    }
}

fn truncate_chars(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut = s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}\n[... truncated ...]", &s[..cut])
    }
}

fn strip_html(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    let no_tags = re.replace_all(html, " ");
    let ws = regex::Regex::new(r"\s{2,}").unwrap();
    ws.replace_all(&no_tags, "\n").trim().to_string()
}

// ── git_snapshot ───────────────────────────────────────────────────────────────

async fn tool_git_snapshot(args: &Value) -> ToolResult {
    let name = args
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("dipralix-auto");

    let out = tokio::process::Command::new("git")
        .args(["stash", "push", "-m", name, "--include-untracked"])
        .output()
        .await;

    match out {
        Ok(o) if o.status.success() => {
            let msg = String::from_utf8_lossy(&o.stdout).trim().to_string();
            ToolResult::ok(format!("Snapshot '{}': {}", name, msg))
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
            ToolResult::err(format!("git stash failed: {}", err))
        }
        Err(e) => ToolResult::err(format!("git not available: {}", e)),
    }
}

// ── memorize_decision ──────────────────────────────────────────────────────────

async fn tool_memorize_decision(args: &Value) -> ToolResult {
    let decision = match args.get("decision").and_then(Value::as_str) {
        Some(d) => d,
        None => return ToolResult::err("Missing 'decision' argument"),
    };

    let memory = crate::memory::MemoryCore::new();
    match memory.record_decision(decision) {
        Ok(_) => ToolResult::ok(format!("Memorized project decision: {}", decision)),
        Err(e) => ToolResult::err(format!("Failed to memorize decision: {}", e)),
    }
}

// ── memorize_pattern ───────────────────────────────────────────────────────────

async fn tool_memorize_pattern(args: &Value) -> ToolResult {
    let name = match args.get("name").and_then(Value::as_str) {
        Some(n) => n,
        None => return ToolResult::err("Missing 'name' argument"),
    };
    let content = match args.get("content").and_then(Value::as_str) {
        Some(c) => c,
        None => return ToolResult::err("Missing 'content' argument"),
    };

    let memory = crate::memory::MemoryCore::new();
    match memory.record_pattern(name, content) {
        Ok(_) => ToolResult::ok(format!("Memorized global pattern: {}", name)),
        Err(e) => ToolResult::err(format!("Failed to memorize pattern: {}", e)),
    }
}

// ── Gemini function declarations ───────────────────────────────────────────────

pub fn core_tool_count() -> usize {
    get_tool_declarations().len()
}

pub fn get_tool_declarations() -> Vec<FunctionDeclaration> {
    vec![
        FunctionDeclaration {
            name: "read_file".to_string(),
            description: "Read a file with line numbers. Supports start_line/end_line pagination (500 lines shown by default).".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "path":       { "type": "STRING",  "description": "File path" },
                    "start_line": { "type": "INTEGER", "description": "First line (1-indexed, default 1)" },
                    "end_line":   { "type": "INTEGER", "description": "Last line (default start+500)" }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "write_file".to_string(),
            description: "Write or overwrite a file. Shows a diff preview for existing files.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "path":    { "type": "STRING", "description": "Destination file path" },
                    "content": { "type": "STRING", "description": "Full file content" }
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "append_file".to_string(),
            description: "Append text to the end of a file.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "path":    { "type": "STRING", "description": "File path" },
                    "content": { "type": "STRING", "description": "Text to append" }
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "edit_file".to_string(),
            description: "Replace text in a file. Supports exact matching with occurrence control, and whitespace-normalized fuzzy matching as fallback. Shows a diff preview.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "path":       { "type": "STRING", "description": "File path" },
                    "old_str":    { "type": "STRING", "description": "Text to replace (exact match, or fuzzy by trimmed lines)" },
                    "new_str":    { "type": "STRING", "description": "Replacement text" },
                    "occurrence": { "type": "INTEGER", "description": "Which occurrence to replace (1-based, default 1, 0 = replace all)" }
                },
                "required": ["path", "old_str", "new_str"]
            }),
        },
        FunctionDeclaration {
            name: "bash".to_string(),
            description: "Run a shell command. Output streams live. Destructive commands require user confirmation.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "command": { "type": "STRING",  "description": "Shell command" },
                    "timeout": { "type": "INTEGER", "description": "Max seconds (default 120, max 600)" }
                },
                "required": ["command"]
            }),
        },
        FunctionDeclaration {
            name: "list_files".to_string(),
            description: "List directory contents. Skips hidden dirs and build artifacts.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "path":      { "type": "STRING",  "description": "Directory (default .)" },
                    "recursive": { "type": "BOOLEAN", "description": "Recurse (default false)" }
                }
            }),
        },
        FunctionDeclaration {
            name: "list_symbols".to_string(),
            description: "Extract high-level symbols (functions, structs, enums) from a file using tree-sitter.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "path": { "type": "STRING", "description": "File path" }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "search_files".to_string(),
            description: "Regex search file contents. Returns file:line:content (up to 100 results).".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "pattern":          { "type": "STRING",  "description": "Regex pattern" },
                    "path":             { "type": "STRING",  "description": "Directory (default .)" },
                    "case_insensitive": { "type": "BOOLEAN", "description": "Case-insensitive (default false)" }
                },
                "required": ["pattern"]
            }),
        },
        FunctionDeclaration {
            name: "glob".to_string(),
            description: "Find files by glob pattern (e.g. 'src/**/*.rs'). Returns sorted list up to 500.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "pattern": { "type": "STRING", "description": "Glob pattern" }
                },
                "required": ["pattern"]
            }),
        },
        FunctionDeclaration {
            name: "create_directory".to_string(),
            description: "Create a directory tree.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "path": { "type": "STRING", "description": "Directory path to create" }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "delete_file".to_string(),
            description: "Delete a file or directory. Requires user confirmation.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "path": { "type": "STRING", "description": "Path to delete" }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "move_file".to_string(),
            description: "Move or rename a file or directory.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "source":      { "type": "STRING", "description": "Source path" },
                    "destination": { "type": "STRING", "description": "Destination path" }
                },
                "required": ["source", "destination"]
            }),
        },
        FunctionDeclaration {
            name: "copy_file".to_string(),
            description: "Copy a file to a new location.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "source":      { "type": "STRING", "description": "Source path" },
                    "destination": { "type": "STRING", "description": "Destination path" }
                },
                "required": ["source", "destination"]
            }),
        },
        FunctionDeclaration {
            name: "url_fetch".to_string(),
            description: "Fetch URL content. HTML is converted to plain text. Default limit 32KB, max 128KB. Results are cached for 1 hour.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "url":        { "type": "STRING",  "description": "Full URL (http/https)" },
                    "max_length": { "type": "INTEGER", "description": "Max chars (default 32000)" }
                },
                "required": ["url"]
            }),
        },
        FunctionDeclaration {
            name: "git_snapshot".to_string(),
            description: "Create a git stash snapshot of current changes so they can be rolled back.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "name": { "type": "STRING", "description": "Snapshot label (default: dipralix-auto)" }
                }
            }),
        },
        FunctionDeclaration {
            name: "memorize_decision".to_string(),
            description: "Record an architectural decision or important project fact to persistent project memory.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "decision": { "type": "STRING", "description": "The decision or fact to record" }
                },
                "required": ["decision"]
            }),
        },
        FunctionDeclaration {
            name: "memorize_pattern".to_string(),
            description: "Record a learned coding pattern or convention to global memory across projects.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "name":    { "type": "STRING", "description": "Short unique name for the pattern (e.g. 'rust_error_handling')" },
                    "content": { "type": "STRING", "description": "The pattern description in Markdown" }
                },
                "required": ["name", "content"]
            }),
        },
    ]
}
