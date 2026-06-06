use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::PathBuf;

/// DIPRALIX Memory Core
/// Manages persistent project-level and global-level memory using Markdown files.
pub struct MemoryCore {
    project_dir: PathBuf,
}

impl MemoryCore {
    pub fn new() -> Self {
        Self {
            project_dir: PathBuf::from(".dipralix/memory"),
        }
    }

    /// Ensure memory directories exist.
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.project_dir)
            .context("Failed to create project memory directory")?;

        let global_dir = dirs::home_dir()
            .map(|h| h.join(".dipralix/patterns"))
            .context("Failed to resolve home directory")?;
        fs::create_dir_all(global_dir).context("Failed to create global patterns directory")?;

        Ok(())
    }

    /// Record a decision or important fact to project memory.
    pub fn record_decision(&self, decision: &str) -> Result<()> {
        self.init()?;
        let path = self.project_dir.join("decisions.md");
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("- [{}] {}\n", now, decision);

        let mut content =
            fs::read_to_string(&path).unwrap_or_else(|_| "# Project Decisions\n\n".to_string());
        content.push_str(&entry);

        fs::write(path, content).context("Failed to write decision to memory")
    }

    /// Record a learned pattern to global memory.
    pub fn record_pattern(&self, pattern_name: &str, content: &str) -> Result<()> {
        let global_dir = dirs::home_dir()
            .map(|h| h.join(".dipralix/patterns"))
            .context("Failed to resolve home directory")?;
        fs::create_dir_all(&global_dir)?;

        let path = global_dir.join(format!(
            "{}.md",
            pattern_name.replace(' ', "_").to_lowercase()
        ));
        fs::write(path, content).context("Failed to write pattern to global memory")
    }

    /// Load all project decisions as a context string.
    pub fn load_project_context(&self) -> String {
        let path = self.project_dir.join("decisions.md");
        if let Ok(content) = fs::read_to_string(path) {
            format!("\n## Project Memory (Decisions)\n\n{}\n", content)
        } else {
            String::new()
        }
    }

    /// Load relevant global patterns (stub for semantic search in future).
    pub fn load_global_patterns(&self) -> String {
        // For now, just load a general index or everything if small
        let global_dir = dirs::home_dir()
            .map(|h| h.join(".dipralix/patterns"))
            .unwrap_or_default();

        let mut patterns = String::new();
        if let Ok(entries) = fs::read_dir(global_dir) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    patterns.push_str(&format!(
                        "\n### {}\n\n{}",
                        entry.file_name().to_string_lossy(),
                        content
                    ));
                }
            }
        }

        if patterns.is_empty() {
            String::new()
        } else {
            format!("\n## Global Patterns & Learnings\n\n{}\n", patterns)
        }
    }
}
