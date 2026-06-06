// Headless CI runner — runs FORGE non-interactively with JSON output.
// Suitable for GitHub Actions, CI pipelines, and automation.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::agent;
use crate::backend::BackendClient;
use crate::config::Config;

#[derive(Serialize)]
pub struct CiResult {
    pub success: bool,
    pub message: String,
    pub files_changed: Vec<String>,
    pub files_created: Vec<String>,
    pub total_tokens: u64,
    pub turns: u64,
}

pub async fn run_ci(config: &Config, prompt: &str) -> Result<CiResult> {
    let resolved = agent::resolve_auto_model(config, prompt);
    let client = BackendClient::new(&resolved)?;
    let result = agent::run_ci_agent(&client, &resolved, prompt).await?;

    Ok(result)
}

pub async fn run_pipeline(config: &Config, name: &str) -> Result<()> {
    let path = std::path::Path::new(".dipralix/pipelines").join(format!("{}.toml", name));
    if !path.exists() {
        return Err(anyhow::anyhow!(
            "Pipeline '{}' not found at {}. Create it first:\n  mkdir -p .dipralix/pipelines\n  dipralix-cli --pack .dipralix/pipelines/{}.toml\n\nThen add task steps to the file.",
            name, path.display(), name
        ));
    }

    let content = std::fs::read_to_string(&path)?;
    let pipeline: toml::Value = toml::from_str(&content).context("Invalid pipeline TOML")?;

    let steps = pipeline
        .get("steps")
        .and_then(|s| s.as_array())
        .context("Pipeline must have a [[steps]] array")?;

    for (i, step) in steps.iter().enumerate() {
        let task = step.get("task").and_then(|t| t.as_str()).unwrap_or("");
        let desc = step
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        if task.is_empty() {
            continue;
        }

        println!(
            "\n  {} Pipeline step {}/{}: {}",
            "[PIPELINE]".cyan(),
            i + 1,
            steps.len(),
            if desc.is_empty() { task } else { desc }.dimmed()
        );

        agent::run_once(config, task, None).await?;
    }

    println!(
        "\n  {} Pipeline '{}' complete — {} steps executed.",
        "[OK]".green(),
        name,
        steps.len()
    );
    Ok(())
}
