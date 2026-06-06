//! Append-only team chat.
//!
//! The server stores chat messages in a per-room file at
//! `<project-root>/.dipralix/dipralix-chat.log` (the allowlist
//! already permits that exact name; see `allowlist::is_allowed`).
//! New entries are appended in JSONL — one JSON object per line —
//! so the file can be tailed, grepped, and rotated without
//! parsing the whole thing.
//!
//! There is no edit or delete operation. Once a line is written it
//! is part of the audit trail.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::{Result, SyncError};
use super::protocol::SyncMessage;

/// Canonical chat log filename inside `.dipralix/`.
pub const CHAT_LOG_FILE: &str = "dipralix-chat.log";

/// One line in the chat log. Mirrors [`SyncMessage::Chat`] but
/// stored with a server-assigned timestamp so a hostile client
/// can't rewrite history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatLine {
    pub user: String,
    pub text: String,
    /// Server-assigned wall clock in ms since epoch.
    pub ts_ms: u64,
}

impl ChatLine {
    /// Validate a chat line before append. Rejects empty bodies,
    /// oversized lines, and lines that contain a literal newline
    /// (the log format is one JSON object per line).
    pub fn validate(&self) -> Result<()> {
        if self.text.is_empty() {
            return Err(SyncError::Protocol("chat: empty text".into()));
        }
        if self.text.len() > 4096 {
            return Err(SyncError::Protocol("chat: text > 4096 bytes".into()));
        }
        if self.text.contains('\n') || self.text.contains('\r') {
            return Err(SyncError::Protocol("chat: text contains newline".into()));
        }
        if self.user.is_empty() || self.user.len() > 64 {
            return Err(SyncError::Protocol("chat: invalid user".into()));
        }
        Ok(())
    }

    /// Render as a one-line JSON string, terminated by `\n`.
    pub fn encode_line(&self) -> Result<String> {
        let mut s = serde_json::to_string(self)
            .map_err(|e| SyncError::Protocol(format!("chat encode: {e}")))?;
        s.push('\n');
        Ok(s)
    }
}

/// Open the chat log for the given project root. Creates the
/// `.dipralix/` directory if it doesn't exist.
pub fn chat_log_path(project_root: &Path) -> PathBuf {
    project_root.join(".dipralix").join(CHAT_LOG_FILE)
}

/// Append a chat line. Creates the file (and parent dir) if it
/// doesn't exist. The file is opened in append mode, written, and
/// flushed + synced before returning, so a crash won't lose the
/// line.
pub fn append_chat_line(project_root: &Path, line: &ChatLine) -> Result<()> {
    line.validate()?;
    let path = chat_log_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(SyncError::Io)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(SyncError::Io)?;
    f.write_all(line.encode_line()?.as_bytes())
        .map_err(SyncError::Io)?;
    f.flush().map_err(SyncError::Io)?;
    f.sync_all().map_err(SyncError::Io)?;
    Ok(())
}

/// Read the entire chat log into memory, line by line. Lines
/// that don't parse as JSON are silently skipped — the log is
/// best-effort human-readable and the reader must not crash on
/// a corrupted entry.
pub fn read_chat_log(project_root: &Path) -> Result<Vec<ChatLine>> {
    let path = chat_log_path(project_root);
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(c) = serde_json::from_str::<ChatLine>(line) {
            out.push(c);
        }
    }
    Ok(out)
}

/// Tail the most recent `n` lines (oldest first).
pub fn tail(project_root: &Path, n: usize) -> Result<Vec<ChatLine>> {
    let all = read_chat_log(project_root)?;
    Ok(all
        .into_iter()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect())
}

/// Convert a [`SyncMessage::Chat`] into a [`ChatLine`] with the
/// server-assigned timestamp. The user is taken from the frame.
pub fn from_chat_frame(msg: &SyncMessage, server_ts_ms: u64) -> Result<ChatLine> {
    let SyncMessage::Chat { user, text, .. } = msg else {
        return Err(SyncError::Protocol("expected chat frame".into()));
    };
    let line = ChatLine {
        user: user.clone(),
        text: text.clone(),
        ts_ms: server_ts_ms,
    };
    line.validate()?;
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("dipralix-chat-{nanos}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn line(user: &str, text: &str) -> ChatLine {
        ChatLine {
            user: user.into(),
            text: text.into(),
            ts_ms: 1,
        }
    }

    #[test]
    fn validate_rejects_empty() {
        assert!(line("u", "").validate().is_err());
    }

    #[test]
    fn validate_rejects_oversized() {
        let big = "x".repeat(4097);
        assert!(line("u", &big).validate().is_err());
    }

    #[test]
    fn validate_rejects_newline() {
        assert!(line("u", "line1\nline2").validate().is_err());
    }

    #[test]
    fn validate_rejects_oversized_user() {
        let big = "u".repeat(65);
        assert!(line(&big, "x").validate().is_err());
    }

    #[test]
    fn append_creates_file_and_parent() {
        let dir = tmp_dir();
        append_chat_line(&dir, &line("alice", "hello")).unwrap();
        let p = chat_log_path(&dir);
        assert!(p.exists());
        let body = std::fs::read_to_string(&p).unwrap();
        assert!(body.contains("\"user\":\"alice\""));
        assert!(body.ends_with('\n'));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_then_read_round_trip() {
        let dir = tmp_dir();
        append_chat_line(&dir, &line("alice", "hi")).unwrap();
        append_chat_line(&dir, &line("bob", "yo")).unwrap();
        let log = read_chat_log(&dir).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].text, "hi");
        assert_eq!(log[1].user, "bob");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_does_not_overwrite() {
        let dir = tmp_dir();
        append_chat_line(&dir, &line("alice", "first")).unwrap();
        append_chat_line(&dir, &line("bob", "second")).unwrap();
        append_chat_line(&dir, &line("carol", "third")).unwrap();
        let log = read_chat_log(&dir).unwrap();
        assert_eq!(
            log.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tail_returns_last_n_in_chronological_order() {
        let dir = tmp_dir();
        for i in 0..5 {
            append_chat_line(&dir, &line("u", &format!("msg{i}"))).unwrap();
        }
        let t = tail(&dir, 2).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].text, "msg3");
        assert_eq!(t[1].text, "msg4");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_skips_garbage_lines() {
        let dir = tmp_dir();
        append_chat_line(&dir, &line("u", "ok")).unwrap();
        let p = chat_log_path(&dir);
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(b"this is not json\n").unwrap();
        f.write_all(b"\n").unwrap();
        f.write_all(b"{\"user\":\"u\",\"text\":\"ok2\",\"ts_ms\":2}\n")
            .unwrap();
        drop(f);
        let log = read_chat_log(&dir).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[1].text, "ok2");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn from_chat_frame_uses_server_ts() {
        let msg = SyncMessage::Chat {
            user: "alice".into(),
            text: "hi".into(),
            ts_ms: 9999,
        };
        let line = from_chat_frame(&msg, 42).unwrap();
        // server timestamp overrides whatever the client sent
        assert_eq!(line.ts_ms, 42);
    }
}
