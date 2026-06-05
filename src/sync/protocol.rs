//! Wire protocol for the realtime sync layer.
//!
//! Every frame on the WebSocket is a JSON-encoded [`SyncMessage`]. Both the
//! client and the server share this exact schema; Phase 3 (mesh) reuses the
//! same frames over a WebRTC data channel.

use serde::{Deserialize, Serialize};

use blake3;

use super::error::{Result, SyncError};

/// Content type discriminator for [`SyncMessage::FileUpdate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    /// UTF-8 text (markdown, TOML, plain prose).
    Text,
    /// Opaque bytes, base64-encoded in `content_b64`.
    Binary,
}

/// A single frame on the sync wire.
///
/// Variants are intentionally flat — no nesting beyond what the server
/// needs to route. New variants land in their own PRs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// Client → server, first message after WS upgrade. Carries the JWT
    /// and the desired room.
    Join {
        /// JWT bearer token.
        token: String,
        /// Room (project) name to join.
        room: String,
        /// Identity advertised to other clients (e.g. "alice").
        user: String,
    },

    /// Server → client, response to a [`SyncMessage::Join`]. On success
    /// `snapshot` carries the last-known state of every path in the room
    /// (empty if `--persist` is off or the room is new).
    JoinAck {
        ok: bool,
        #[serde(default)]
        snapshot: Vec<FileUpdate>,
        #[serde(default)]
        error: Option<String>,
    },

    /// A file changed. Direction is implicit by who sends it:
    /// the client that detected a local change sends, the server
    /// re-broadcasts to every other client in the room.
    FileUpdate(FileUpdate),

    /// Server → client, after a `FileUpdate` is persisted (only sent
    /// when `--persist` is on; clients use it for at-least-once UI hints).
    Ack {
        /// Path the server acknowledged.
        path: String,
        /// Server-side sequence number (monotonic per room).
        seq: u64,
    },

    /// Either side, last resort — closes the session on receipt.
    Error {
        /// Human-readable reason.
        message: String,
        /// True if the error is fatal and the connection will close.
        fatal: bool,
    },

    /// Server → client, periodic liveness ping.
    Ping {
        /// Server time in milliseconds since epoch.
        ts_ms: u64,
    },

    /// Client → server, response to a [`SyncMessage::Ping`].
    Pong {
        /// Echoed timestamp from the matching [`SyncMessage::Ping`].
        ts_ms: u64,
    },
}

/// Payload for [`SyncMessage::FileUpdate`].
///
/// The path is always relative to the project root, POSIX-style, and must
/// be inside the allowlist (see `allowlist::is_allowed`). The hash is
/// `blake3` of the raw bytes (before encoding) and is what clients use
/// to suppress loops when the server echoes their own write back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileUpdate {
    /// Relative POSIX path, e.g. `memory/auth-analysis.md`.
    pub path: String,
    /// Hex-encoded blake3 hash of the raw bytes.
    pub hash: String,
    /// File size in bytes.
    pub size: u64,
    /// Content type.
    pub kind: ContentKind,
    /// Text content (UTF-8) when `kind == Text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Base64 content when `kind == Binary`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_b64: Option<String>,
    /// Logical author (the user who made the change).
    pub author: String,
    /// Monotonic timestamp in milliseconds since epoch.
    pub ts_ms: u64,
}

impl FileUpdate {
    /// Construct a text `FileUpdate` from raw bytes. The hash is computed
    /// internally so callers can't accidentally lie about it.
    pub fn from_text(path: impl Into<String>, body: &[u8], author: impl Into<String>) -> Self {
        let hash = blake3::hash(body).to_hex().to_string();
        let content = std::str::from_utf8(body).ok().map(str::to_owned);
        Self {
            path: path.into(),
            hash,
            size: body.len() as u64,
            kind: ContentKind::Text,
            content,
            content_b64: None,
            author: author.into(),
            ts_ms: crate::sync::now_ms(),
        }
    }

    /// Construct a binary `FileUpdate` (base64-encoded payload).
    pub fn from_binary(path: impl Into<String>, body: &[u8], author: impl Into<String>) -> Self {
        let hash = blake3::hash(body).to_hex().to_string();
        let content_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, body);
        Self {
            path: path.into(),
            hash,
            size: body.len() as u64,
            kind: ContentKind::Binary,
            content: None,
            content_b64: Some(content_b64),
            author: author.into(),
            ts_ms: crate::sync::now_ms(),
        }
    }
}

