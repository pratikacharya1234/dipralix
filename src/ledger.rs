//! Verified Outcome Ledger — Dipralix's per-repo memory of what it has actually
//! verified, what failed, and which facts have been superseded over time.
//!
//! This is the read/write side of Dipralix's trust model. Unlike a static skill
//! file or a vector store, the ledger is:
//!
//! - **Verified-first.** An entry is only [`Outcome::Verified`] when a machine
//!   confirmed it — a build or test that exited cleanly. The proof is attached.
//! - **Temporal.** Facts can be *superseded*. When today's reality contradicts a
//!   past entry, a new entry records the change and points back at the one it
//!   replaces, so the agent reasons about *how things changed*, not just what is.
//! - **Append-only and local.** It lives at `.dipralix/ledger/outcomes.jsonl`,
//!   one JSON object per line, git-trackable, readable by a human.
//!
//! The ledger is injected into the system prompt each turn (see
//! [`Ledger::to_prompt_context`]) so a session inherits what the repo already
//! proved, and the agent writes to it through the `record_outcome` tool.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Errors raised while reading or writing the ledger.
#[derive(Debug, Error)]
pub enum LedgerError {
    /// Filesystem failure reading or writing the ledger file.
    #[error("ledger io error: {0}")]
    Io(#[from] std::io::Error),
    /// A ledger line could not be parsed or an entry could not be serialized.
    #[error("ledger serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// The verdict on a recorded task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// A machine confirmed the change works (build/test exited cleanly).
    Verified,
    /// The attempt was tried and failed verification.
    Failed,
    /// The user reviewed the change and declined it.
    Rejected,
    /// A marker entry: a prior fact no longer holds (see `supersedes`).
    Superseded,
}

impl Outcome {
    /// Parse a loose user/agent-supplied string into an [`Outcome`].
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "verified" | "verify" | "ok" | "pass" | "passed" => Some(Outcome::Verified),
            "failed" | "fail" | "error" => Some(Outcome::Failed),
            "rejected" | "reject" | "declined" => Some(Outcome::Rejected),
            "superseded" | "supersede" | "stale" => Some(Outcome::Superseded),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Outcome::Verified => "VERIFIED",
            Outcome::Failed => "FAILED",
            Outcome::Rejected => "REJECTED",
            Outcome::Superseded => "SUPERSEDED",
        }
    }
}

/// How much the agent trusts an outcome, calibrated by the strength of its proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// Backed by passing tests, or a clean build of a well-covered change.
    High,
    /// Builds and runs, but verification was partial.
    Medium,
    /// Plausible but unproven; treat with caution.
    Low,
}

impl Confidence {
    /// Parse a loose string into a [`Confidence`], defaulting to `Medium`.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "high" | "h" => Confidence::High,
            "low" | "l" => Confidence::Low,
            _ => Confidence::Medium,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }
}

/// The machine-checked evidence behind an outcome — the "proof of work".
///
/// This is what separates Dipralix's claim of "done" from a guess: the exact
/// command that ran, its exit code, and any test counts it produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfWork {
    /// The verification command that was run (e.g. `cargo test`).
    pub command: String,
    /// Its exit code. Zero means success.
    pub exit_code: i32,
    /// Tests observed passing, if the command reported a count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests_passed: Option<u32>,
    /// New tests added by the change, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests_added: Option<u32>,
}

impl ProofOfWork {
    /// Whether the verification command succeeded.
    pub fn succeeded(&self) -> bool {
        self.exit_code == 0
    }

    /// A compact one-line human rendering of the proof.
    pub fn render(&self) -> String {
        let mut s = format!("`{}` exit {}", self.command, self.exit_code);
        if let Some(p) = self.tests_passed {
            s.push_str(&format!(", {p} passed"));
        }
        if let Some(a) = self.tests_added {
            s.push_str(&format!(", {a} added"));
        }
        s
    }
}

