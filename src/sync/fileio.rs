//! Shared filesystem apply/read logic for the sync clients.
//!
//! Both the server-mediated [`crate::sync::client::SyncClient`] and the
//! serverless [`crate::sync::mesh::MeshNode`] need the same three behaviors:
//!
//! 1. Turn a locally-changed, allowlisted file into a [`FileUpdate`] to send.
//! 2. Apply an inbound [`FileUpdate`] to disk.
//! 3. Suppress echo loops — never re-send content we just received, and
//!    never re-apply content that already matches what is on disk.
//!
//! [`FileSync`] owns the `path → last-known blake3 hash` cache that makes the
//! loop-suppression work, so there is exactly one implementation of this
//! logic instead of one per transport.

use std::collections::HashMap;
use std::path::PathBuf;

use tracing::debug;

use super::allowlist;
use super::error::{Result, SyncError};
use super::protocol::{ContentKind, FileUpdate};

/// Project-rooted file applier with echo-suppression state.
pub struct FileSync {
    root: PathBuf,
    /// `rel_path → last-known blake3 hash` (hex). Updated on every send and
    /// every apply so a server/peer echo of our own write is a no-op.
    last_known: HashMap<String, String>,
}

impl FileSync {
    /// Create a `FileSync` rooted at the project directory. Paths are always
    /// resolved as `<root>/<rel_path>`.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            last_known: HashMap::new(),
        }
    }

    /// The project root this applier writes under.
    #[must_use]
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Build a [`FileUpdate`] for a locally-changed file, or `Ok(None)` if the
    /// path is not allowlisted, the file is gone, or its content is unchanged
    /// since we last saw it (the echo case).
    ///
    /// Text files are sent as UTF-8; non-UTF-8 files are base64-encoded.
    ///
    /// # Errors
    /// Returns [`SyncError::Io`] on a read error other than not-found.
    pub fn local_update(&mut self, rel_path: &str, author: &str) -> Result<Option<FileUpdate>> {
        if !allowlist::is_allowed(rel_path) {
            debug!(path = %rel_path, "skip: not allowlisted");
            return Ok(None);
        }
        let abs = self.root.join(rel_path);
        let body = match std::fs::read(&abs) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(SyncError::Io(e)),
        };
        let hash = blake3::hash(&body).to_hex().to_string();
        if self.last_known.get(rel_path) == Some(&hash) {
            debug!(path = %rel_path, "skip: unchanged / echo");
            return Ok(None);
        }
        self.last_known.insert(rel_path.to_string(), hash);

        let upd = if std::str::from_utf8(&body).is_ok() {
            FileUpdate::from_text(rel_path, &body, author)
        } else {
            FileUpdate::from_binary(rel_path, &body, author)
        };
        Ok(Some(upd))
    }

    /// Apply an inbound [`FileUpdate`] to disk. Returns `Ok(true)` if the file
    /// was written, `Ok(false)` if it already matched (deduped).
    ///
    /// # Errors
    /// Returns [`SyncError::PathNotAllowed`] if the update targets a path
    /// outside the allowlist, [`SyncError::Protocol`] on a bad base64 body, or
    /// [`SyncError::Io`] on a write failure.
    pub fn apply(&mut self, upd: &FileUpdate) -> Result<bool> {
        if !allowlist::is_allowed(&upd.path) {
            return Err(SyncError::PathNotAllowed(upd.path.clone()));
        }
        let abs = self.root.join(&upd.path);
        if let Ok(existing) = std::fs::read(&abs) {
            let existing_hash = blake3::hash(&existing).to_hex().to_string();
            if existing_hash == upd.hash {
                self.last_known.insert(upd.path.clone(), upd.hash.clone());
                return Ok(false);
            }
        }
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).map_err(SyncError::Io)?;
        }
        let bytes = decode_body(upd)?;
        std::fs::write(&abs, &bytes).map_err(SyncError::Io)?;
        self.last_known.insert(upd.path.clone(), upd.hash.clone());
        debug!(path = %upd.path, size = bytes.len(), author = %upd.author, "applied remote change");
        Ok(true)
    }

    /// Record a hash for a path without touching disk. Used to seed the cache
    /// from a join snapshot so the first local scan does not re-broadcast it.
    pub fn remember(&mut self, rel_path: &str, hash: &str) {
        self.last_known
            .insert(rel_path.to_string(), hash.to_string());
    }
}

