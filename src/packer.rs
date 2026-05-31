// Project context packer — creates a portable .dipralix-pack file with everything
// an AI model needs to understand a project in one shot.
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn pack_project(output: Option<&str>) -> Result<String> {
    let out_path = output.unwrap_or(".dipralix-pack");
    let mut buf = String::new();

    // Header
    let project_name = Path::new(".")
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    buf.push_str(&format!("# Project Context: {}\n", project_name));
    buf.push_str(&format!("Generated: {}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M")));

    // Git info
    if let Ok(output) = Command::new("git").args(["log", "--oneline", "-10"]).output() {
        let log = String::from_utf8_lossy(&output.stdout);
        if !log.trim().is_empty() {
            buf.push_str("## Recent Git History\n```\n");
            buf.push_str(&log);
            buf.push_str("```\n\n");
        }
    }

    if let Ok(output) = Command::new("git").args(["remote", "get-url", "origin"]).output() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !url.is_empty() {
            buf.push_str(&format!("**Remote:** {}\n\n", url));
        }
    }

    // File tree
    buf.push_str("## Project Structure\n```\n");
    pack_tree(".", &mut buf, 0, 3)?; // max depth 3
    buf.push_str("```\n\n");

    // Key files
    let key_files = [
        "README.md", "package.json", "Cargo.toml", "pyproject.toml",
        "Makefile", "Dockerfile", ".gitignore", "docker-compose.yml",
        "tsconfig.json", "go.mod", "Gemfile", "requirements.txt",
        "src/main.rs", "src/main.go", "src/index.ts", "src/app.py",
    ];

    buf.push_str("## Key Files\n\n");
    for file in &key_files {
        if Path::new(file).exists() {
            if let Ok(content) = fs::read_to_string(file) {
                let truncated = if content.lines().count() > 100 {
                    let short: String = content.lines().take(100).collect::<Vec<_>>().join("\n");
                    format!("{}\n... (truncated, {} total lines)", short, content.lines().count())
                } else {
                    content
                };
                buf.push_str(&format!("### {}\n```\n{}\n```\n\n", file, truncated));
            }
        }
    }

    // Dependencies
    if Path::new("Cargo.toml").exists() {
        buf.push_str("## Rust Dependencies\n```toml\n");
        if let Ok(content) = fs::read_to_string("Cargo.toml") {
            buf.push_str(&content);
        }
        buf.push_str("\n```\n\n");
    } else if Path::new("package.json").exists() {
        buf.push_str("## Dependencies (package.json)\n```json\n");
        if let Ok(content) = fs::read_to_string("package.json") {
            buf.push_str(&truncate_json(&content, 80));
        }
        buf.push_str("\n```\n\n");
    }

    fs::write(out_path, &buf)?;
    let size = fs::metadata(out_path)?.len();
    Ok(format!("Packed project context → {} ({:.1}KB)", out_path, size as f64 / 1024.0))
}

fn pack_tree(dir: &str, buf: &mut String, depth: usize, max_depth: usize) -> Result<()> {
    if depth > max_depth { return Ok(()); }
    let skip_dirs = ["target", "node_modules", ".git", "dist", "__pycache__", ".venv", "venv", "build"];

    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| !skip_dirs.contains(&e.file_name().to_string_lossy().as_ref()))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let indent = "  ".repeat(depth);
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if is_dir {
            buf.push_str(&format!("{}{}/\n", indent, name));
            let path = format!("{}/{}", dir, name);
            let _ = pack_tree(&path, buf, depth + 1, max_depth);
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            buf.push_str(&format!("{}{}  ({}B)\n", indent, name, size));
        }
    }
    Ok(())
}

fn truncate_json(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= max_lines {
        content.to_string()
    } else {
        format!("{}\n... ({} total lines)", lines[..max_lines].join("\n"), lines.len())
    }
}
