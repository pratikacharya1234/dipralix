//! Per-room presence roster.
//!
//! Each connected client sends a `Presence` heartbeat every
//! [`HEARTBEAT_INTERVAL`]. The server maintains a per-room roster
//! keyed by user identity and re-broadcasts each heartbeat to the
//! other room members. Clients use the roster to render the §3.2
//! "live cursor" block.
//!
//! Heartbeats are also used to detect dead connections: a user
//! whose last heartbeat is older than [`STALE_AFTER`] is dropped
//! from the roster and announced as `Offline` to the rest of the
//! room.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use super::protocol::{PresenceStatus, SyncMessage};

/// How often a client should send a `Presence` heartbeat.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

/// A user with no heartbeat for this long is considered offline.
pub const STALE_AFTER: Duration = Duration::from_secs(30);

/// One entry in the room roster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceEntry {
    /// Stable user identity (`Join::user`).
    pub user: String,
    /// Latest known status.
    pub status: PresenceStatus,
    /// Latest activity string (e.g. "editing memory/x.md").
    pub activity: Option<String>,
    /// Wall-clock instant of the last heartbeat.
    pub last_seen: Instant,
}

impl PresenceEntry {
    /// True if this entry's `last_seen` is older than [`STALE_AFTER`].
    pub fn is_stale(&self, now: Instant) -> bool {
        now.duration_since(self.last_seen) > STALE_AFTER
    }
}

/// A per-room roster.
#[derive(Debug, Default)]
pub struct Roster {
    /// `user → entry`.
    members: HashMap<String, PresenceEntry>,
}

impl Roster {
    /// Construct an empty roster.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a presence heartbeat. Returns the previous entry for
    /// this user, if any (used by callers to detect transitions).
    pub fn apply(
        &mut self,
        user: String,
        status: PresenceStatus,
        activity: Option<String>,
        now: Instant,
    ) -> Option<PresenceEntry> {
        let prev = self.members.remove(&user);
        self.members.insert(
            user.clone(),
            PresenceEntry {
                user,
                status,
                activity,
                last_seen: now,
            },
        );
        prev
    }

    /// Remove a user from the roster (e.g. on disconnect).
    pub fn remove(&mut self, user: &str) -> Option<PresenceEntry> {
        self.members.remove(user)
    }

    /// Drop entries whose heartbeat is older than `STALE_AFTER`.
    /// Returns the users that were removed (so the caller can
    /// emit `Offline` heartbeats for them).
    pub fn sweep(&mut self, now: Instant) -> Vec<String> {
        let stale: Vec<String> = self
            .members
            .iter()
            .filter(|(_, e)| e.is_stale(now))
            .map(|(u, _)| u.clone())
            .collect();
        for u in &stale {
            self.members.remove(u);
        }
        stale
    }

    /// Snapshot of the current roster.
    pub fn snapshot(&self) -> Vec<PresenceEntry> {
        self.members.values().cloned().collect()
    }

    /// Look up a single entry.
    #[allow(dead_code)]
    pub fn get(&self, user: &str) -> Option<&PresenceEntry> {
        self.members.get(user)
    }

    /// Number of active members.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.members.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

/// Thread-safe wrapper around a `Roster`.
#[derive(Debug, Default, Clone)]
pub struct SharedRoster(Arc<RwLock<Roster>>);

impl SharedRoster {
    /// Construct a new shared roster.
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(Roster::new())))
    }

    /// Apply a presence heartbeat.
    pub async fn apply(&self, msg: &SyncMessage, now: Instant) -> Option<PresenceEntry> {
        let SyncMessage::Presence {
            user,
            status,
            activity,
            ..
        } = msg
        else {
            return None;
        };
        self.0
            .write()
            .await
            .apply(user.clone(), *status, activity.clone(), now)
    }

    /// Remove a user.
    pub async fn remove(&self, user: &str) -> Option<PresenceEntry> {
        self.0.write().await.remove(user)
    }

    /// Sweep stale entries. Returns the users that were removed.
    pub async fn sweep(&self, now: Instant) -> Vec<String> {
        self.0.write().await.sweep(now)
    }

    /// Snapshot of the current roster.
    pub async fn snapshot(&self) -> Vec<PresenceEntry> {
        self.0.read().await.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn entry(user: &str) -> PresenceEntry {
        PresenceEntry {
            user: user.to_string(),
            status: PresenceStatus::Active,
            activity: None,
            last_seen: Instant::now(),
        }
    }

    #[test]
    fn apply_records_heartbeat() {
        let mut r = Roster::new();
        let t0 = Instant::now();
        r.apply("alice".into(), PresenceStatus::Active, Some("x".into()), t0);
        let snap = r.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].user, "alice");
        assert_eq!(snap[0].activity.as_deref(), Some("x"));
    }

    #[test]
    fn apply_replaces_previous_entry() {
        let mut r = Roster::new();
        let t0 = Instant::now();
        r.apply("alice".into(), PresenceStatus::Active, Some("a".into()), t0);
        r.apply(
            "alice".into(),
            PresenceStatus::Idle,
            None,
            t0 + Duration::from_secs(1),
        );
        let snap = r.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].status, PresenceStatus::Idle);
        assert!(snap[0].activity.is_none());
    }

    #[test]
    fn sweep_drops_stale_only() {
        let mut r = Roster::new();
        let t0 = Instant::now();
        r.apply("alice".into(), PresenceStatus::Active, None, t0);
        // bob's heartbeat was a long time ago
        r.apply(
            "bob".into(),
            PresenceStatus::Active,
            None,
            t0 - Duration::from_secs(60),
        );
        let removed = r.sweep(t0);
        assert_eq!(removed, vec!["bob".to_string()]);
        assert_eq!(r.snapshot().len(), 1);
    }

    #[test]
    fn remove_returns_entry() {
        let mut r = Roster::new();
        r.apply("alice".into(), PresenceStatus::Active, None, Instant::now());
        let e = r.remove("alice");
        assert!(e.is_some());
        assert!(r.is_empty());
    }

    #[test]
    fn is_stale_threshold() {
        let t0 = Instant::now();
        let e = entry("alice");
        // within threshold
        assert!(!e.is_stale(t0 + Duration::from_secs(10)));
        // past threshold
        assert!(e.is_stale(t0 + Duration::from_secs(31)));
    }

    #[tokio::test]
    async fn shared_roster_handles_concurrent_writes() {
        let r = SharedRoster::new();
        for u in ["alice", "bob", "carol"] {
            let msg = SyncMessage::Presence {
                user: u.into(),
                status: PresenceStatus::Active,
                activity: None,
                ts_ms: 0,
            };
            r.apply(&msg, Instant::now()).await;
        }
        assert_eq!(r.snapshot().await.len(), 3);
        r.remove("bob").await;
        let snap = r.snapshot().await;
        assert_eq!(snap.len(), 2);
        assert!(snap.iter().all(|e| e.user != "bob"));
    }
}
