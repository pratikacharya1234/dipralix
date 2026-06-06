//! Typed errors for the realtime sync subsystem.
//!
//! Use [`SyncError`] at library boundaries; convert to `anyhow::Error` at the
//! `main` boundary. No `.unwrap()` or `.expect()` in non-test code paths —
//! surface failures here instead.

use thiserror::Error;

/// All errors that can occur in the realtime sync stack.
///
/// The server, the client, the watcher, the store, and the protocol parser
/// all funnel into this single enum so callers can match on a closed set
/// without digging through `String` payloads.
#[derive(Debug, Error)]
pub enum SyncError {
    /// JWT was missing, malformed, expired, or signed by a different secret.
    #[error("authentication failed: {0}")]
    Auth(String),

    /// I/O failure (file read/write, socket, watcher).
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// The wire format was violated: bad JSON, unknown variant, missing field.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Persistence layer (SQLite) failure.
    #[error("store error: {0}")]
    Store(String),

    /// WebSocket transport failure.
    #[error("transport error: {0}")]
    Transport(String),

    /// Underlying tungstenite error (used by the `?` operator in
    /// transport call sites; the [`SyncError::Transport`] variant is
    /// the user-facing form). Boxed because the inner type is 136 B
    /// and would balloon the [`SyncError`] size.
    #[error("tungstenite error: {0}")]
    Tungstenite(Box<tokio_tungstenite::tungstenite::Error>),

    /// The path being synced is outside the allowlist (e.g. source code,
    /// `config.local`, anything containing an API key).
    #[error("path rejected by allowlist: {0}")]
    PathNotAllowed(String),

    /// A path was rejected for some other reason (e.g. not a `.dipralix/`
    /// subpath, or absolute path traversal).
    #[error("invalid path: {0}")]
    InvalidPath(String),

    /// Catch-all for unexpected internal failures.
    #[error("internal error: {0}")]
    Internal(String),
}

impl SyncError {
    /// Returns true if this error should terminate the current sync session
    /// rather than be retried. Used by the client loop to decide whether to
    /// drop the connection or back off and reconnect.
    pub fn is_fatal(&self) -> bool {
        matches!(self, SyncError::Auth(_) | SyncError::InvalidPath(_))
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for SyncError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        SyncError::Tungstenite(Box::new(e))
    }
}

/// Convenience alias used throughout the sync module.
pub type Result<T> = std::result::Result<T, SyncError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_errors_are_fatal() {
        let e = SyncError::Auth("expired".to_string());
        assert!(e.is_fatal());
    }

    #[test]
    fn invalid_path_is_fatal() {
        let e = SyncError::InvalidPath("../etc/passwd".to_string());
        assert!(e.is_fatal());
    }

    #[test]
    fn transport_errors_are_recoverable() {
        let e = SyncError::Transport("connection reset".to_string());
        assert!(!e.is_fatal());
    }

    #[test]
    fn io_errors_are_recoverable() {
        let e = SyncError::Io(std::io::Error::other("disk full"));
        assert!(!e.is_fatal());
    }

    #[test]
    fn display_messages_are_meaningful() {
        let e = SyncError::Protocol("unexpected EOF".to_string());
        let msg = format!("{e}");
        assert!(msg.contains("protocol error"));
        assert!(msg.contains("unexpected EOF"));
    }
}
