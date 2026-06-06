//! Path allowlist for the sync layer.
//!
//! **Only** files under `.dipralix/` (and a small list of well-known
//! sub-paths) may be transmitted. Source code, API keys, and personal
//! `config.local` files are rejected before they ever reach the wire.
//!
//! The allowlist is the single source of truth for the
//! "no source-code / API keys / config.local" guarantee in
//! `dipralix-realtime.md` §10 Phase 1 DoD.

use std::path::{Component, Path};

use crate::sync::error::{Result, SyncError};

/// The top-level directory under the project root that we sync.
pub const DIPRALIX_DIR: &str = ".dipralix";

/// Filename (anywhere) that must never be transmitted.
const FORBIDDEN_FILENAMES: &[&str] = &[
    ".env",
    "config.local",
    "config.local.toml",
    "id_rsa",
    "id_ed25519",
    "id_dsa",
    "credentials",
    "credentials.json",
    "secrets.toml",
    "secrets.yaml",
    "secrets.yml",
    "secrets.json",
];

/// Path-component prefixes that must never be transmitted (matched on
/// the *first* path component after the project root, case-sensitive).
const FORBIDDEN_TOP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".ssh",
    ".aws",
    ".kube",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
];

/// Known-good subpaths under `.dipralix/`. Anything else under
/// `.dipralix/` is also allowed (e.g. user-defined `notes/`), so this
/// list is a *positive* hint, not a closed set.
const ALLOWED_SUBDIRS: &[&str] = &[
    "memory",
    "plans",
    "skills",
    "context",
    "approval.toml",
    "config.toml",
    "audit.log",
    "sync.log",
    "dipralix-chat.log",
];

/// Returns true if `path` is allowed to be sent over the wire.
///
/// `path` is expected to be relative to the project root, POSIX-style
/// (forward slashes), with no leading `./`.
pub fn is_allowed(path: &str) -> bool {
    if let Err(e) = validate(path) {
        tracing::debug!(path, error = %e, "path rejected by allowlist");
        return false;
    }
    true
}

/// Strict variant of [`is_allowed`] that returns the typed error
/// explaining *why* a path was rejected. Used by callers that need to
/// surface a reason to the user.
pub fn validate(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(SyncError::InvalidPath("empty path".to_string()));
    }

    let p = Path::new(path);

    // Reject absolute paths and any path containing a parent traversal
    // component. Both are clear signs of escape attempts.
    if p.is_absolute() {
        return Err(SyncError::InvalidPath(format!(
            "absolute path not allowed: {path}"
        )));
    }
    for c in p.components() {
        if matches!(c, Component::ParentDir) {
            return Err(SyncError::InvalidPath(format!(
                "parent traversal not allowed: {path}"
            )));
        }
    }

    // Top-level directory check (e.g. `target/`, `.git/`).
    if let Some(first) = p.components().next() {
        let first = first.as_os_str().to_string_lossy().to_string();
        if FORBIDDEN_TOP_DIRS.contains(&first.as_str()) {
            return Err(SyncError::PathNotAllowed(format!(
                "top-level dir '{first}' is not synced"
            )));
        }
    }

    // The chat log and the sync log live at the project root, not
    // under .dipralix, but are explicitly allowed by the protocol.
    if matches!(
        path,
        "dipralix-chat.log" | "dipralix-sync.log" | "dipralix-sync.db"
    ) {
        return Ok(());
    }

    // The path must live under `.dipralix/`. This is the single
    // biggest safety net.
    let under_dipralix = path == DIPRALIX_DIR || path.starts_with(&format!("{DIPRALIX_DIR}/"));

    if !under_dipralix {
        return Err(SyncError::PathNotAllowed(format!(
            "path is not under {DIPRALIX_DIR}/: {path}"
        )));
    }

    // Forbid specific filenames anywhere under .dipralix/.
    let basename = p
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    if FORBIDDEN_FILENAMES.contains(&basename.as_str()) {
        return Err(SyncError::PathNotAllowed(format!(
            "filename '{basename}' is never synced"
        )));
    }

    // If the path is `.dipralix/<sub>/...` and `<sub>` is a known
    // subpath, accept. (Anything else under .dipralix is also
    // allowed — we don't want to be brittle.)
    if let Some(rest) = path.strip_prefix(&format!("{DIPRALIX_DIR}/")) {
        if let Some(sub) = rest.split('/').next() {
            if ALLOWED_SUBDIRS.contains(&sub) {
                return Ok(());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_memory_files() {
        assert!(is_allowed(".dipralix/memory/decisions.md"));
        assert!(is_allowed(".dipralix/memory/auth/notes.md"));
    }

    #[test]
    fn allows_plans_and_skills() {
        assert!(is_allowed(".dipralix/plans/current.md"));
        assert!(is_allowed(".dipralix/skills/rust-patterns.md"));
    }

    #[test]
    fn allows_known_metadata_files() {
        assert!(is_allowed(".dipralix/approval.toml"));
        assert!(is_allowed(".dipralix/config.toml"));
        assert!(is_allowed(".dipralix/audit.log"));
    }

    #[test]
    fn allows_chat_log_at_root() {
        assert!(is_allowed("dipralix-chat.log"));
    }

    #[test]
    fn rejects_source_code() {
        for p in [
            "src/main.rs",
            "src/sync/protocol.rs",
            "backend/server.go",
            "frontend/index.jsx",
        ] {
            assert!(!is_allowed(p), "{p} should be rejected");
        }
    }

    #[test]
    fn rejects_env_and_config_local() {
        for p in [
            ".env",
            ".env.production",
            "config.local",
            "config.local.toml",
            ".dipralix/config.local",
        ] {
            assert!(!is_allowed(p), "{p} should be rejected");
        }
    }

    #[test]
    fn rejects_secrets() {
        for p in [
            ".dipralix/secrets.toml",
            ".dipralix/keys/id_rsa",
            "secrets.yaml",
            "credentials.json",
        ] {
            assert!(!is_allowed(p), "{p} should be rejected");
        }
    }

    #[test]
    fn rejects_target_and_node_modules() {
        assert!(!is_allowed("target/debug/main"));
        assert!(!is_allowed("node_modules/foo/index.js"));
        assert!(!is_allowed(".git/HEAD"));
        assert!(!is_allowed(".ssh/id_ed25519"));
        assert!(!is_allowed(".aws/credentials"));
    }

    #[test]
    fn rejects_absolute_paths() {
        assert!(!is_allowed("/etc/passwd"));
        assert!(!is_allowed("/Users/alice/.dipralix/memory/x.md"));
    }

    #[test]
    fn rejects_parent_traversal() {
        assert!(!is_allowed("../etc/passwd"));
        assert!(!is_allowed(".dipralix/../../etc/passwd"));
    }

    #[test]
    fn rejects_empty_path() {
        assert!(!is_allowed(""));
    }
}