impl SyncMessage {
    /// Encode a frame to JSON bytes.
    pub fn encode(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| SyncError::Protocol(format!("encode: {e}")))
    }

    /// Decode a frame from JSON bytes. The deserializer is configured to
    /// reject unknown variants on the tagged enum, so adding a new variant
    /// will explicitly break older clients (which is the desired behavior).
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| SyncError::Protocol(format!("decode: {e}")))
    }

    /// Short tag string for logging (e.g. `"join"`, `"file_update"`).
    pub fn kind(&self) -> &'static str {
        match self {
            SyncMessage::Join { .. } => "join",
            SyncMessage::JoinAck { .. } => "join_ack",
            SyncMessage::FileUpdate(_) => "file_update",
            SyncMessage::Ack { .. } => "ack",
            SyncMessage::Error { .. } => "error",
            SyncMessage::Ping { .. } => "ping",
            SyncMessage::Pong { .. } => "pong",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_join() {
        let m = SyncMessage::Join {
            token: "abc".to_string(),
            room: "myproject".to_string(),
            user: "alice".to_string(),
        };
        let bytes = m.encode().expect("encode");
        let back = SyncMessage::decode(&bytes).expect("decode");
        assert_eq!(m, back);
    }

    #[test]
    fn round_trip_file_update_text() {
        let body = b"# Decision: use blake3\n";
        let upd = FileUpdate::from_text("memory/x.md", body, "alice");
        let m = SyncMessage::FileUpdate(upd);
        let bytes = m.encode().expect("encode");
        let back = SyncMessage::decode(&bytes).expect("decode");
        assert_eq!(m, back);
    }

    #[test]
    fn round_trip_file_update_binary() {
        let body = vec![0u8, 1, 2, 3, 255, 254, 253];
        let upd = FileUpdate::from_binary("skills/foo.bin", &body, "bob");
        let m = SyncMessage::FileUpdate(upd);
        let bytes = m.encode().expect("encode");
        let back = SyncMessage::decode(&bytes).expect("decode");
        assert_eq!(m, back);
    }

    #[test]
    fn round_trip_join_ack_with_snapshot() {
        let m = SyncMessage::JoinAck {
            ok: true,
            snapshot: vec![FileUpdate::from_text("memory/a.md", b"hi", "carol")],
            error: None,
        };
        let bytes = m.encode().expect("encode");
        let back = SyncMessage::decode(&bytes).expect("decode");
        assert_eq!(m, back);
    }

    #[test]
    fn round_trip_error() {
        let m = SyncMessage::Error {
            message: "nope".to_string(),
            fatal: true,
        };
        let bytes = m.encode().expect("encode");
        let back = SyncMessage::decode(&bytes).expect("decode");
        assert_eq!(m, back);
    }

    #[test]
    fn decode_rejects_garbage() {
        let r = SyncMessage::decode(b"{not valid json");
        assert!(matches!(r, Err(SyncError::Protocol(_))));
    }

    #[test]
    fn decode_rejects_unknown_variant() {
        let r = SyncMessage::decode(br#"{"type":"shutdown"}"#);
        assert!(matches!(r, Err(SyncError::Protocol(_))));
    }

    #[test]
    fn file_update_hash_is_deterministic() {
        let a = FileUpdate::from_text("p.md", b"hello", "x");
        let b = FileUpdate::from_text("p.md", b"hello", "y");
        assert_eq!(a.hash, b.hash);
        assert_ne!(a.author, b.author);
    }

    #[test]
    fn file_update_hash_differs_on_content() {
        let a = FileUpdate::from_text("p.md", b"hello", "x");
        let b = FileUpdate::from_text("p.md", b"hellp", "x");
        assert_ne!(a.hash, b.hash);
    }

    #[test]
    fn kind_strings_match_variants() {
        assert_eq!(SyncMessage::Ping { ts_ms: 0 }.kind(), "ping");
        assert_eq!(SyncMessage::Pong { ts_ms: 0 }.kind(), "pong");
        assert_eq!(
            SyncMessage::Ack {
                path: "x".into(),
                seq: 1
            }
            .kind(),
            "ack"
        );
    }
}