/// Scan the project's `.dipralix/` tree and build a [`FileUpdate`] for every
/// allowlisted file. Sent to a peer when it first connects so it converges on
/// our current state (the mesh equivalent of the server's join snapshot).
///
/// Best-effort: unreadable entries are skipped rather than failing the scan.
#[must_use]
pub fn scan_snapshot(root: &std::path::Path, author: &str) -> Vec<FileUpdate> {
    let dipralix = root.join(allowlist::DIPRALIX_DIR);
    let mut out = Vec::new();
    if !dipralix.is_dir() {
        return out;
    }
    for entry in walkdir::WalkDir::new(&dipralix)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(rel_path) = entry.path().strip_prefix(root) else {
            continue;
        };
        // Normalize to POSIX-style for the allowlist + wire.
        let rel = rel_path.to_string_lossy().replace('\\', "/");
        if !allowlist::is_allowed(&rel) {
            continue;
        }
        let Ok(body) = std::fs::read(entry.path()) else {
            continue;
        };
        let upd = if std::str::from_utf8(&body).is_ok() {
            FileUpdate::from_text(&rel, &body, author)
        } else {
            FileUpdate::from_binary(&rel, &body, author)
        };
        out.push(upd);
    }
    out
}

/// Decode a [`FileUpdate`] body to raw bytes (text or base64 binary).
fn decode_body(upd: &FileUpdate) -> Result<Vec<u8>> {
    match upd.kind {
        ContentKind::Text => Ok(upd.content.clone().unwrap_or_default().into_bytes()),
        ContentKind::Binary => match &upd.content_b64 {
            Some(b64) => base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
                .map_err(|e| SyncError::Protocol(format!("base64: {e}"))),
            None => Ok(Vec::new()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("dipralix-fileio-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(p.join(".dipralix/memory")).unwrap();
        p
    }

    #[test]
    fn local_update_then_echo_is_suppressed() {
        let root = tmp();
        let rel = ".dipralix/memory/a.md";
        std::fs::write(root.join(rel), b"hello").unwrap();
        let mut fs = FileSync::new(root.clone());

        let first = fs.local_update(rel, "alice").unwrap();
        assert!(first.is_some(), "first change should produce an update");
        let second = fs.local_update(rel, "alice").unwrap();
        assert!(second.is_none(), "unchanged content must be suppressed");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn local_update_rejects_non_allowlisted() {
        let root = tmp();
        std::fs::write(root.join("src_secret.rs"), b"fn main(){}").unwrap();
        let mut fs = FileSync::new(root.clone());
        assert!(fs.local_update("src_secret.rs", "alice").unwrap().is_none());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn apply_writes_then_dedups() {
        let root = tmp();
        let mut fs = FileSync::new(root.clone());
        let upd = FileUpdate::from_text(".dipralix/memory/b.md", b"world", "bob");

        assert!(fs.apply(&upd).unwrap(), "first apply writes");
        assert_eq!(
            std::fs::read(root.join(".dipralix/memory/b.md")).unwrap(),
            b"world"
        );
        assert!(!fs.apply(&upd).unwrap(), "identical apply is deduped");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn apply_rejects_path_outside_allowlist() {
        let root = tmp();
        let mut fs = FileSync::new(root.clone());
        let upd = FileUpdate::from_text("../../etc/evil", b"x", "mallory");
        assert!(matches!(fs.apply(&upd), Err(SyncError::PathNotAllowed(_))));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn apply_round_trips_binary() {
        let root = tmp();
        let mut fs = FileSync::new(root.clone());
        let body = vec![0u8, 159, 146, 150]; // invalid utf-8
        let upd = FileUpdate::from_binary(".dipralix/skills/x.bin", &body, "carol");
        assert!(fs.apply(&upd).unwrap());
        assert_eq!(
            std::fs::read(root.join(".dipralix/skills/x.bin")).unwrap(),
            body
        );
        std::fs::remove_dir_all(root).ok();
    }
}
