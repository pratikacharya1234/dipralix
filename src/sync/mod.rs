//! Realtime sync layer (Phases 1–4).
//!
//! Public surface:
//! - `SyncMessage`, `FileUpdate`, `ContentKind`, `PresenceStatus`,
//!   `ApprovalVoteKind` — the wire schema.
//! - `Store`, `MemStore`, `SqliteStore` — server-side persistence.
//! - `SyncClient`, `ClientConfig` — client-side WebSocket sync.
//! - `start_watching` — debounced filesystem watcher.
//! - `allowlist` — path allowlist (`.dipralix/...` only).
//! - `presence::{Roster, SharedRoster, PresenceEntry}` — Phase 2
//!   liveness/presence tracking.
//! - `chat` — Phase 2 append-only team chat.
//! - `mesh` — Phase 3 transport-agnostic sync over P2P (stubs
//!   here; see the module docs).

#![deny(clippy::all)]
#![allow(clippy::module_name_repetitions)]

use std::time::{SystemTime, UNIX_EPOCH};

pub mod allowlist;
pub mod chat;
pub mod client;
pub mod crypto;
pub mod error;
pub mod mesh;
pub mod presence;
pub mod protocol;
pub mod store;
pub mod watcher;

// Public re-exports for the library's users. Marked `#[allow(unused_imports)]`
// because the binary crate's `main.rs` only references a small subset; the
// rest of the surface is part of the library's public API and is exercised
// by the integration test (`tests/sync_integration.rs`).
#[allow(unused_imports)]
pub use allowlist::{is_allowed, validate, DIPRALIX_DIR};
#[allow(unused_imports)]
pub use chat::{
    append_chat_line, chat_log_path, read_chat_log, tail as tail_chat, ChatLine, CHAT_LOG_FILE,
};
#[allow(unused_imports)]
pub use client::{ClientConfig, SyncClient, WsSink, WsStream};
#[allow(unused_imports)]
pub use crypto::{Psk, PSK_LEN};
#[allow(unused_imports)]
pub use error::{Result, SyncError};
#[allow(unused_imports)]
pub use mesh::{
    MeshConfig, MeshPeer, MeshRole, MeshSession, MeshTransport, SyncTransport, WsTransport,
};
#[allow(unused_imports)]
pub use presence::{PresenceEntry, Roster, SharedRoster, HEARTBEAT_INTERVAL, STALE_AFTER};
#[allow(unused_imports)]
pub use protocol::{ApprovalVoteKind, ContentKind, FileUpdate, PresenceStatus, SyncMessage};
#[allow(unused_imports)]
pub use store::{MemStore, SqliteStore, Store};
#[allow(unused_imports)]
pub use watcher::{start_watching, ChangeEvent, ChangeKind, DEBOUNCE, SYNCED_SUBDIRS};

/// Current monotonic-ish time in milliseconds since the Unix epoch.
/// Used by `FileUpdate::from_text` / `FileUpdate::from_binary` to
/// stamp outbound frames.
#[allow(clippy::cast_possible_truncation)]
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_ms_is_monotonic_wall_clock() {
        let a = now_ms();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let b = now_ms();
        assert!(b >= a);
        assert!(b - a < 5_000, "clock went backwards or jumped");
    }
}
