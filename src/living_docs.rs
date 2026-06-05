use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::backend::BackendClient;
use crate::config::Config;

pub struct LivingDocs {
    client: std::sync::Arc<BackendClient>,
    config: Config,
}

impl LivingDocs {
    pub fn new(client: std::sync::Arc<BackendClient>, config: Config) -> Self {
        Self { client, config }
    }

    pub async fn sync_docs(&self) -> Result<()> {
        println!(
            "\n  {} Analyzing project structure to sync Living Documentation...",
            "[DOCS]".cyan()
        );

        // Check if ARCHITECTURE.md exists
        let has_arch = Path::new("ARCHITECTURE.md").exists();

        let prompt = if has_arch {
            let current = fs::read_to_string("ARCHITECTURE.md").unwrap_or_default();
            format!(
                "You are updating the ARCHITECTURE.md file for this project.\n\n\
                Current content:\n{}\n\n\
                Scan the codebase context and suggest updates to the architecture document. Output ONLY the complete, updated markdown content for ARCHITECTURE.md.",
                current
            )
        } else {
            "You are creating a new ARCHITECTURE.md file for this project.\n\n\
            Scan the codebase context and generate a comprehensive architecture document, including a Mermaid.js diagram of the high-level components. Output ONLY the complete markdown content for ARCHITECTURE.md.".to_string()
        };

        println!("  {} Generating ARCHITECTURE.md updates...", "→".dimmed());
        let res = crate::agent::run_ci_agent(&self.client, &self.config, &prompt).await?;

        let mut content = res.message.trim().to_string();

        // Strip markdown code blocks if the model wrapped the whole response
        if content.starts_with("```markdown") {
            content = content.trim_start_matches("```markdown").to_string();
            content = content.trim_end_matches("```").trim().to_string();
        } else if content.starts_with("```") {
            content = content.trim_start_matches("```").to_string();
            content = content.trim_end_matches("```").trim().to_string();
        }

        fs::write("ARCHITECTURE.md", content)?;
        println!("  {} Updated ARCHITECTURE.md", "[OK]".green());

        Ok(())
    }
}