/// A single recorded outcome in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeEntry {
    /// Short unique id, used as the target of a later supersession.
    pub id: String,
    /// RFC3339 timestamp of when the outcome was recorded.
    pub ts: String,
    /// One-line description of the task.
    pub task: String,
    /// Task classification (e.g. "code-change", "analysis").
    #[serde(default)]
    pub kind: String,
    /// Files the change touched.
    #[serde(default)]
    pub files: Vec<String>,
    /// The verdict.
    pub outcome: Outcome,
    /// Calibrated confidence in the verdict.
    pub confidence: Confidence,
    /// The machine-checked evidence, when there is any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<ProofOfWork>,
    /// Id of an earlier entry this one supersedes, if it overturns a past fact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    /// Free-text note: the failure reason, the caveat, the why.
    #[serde(default)]
    pub note: String,
}

impl OutcomeEntry {
    fn date(&self) -> &str {
        self.ts.get(..10).unwrap_or(&self.ts)
    }

    fn render_line(&self) -> String {
        let mut line = format!(
            "- [{} {}] {} ({})",
            self.outcome.label(),
            self.date(),
            self.task.trim(),
            self.confidence.label()
        );
        if let Some(proof) = &self.proof {
            line.push_str(&format!(" — {}", proof.render()));
        }
        if !self.note.trim().is_empty() {
            line.push_str(&format!(" — note: {}", self.note.trim()));
        }
        line
    }
}

/// An append-only, per-repo log of verified outcomes.
pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    /// Open the ledger at the conventional location for the current repo
    /// (`.dipralix/ledger/outcomes.jsonl`). The file is created lazily on the
    /// first write, so this never fails for a missing file.
    pub fn open() -> Self {
        Ledger::at(PathBuf::from(".dipralix/ledger/outcomes.jsonl"))
    }

    /// Open a ledger at an explicit path. Used by tests and tooling.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Ledger { path: path.into() }
    }

    /// Append one entry as a JSON line, creating the parent directory if needed.
    pub fn append(&self, entry: &OutcomeEntry) -> Result<(), LedgerError> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    /// Build and append a fresh entry, returning it (so the caller can render a
    /// receipt). Generates the id and timestamp.
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &self,
        task: impl Into<String>,
        kind: impl Into<String>,
        files: Vec<String>,
        outcome: Outcome,
        confidence: Confidence,
        proof: Option<ProofOfWork>,
        supersedes: Option<String>,
        note: impl Into<String>,
    ) -> Result<OutcomeEntry, LedgerError> {
        let entry = OutcomeEntry {
            id: new_id(),
            ts: now_rfc3339(),
            task: task.into(),
            kind: kind.into(),
            files,
            outcome,
            confidence,
            proof,
            supersedes,
            note: note.into(),
        };
        self.append(&entry)?;
        Ok(entry)
    }

    /// Read every entry, oldest first. Unparseable lines are skipped so a single
    /// corrupt line never blinds the agent to the rest of its history.
    pub fn all(&self) -> Result<Vec<OutcomeEntry>, LedgerError> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let entries = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<OutcomeEntry>(l).ok())
            .collect();
        Ok(entries)
    }

    /// The most recent `n` entries, newest first.
    pub fn recent(&self, n: usize) -> Result<Vec<OutcomeEntry>, LedgerError> {
        let mut all = self.all()?;
        all.reverse();
        all.truncate(n);
        Ok(all)
    }

    /// Entries whose facts still hold: drops supersession markers and any entry
    /// a later entry explicitly superseded. Newest first.
    pub fn active(&self) -> Result<Vec<OutcomeEntry>, LedgerError> {
        let all = self.all()?;
        let superseded: std::collections::HashSet<&str> =
            all.iter().filter_map(|e| e.supersedes.as_deref()).collect();
        let mut active: Vec<OutcomeEntry> = all
            .iter()
            .filter(|e| e.outcome != Outcome::Superseded)
            .filter(|e| !superseded.contains(e.id.as_str()))
            .cloned()
            .collect();
        active.reverse();
        Ok(active)
    }

    /// Render the active ledger as a system-prompt section, or an empty string
    /// when there is nothing to show. Capped to the most recent entries to keep
    /// the prompt lean.
    pub fn to_prompt_context(&self) -> String {
        const MAX: usize = 12;
        let active = match self.active() {
            Ok(a) if !a.is_empty() => a,
            _ => return String::new(),
        };
        let mut out = String::from(
            "\n## Verified Outcome Ledger (this repo)\n\nOutcomes Dipralix has already verified or seen fail here. Trust these over your priors; if reality now differs, record the supersession.\n\n",
        );
        for entry in active.iter().take(MAX) {
            out.push_str(&entry.render_line());
            out.push('\n');
        }
        out
    }
}

