//! Approval Matrix — granular per-action approval policy.
//!
//! Inspired by Cline's approval-first workflow. Four levels:
//! Auto, Notify, Confirm, Deny. Stored in `.dipralix/approval.toml`.

use colored::Colorize;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalLevel {
    Auto,
    Notify,
    Confirm,
    Deny,
}

impl ApprovalLevel {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "notify" => Some(Self::Notify),
            "confirm" => Some(Self::Confirm),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }

    fn label(&self) -> colored::ColoredString {
        match self {
            Self::Auto => "Auto".green(),
            Self::Notify => "Notify".cyan(),
            Self::Confirm => "Confirm".yellow(),
            Self::Deny => "Deny".red(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApprovalMatrix {
    pub actions: HashMap<String, ApprovalLevel>,
}

impl Default for ApprovalMatrix {
    fn default() -> Self {
        let mut actions = HashMap::new();
        actions.insert("read_file".into(),      ApprovalLevel::Auto);
        actions.insert("write_file".into(),     ApprovalLevel::Notify);
        actions.insert("edit_file".into(),      ApprovalLevel::Confirm);
        actions.insert("bash".into(),           ApprovalLevel::Confirm);
        actions.insert("bash_rm".into(),        ApprovalLevel::Deny);
        actions.insert("bash_git_push".into(),  ApprovalLevel::Deny);
        actions.insert("bash_docker_run".into(),ApprovalLevel::Confirm);
        actions.insert("bash_curl".into(),      ApprovalLevel::Confirm);
        actions.insert("bash_sudo".into(),      ApprovalLevel::Deny);
        Self { actions }
    }
}

static MATRIX: OnceLock<RwLock<ApprovalMatrix>> = OnceLock::new();

fn matrix() -> &'static RwLock<ApprovalMatrix> {
    MATRIX.get_or_init(|| RwLock::new(load_from_disk()))
}

fn load_from_disk() -> ApprovalMatrix {
    let path = std::path::Path::new(".dipralix/approval.toml");
    let Ok(content) = std::fs::read_to_string(path) else {
        return ApprovalMatrix::default();
    };

    #[derive(serde::Deserialize)]
    struct Raw {
        actions: Option<HashMap<String, String>>,
    }

    let Ok(raw) = toml::from_str::<Raw>(&content) else {
        return ApprovalMatrix::default();
    };

    let mut m = ApprovalMatrix::default();
    if let Some(actions) = raw.actions {
        for (k, v) in actions {
            if let Some(level) = ApprovalLevel::from_str(&v) {
                m.actions.insert(k, level);
            }
        }
    }
    m
}

/// Classify an action by name. Falls back to Auto for unknown actions.
#[allow(dead_code)]
pub fn level_for(action: &str) -> ApprovalLevel {
    matrix().read().unwrap().actions.get(action).copied().unwrap_or(ApprovalLevel::Auto)
}

/// Classify a bash command into a more specific action subtype.
#[allow(dead_code)]
pub fn classify_bash(cmd: &str) -> ApprovalLevel {
    let c = cmd.trim().to_lowercase();
    if c.contains("sudo ")            { return level_for("bash_sudo"); }
    if c.contains("git push")         { return level_for("bash_git_push"); }
    if c.starts_with("rm ") || c.contains(" rm -") { return level_for("bash_rm"); }
    if c.contains("docker run")       { return level_for("bash_docker_run"); }
    if c.starts_with("curl ") || c.contains(" curl ") { return level_for("bash_curl"); }
    level_for("bash")
}

pub fn print_matrix() {
    let m = matrix().read().unwrap();
    println!("\n  {}", "Approval Matrix".cyan().bold());
    let mut entries: Vec<_> = m.actions.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (action, level) in entries {
        println!("    {:<22} {}", action.dimmed(), level.label());
    }
    println!("\n  {} {}", "Source:".dimmed(), ".dipralix/approval.toml (or defaults)".dimmed());
}

pub fn set_speed_fast() {
    let mut m = matrix().write().unwrap();
    for level in m.actions.values_mut() {
        if *level != ApprovalLevel::Deny {
            *level = ApprovalLevel::Auto;
        }
    }
    println!("  {} Speed: {} (all non-Deny actions auto-approved)", "[OK]".green(), "FAST".green().bold());
}

pub fn set_speed_safe() {
    let mut m = matrix().write().unwrap();
    for level in m.actions.values_mut() {
        if *level == ApprovalLevel::Auto {
            *level = ApprovalLevel::Confirm;
        }
    }
    println!("  {} Speed: {} (all Auto actions now require Confirm)", "[OK]".green(), "SAFE".yellow().bold());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_bash_sudo() {
        assert_eq!(classify_bash("sudo apt update"), ApprovalLevel::Deny);
    }

    #[test]
    fn classify_bash_default() {
        assert_eq!(classify_bash("ls -la"), ApprovalLevel::Confirm);
    }

    #[test]
    fn unknown_action_is_auto() {
        assert_eq!(level_for("nonexistent_action"), ApprovalLevel::Auto);
    }
}
