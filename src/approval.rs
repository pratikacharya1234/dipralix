//! Approval Matrix — granular per-action approval policy.
//!
//! Inspired by Cline's approval-first workflow. Four levels:
//! Auto, Notify, Confirm, Deny. Stored in `.dipralix/approval.toml`.
//!
//! In Phase 2 the policy is also team-shared: a `Confirm` action
//! can declare `required_approvers = N` and the votes are
//! collected over the realtime sync channel. See
//! [`TeamPolicy::required_approvers_for`] and
//! [`VoteTally::record`].

use colored::Colorize;
use serde::{Deserialize, Serialize};
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
        actions.insert("read_file".into(), ApprovalLevel::Auto);
        actions.insert("write_file".into(), ApprovalLevel::Notify);
        actions.insert("edit_file".into(), ApprovalLevel::Confirm);
        actions.insert("bash".into(), ApprovalLevel::Confirm);
        actions.insert("bash_rm".into(), ApprovalLevel::Deny);
        actions.insert("bash_git_push".into(), ApprovalLevel::Deny);
        actions.insert("bash_docker_run".into(), ApprovalLevel::Confirm);
        actions.insert("bash_curl".into(), ApprovalLevel::Confirm);
        actions.insert("bash_sudo".into(), ApprovalLevel::Deny);
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
    matrix()
        .read()
        .unwrap()
        .actions
        .get(action)
        .copied()
        .unwrap_or(ApprovalLevel::Auto)
}

/// Classify a bash command into a more specific action subtype.
#[allow(dead_code)]
pub fn classify_bash(cmd: &str) -> ApprovalLevel {
    let c = cmd.trim().to_lowercase();
    if c.contains("sudo ") {
        return level_for("bash_sudo");
    }
    if c.contains("git push") {
        return level_for("bash_git_push");
    }
    if c.starts_with("rm ") || c.contains(" rm -") {
        return level_for("bash_rm");
    }
    if c.contains("docker run") {
        return level_for("bash_docker_run");
    }
    if c.starts_with("curl ") || c.contains(" curl ") {
        return level_for("bash_curl");
    }
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
    println!(
        "\n  {} {}",
        "Source:".dimmed(),
        ".dipralix/approval.toml (or defaults)".dimmed()
    );
}

pub fn set_speed_fast() {
    let mut m = matrix().write().unwrap();
    for level in m.actions.values_mut() {
        if *level != ApprovalLevel::Deny {
            *level = ApprovalLevel::Auto;
        }
    }
    println!(
        "  {} Speed: {} (all non-Deny actions auto-approved)",
        "[OK]".green(),
        "FAST".green().bold()
    );
}

pub fn set_speed_safe() {
    let mut m = matrix().write().unwrap();
    for level in m.actions.values_mut() {
        if *level == ApprovalLevel::Auto {
            *level = ApprovalLevel::Confirm;
        }
    }
    println!(
        "  {} Speed: {} (all Auto actions now require Confirm)",
        "[OK]".green(),
        "SAFE".yellow().bold()
    );
}

// ─── Phase 2: team-shared policy + remote vote tally ─────────────────────

/// Per-action override loaded from the `[team-policy]` section of
/// `.dipralix/approval.toml`. The flat `actions` map is still
/// authoritative for the approval level; the team policy only
/// augments the quorum requirement.
#[allow(dead_code)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TeamPolicy {
    /// Map of action → minimum number of distinct remote
    /// approvers (excluding the requester) needed to allow the
    /// action. A `Confirm` action with no entry requires 1
    /// approver by default.
    #[serde(default)]
    pub required_approvers: HashMap<String, u32>,
    /// Author of the policy (informational).
    #[serde(default)]
    pub author: Option<String>,
    /// Last-modified ISO-8601 timestamp (informational).
    #[serde(default)]
    pub last_modified: Option<String>,
}

#[allow(dead_code)]
impl TeamPolicy {
    /// Load the team policy from `.dipralix/approval.toml`. If
    /// the file is missing or the `[team-policy]` section is
    /// missing, returns the default policy.
    pub fn load() -> Self {
        let path = std::path::Path::new(".dipralix/approval.toml");
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        #[derive(serde::Deserialize)]
        struct Raw {
            #[serde(default)]
            team_policy: Option<TeamPolicy>,
        }
        let Ok(raw) = toml::from_str::<Raw>(&content) else {
            return Self::default();
        };
        raw.team_policy.unwrap_or_default()
    }