/// Current time as an RFC3339 string.
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// A short, collision-resistant id for an entry.
fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_ledger() -> Ledger {
        let p = std::env::temp_dir().join(format!("dipralix_ledger_{}.jsonl", new_id()));
        Ledger::at(p)
    }

    #[test]
    fn append_and_read_roundtrip() {
        let ledger = temp_ledger();
        let proof = ProofOfWork {
            command: "cargo test".into(),
            exit_code: 0,
            tests_passed: Some(12),
            tests_added: Some(1),
        };
        assert!(proof.succeeded());
        ledger
            .record(
                "add rate limiting",
                "code-change",
                vec!["src/api.rs".into()],
                Outcome::Verified,
                Confidence::High,
                Some(proof),
                None,
                "",
            )
            .unwrap();

        let all = ledger.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].outcome, Outcome::Verified);
        assert_eq!(all[0].proof.as_ref().unwrap().tests_passed, Some(12));
        let _ = std::fs::remove_file(&ledger.path);
    }

    #[test]
    fn supersession_hides_old_fact() {
        let ledger = temp_ledger();
        let first = ledger
            .record(
                "uses axum 0.7",
                "analysis",
                vec![],
                Outcome::Verified,
                Confidence::High,
                None,
                None,
                "",
            )
            .unwrap();
        ledger
            .record(
                "migrated to axum 0.8",
                "code-change",
                vec!["Cargo.toml".into()],
                Outcome::Verified,
                Confidence::High,
                None,
                Some(first.id.clone()),
                "Handler trait bounds changed",
            )
            .unwrap();

        let active = ledger.active().unwrap();
        assert_eq!(active.len(), 1, "superseded entry should be filtered out");
        assert_eq!(active[0].task, "migrated to axum 0.8");

        let ctx = ledger.to_prompt_context();
        assert!(ctx.contains("Verified Outcome Ledger"));
        assert!(ctx.contains("axum 0.8"));
        assert!(!ctx.contains("uses axum 0.7"));
        let _ = std::fs::remove_file(&ledger.path);
    }

    #[test]
    fn missing_file_is_empty_not_error() {
        let ledger = Ledger::at(std::env::temp_dir().join("dipralix_nonexistent_ledger.jsonl"));
        let _ = std::fs::remove_file(&ledger.path);
        assert!(ledger.all().unwrap().is_empty());
        assert!(ledger.to_prompt_context().is_empty());
    }

    #[test]
    fn proof_renders_and_outcome_parses() {
        let proof = ProofOfWork {
            command: "cargo build".into(),
            exit_code: 101,
            tests_passed: None,
            tests_added: None,
        };
        assert!(!proof.succeeded());
        assert_eq!(proof.render(), "`cargo build` exit 101");
        assert_eq!(Outcome::parse("pass"), Some(Outcome::Verified));
        assert_eq!(Outcome::parse("reject"), Some(Outcome::Rejected));
        assert_eq!(Outcome::parse("stale"), Some(Outcome::Superseded));
        assert_eq!(Outcome::parse("???"), None);
        assert_eq!(Confidence::parse("h").label(), "high");
        assert_eq!(Confidence::parse("unknown").label(), "medium");
    }
}
