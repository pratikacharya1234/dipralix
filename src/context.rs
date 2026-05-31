use std::path::PathBuf;
use std::fs;

/// DIPRALIX Lazy Context Assembler
/// Dynamically loads relevant skills and documentation based on project state.
pub struct ContextAssembler {
    global_skills_dir: PathBuf,
}

impl ContextAssembler {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            global_skills_dir: home.join(".dipralix/skills"),
        }
    }

    /// Load relevant skills based on detected technologies.
    pub fn assemble_skills(&self, dna: &crate::learning::ProjectDna) -> String {
        let mut context = String::new();
        
        // 1. Load project-specific skills from .dipralix/skills/
        if let Ok(entries) = fs::read_dir(".dipralix/skills") {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    context.push_str(&format!("\n## Project Skill: {}\n\n{}", 
                        entry.file_name().to_string_lossy(),
                        content));
                }
            }
        }

        // 2. Load global skills based on DNA detection
        let mut skills_to_load = Vec::new();
        if dna.language == "rust" {
            skills_to_load.push("rust/basics.md");
            if dna.framework.contains("axum") || dna.conventions.iter().any(|c| c.contains("axum")) {
                skills_to_load.push("rust/axum_best_practices.md");
            }
        } else if dna.language == "typescript" || dna.language == "javascript" {
            skills_to_load.push("js/basics.md");
        }

        for skill_path in skills_to_load {
            let path = self.global_skills_dir.join(skill_path);
            if let Ok(content) = fs::read_to_string(path) {
                context.push_str(&format!("\n## Global Skill: {}\n\n{}", 
                    skill_path,
                    content));
            }
        }

        context
    }
}