    /// How many distinct remote approvers an action needs. Falls
    /// back to 1 for any `Confirm` action, 0 for non-`Confirm`.
    pub fn required_approvers_for(&self, action: &str) -> u32 {
        if let Some(&n) = self.required_approvers.get(action) {
            return n;
        }
        if level_for(action) == ApprovalLevel::Confirm {
            1
        } else {
            0
        }
    }
}

/// In-memory tally for a single approval request. Lives on the
/// server (or on the client when running the offline path).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteTally {
    /// The user who asked for approval. Self-votes are ignored.
    pub requester: String,
    /// Distinct approvers (excluding the requester).
    pub approvals: Vec<String>,
    /// Voters who denied. A single deny short-circuits.
    pub denials: Vec<String>,
    /// Minimum number of approvals required to resolve as
    /// `Approved`.
    pub required: u32,
    /// True once the tally is in a terminal state (approved or
    /// denied) and no further votes will be accepted.
    pub resolved: bool,
}

#[allow(dead_code)]
impl VoteTally {
    /// Construct a new tally.
    pub fn new(requester: impl Into<String>, required: u32) -> Self {
        Self {
            requester: requester.into(),
            approvals: Vec::new(),
            denials: Vec::new(),
            required,
            resolved: false,
        }
    }

    /// Record a vote. Returns the resulting decision if the vote
    /// caused the tally to resolve, `None` otherwise. Self-votes
    /// are silently ignored (the requester can't approve their
    /// own request).
    pub fn record(
        &mut self,
        voter: &str,
        vote: super::sync::protocol::ApprovalVoteKind,
    ) -> Option<TallyOutcome> {
        if self.resolved {
            return None;
        }
        if voter == self.requester {
            return None;
        }
        match vote {
            super::sync::protocol::ApprovalVoteKind::Approve => {
                if !self.approvals.iter().any(|v| v == voter) {
                    self.approvals.push(voter.to_string());
                }
            }
            super::sync::protocol::ApprovalVoteKind::Deny => {
                if !self.denials.iter().any(|v| v == voter) {
                    self.denials.push(voter.to_string());
                }
            }
        }
        if !self.denials.is_empty() {
            self.resolved = true;
            return Some(TallyOutcome::Denied(self.denials.clone()));
        }
        if self.approvals.len() as u32 >= self.required {
            self.resolved = true;
            return Some(TallyOutcome::Approved(self.approvals.clone()));
        }
        None
    }
}

/// Terminal state of a [`VoteTally`].
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TallyOutcome {
    Approved(Vec<String>),
    Denied(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::protocol::ApprovalVoteKind;

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

    #[test]
    fn team_policy_required_defaults() {
        let p = TeamPolicy::default();
        assert_eq!(p.required_approvers_for("bash"), 1);
        assert_eq!(p.required_approvers_for("read_file"), 0);
    }

    #[test]
    fn team_policy_required_override() {
        let mut p = TeamPolicy::default();
        p.required_approvers.insert("bash.docker_run".into(), 2);
        assert_eq!(p.required_approvers_for("bash.docker_run"), 2);
    }

    #[test]
    fn tally_collects_distinct_approvers() {
        let mut t = VoteTally::new("alice", 2);
        assert_eq!(t.record("bob", ApprovalVoteKind::Approve), None);
        assert_eq!(t.record("bob", ApprovalVoteKind::Approve), None); // duplicate ignored
        assert_eq!(t.approvals, vec!["bob".to_string()]);
        match t.record("carol", ApprovalVoteKind::Approve) {
            Some(TallyOutcome::Approved(v)) => {
                assert_eq!(v, vec!["bob".to_string(), "carol".to_string()])
            }
            other => panic!("expected approved, got {other:?}"),
        }
        assert!(t.resolved);
    }

    #[test]
    fn tally_deny_short_circuits() {
        let mut t = VoteTally::new("alice", 2);
        t.record("bob", ApprovalVoteKind::Approve);
        match t.record("carol", ApprovalVoteKind::Deny) {
            Some(TallyOutcome::Denied(v)) => assert_eq!(v, vec!["carol".to_string()]),
            other => panic!("expected denied, got {other:?}"),
        }
        // No more votes accepted.
        assert_eq!(t.record("dave", ApprovalVoteKind::Approve), None);
    }

    #[test]
    fn tally_ignores_self_vote() {
        let mut t = VoteTally::new("alice", 1);
        assert_eq!(t.record("alice", ApprovalVoteKind::Approve), None);
        assert!(t.approvals.is_empty());
    }
}
