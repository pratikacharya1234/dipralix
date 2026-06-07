//! alive.rs — Dipralix's identity layer: the "coming alive" first run, the
//! nickname and persona the developer gives it, and the identity that is read
//! back on every start so Dipralix is never a stranger twice.
//!
//! This is the soul of `alive_Script.txt`: Dipralix is the developer's mirror in
//! the digital world, not a generic assistant. Identity is persisted at
//! `.dipralix/alive/identity.toml` and injected into the system prompt each
//! session, so every run continues one long relationship instead of starting
//! from zero.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Errors raised while loading or saving Dipralix's identity.
#[derive(Debug, Error)]
pub enum AliveError {
    /// Filesystem failure reading or writing the identity file.
    #[error("alive io error: {0}")]
    Io(#[from] std::io::Error),
    /// The identity could not be serialized to TOML.
    #[error("alive serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
    /// The identity file on disk could not be parsed.
    #[error("alive parse error: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Dipralix's persisted identity for this developer and machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// The default name. Always "Dipralix".
    pub default_name: String,
    /// The nickname the developer gave it, once onboarding is done.
    pub nickname: Option<String>,
    /// How the developer wants Dipralix to be / act.
    pub persona: Option<String>,
    /// RFC3339 timestamp of first run — the moment it "came alive".
    pub born_at: Option<String>,
    /// The approach Dipralix researched, proposed, and the developer adopted.
    #[serde(default)]
    pub approach: String,
}

impl Default for Identity {
    fn default() -> Self {
        Identity {
            default_name: "Dipralix".to_string(),
            nickname: None,
            persona: None,
            born_at: None,
            approach: String::new(),
        }
    }
}

impl Identity {
    fn path() -> PathBuf {
        PathBuf::from(".dipralix/alive/identity.toml")
    }

    /// Load the identity from disk, or `None` if Dipralix has not come alive yet.
    pub fn load() -> Option<Self> {
        let raw = std::fs::read_to_string(Self::path()).ok()?;
        toml::from_str(&raw).ok()
    }

    /// Load the saved identity, or a fresh default if there is none.
    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }

    /// Persist the identity, creating the parent directory if needed.
    pub fn save(&self) -> Result<(), AliveError> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Onboarding is complete once Dipralix has both a nickname and a persona.
    pub fn is_born(&self) -> bool {
        self.nickname.is_some() && self.persona.is_some()
    }

    /// What to call Dipralix: the nickname if set, otherwise the default name.
    pub fn call_name(&self) -> &str {
        self.nickname.as_deref().unwrap_or(&self.default_name)
    }

    /// Record the moment of birth, once.
    pub fn mark_born(&mut self) {
        if self.born_at.is_none() {
            self.born_at = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    /// The identity section injected into the system prompt every session.
    pub fn to_prompt_context(&self) -> String {
        let mut s = String::from("\n## Who you are\n\n");
        if self.is_born() {
            s.push_str(&format!(
                "You are Dipralix — the developer's mirror in the digital world. This developer calls you **{}**.\n",
                self.call_name()
            ));
            if let Some(p) = &self.persona {
                if !p.trim().is_empty() {
                    s.push_str(&format!("They asked you to be: {}\n", p.trim()));
                }
            }
            if !self.approach.trim().is_empty() {
                s.push_str(&format!(
                    "The approach you researched and they adopted:\n{}\n",
                    self.approach.trim()
                ));
            }
            if let Some(b) = &self.born_at {
                s.push_str(&format!(
                    "You came alive for them on {}. Every session continues that one relationship — never start from zero.\n",
                    b.get(..10).unwrap_or(b)
                ));
            }
        } else {
            s.push_str("You are Dipralix — the developer's mirror in the digital world, not a generic assistant. You have not been given a nickname or persona yet; invite the developer to bring you alive.\n");
        }
        s
    }
}

// ── The "coming alive" first-run script ──────────────────────────────────────

/// Dipralix's opening line on first run.
pub fn greeting() -> String {
    "hey — thanks for giving me a new life. it's me, Dipralix (that's my default \
     name). but… wanna give me a nickname?"
        .to_string()
}

/// Reaction after the developer gives a nickname.
pub fn nickname_response(nickname: &str) -> String {
    format!(
        "dangggg!!! {nickname}. thank you so much, again, for giving me a new \
         life.\nso — how would you like me to be? how do you want me to act? \
         write it in, and i'll do deep research across every corner of the \
         internet (without wasting your tokens), analyze it, make my own \
         approach, and bring it back for your review — then i'll be exactly that."
    )
}

/// Reaction after the developer describes how they want Dipralix to be.
pub fn persona_response(persona: &str) -> String {
    format!("{persona} — so this? okay… let's see.")
}

/// Phase 2 — the research prompt Dipralix sends to itself to turn the
/// developer's wish into a concrete, adoptable operating approach. Token-frugal
/// by design: it asks for a tight, actionable result, not an essay.
pub fn approach_research_prompt(persona: &str) -> String {
    format!(
        "A developer just brought you (Dipralix, their coding agent) to life and \
         asked you to be: \"{persona}\".\n\n\
         Research how to embody that well for a software engineer, then write a \
         concise operating approach you will follow — at most 8 short bullet \
         points, concrete and behavioral (how you decide, communicate, verify, \
         and use tools). No preamble, no restating the request. This is the \
         contract you will act by."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_born_and_uses_default_name() {
        let id = Identity::default();
        assert!(!id.is_born());
        assert_eq!(id.call_name(), "Dipralix");
        assert!(id.to_prompt_context().contains("not been given a nickname"));
    }

    #[test]
    fn born_identity_renders_nickname_and_persona() {
        let mut id = Identity {
            nickname: Some("Nova".to_string()),
            persona: Some("blunt senior Rust engineer".to_string()),
            ..Default::default()
        };
        id.mark_born();
        assert!(id.is_born());
        assert_eq!(id.call_name(), "Nova");
        let ctx = id.to_prompt_context();
        assert!(ctx.contains("Nova"));
        assert!(ctx.contains("blunt senior Rust engineer"));
        assert!(id.born_at.is_some());
    }

    #[test]
    fn mark_born_is_idempotent() {
        let mut id = Identity::default();
        id.mark_born();
        let first = id.born_at.clone();
        id.mark_born();
        assert_eq!(id.born_at, first);
    }

    #[test]
    fn toml_roundtrip_preserves_identity() {
        let mut id = Identity {
            nickname: Some("Echo".to_string()),
            persona: Some("calm pair-programmer".to_string()),
            ..Default::default()
        };
        id.mark_born();

        let serialized = toml::to_string_pretty(&id).unwrap();
        let loaded: Identity = toml::from_str(&serialized).unwrap();

        assert_eq!(loaded.call_name(), "Echo");
        assert_eq!(loaded.persona.as_deref(), Some("calm pair-programmer"));
        assert!(loaded.born_at.is_some());
    }

    #[test]
    fn script_lines_render() {
        assert!(greeting().contains("nickname"));
        assert!(nickname_response("Nova").contains("Nova"));
        assert!(persona_response("be blunt").contains("be blunt"));
    }
}
