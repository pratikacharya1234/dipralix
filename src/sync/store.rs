//! Server-side persistence for room state.
//!
//! In-memory [`MemStore`] is the default. [`SqliteStore`] is enabled when
//! the server is started with `--persist <path>`. Both implement the
//! [`Store`] trait so the rest of the server doesn't care.

use std::collections::HashMap;
use std::path::Path;

use tokio::sync::Mutex;

use crate::sync::error::{Result, SyncError};
use crate::sync::protocol::FileUpdate;

/// Server-side persistence for a single room's last-known [`FileUpdate`]
/// per path. Implementations must be safe to call from many tasks.
///
/// Methods are `async` because the SQLite-backed impl needs an
/// `async` mutex around the `Connection` (which is `!Sync`).
#[allow(async_fn_in_trait)]
pub trait Store: Send + Sync + 'static {
    /// Return the last [`FileUpdate`] recorded for `path` in `room`, or
    /// `None` if no such path has been seen.
    async fn get(&self, room: &str, path: &str) -> Result<Option<FileUpdate>>;

    /// Return the full state of `room` (path → last update). Used to
    /// seed a newly joined client.
    async fn snapshot(&self, room: &str) -> Result<Vec<FileUpdate>>;

    /// Persist a new value. Must be atomic — if the write fails, the
    /// previous value must remain observable.
    async fn put(&self, room: &str, update: &FileUpdate) -> Result<()>;

    /// All rooms currently known to the store. Used for diagnostics.
    async fn rooms(&self) -> Result<Vec<String>>;
}

/// In-memory store. Default for ephemeral servers.
#[derive(Debug, Default)]
pub struct MemStore {
    /// `room → (path → last update)`
    inner: Mutex<HashMap<String, HashMap<String, FileUpdate>>>,
}

impl MemStore {
    /// Construct a fresh empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Store for MemStore {
    async fn get(&self, room: &str, path: &str) -> Result<Option<FileUpdate>> {
        let guard = self.inner.lock().await;
        Ok(guard.get(room).and_then(|r| r.get(path)).cloned())
    }

    async fn snapshot(&self, room: &str) -> Result<Vec<FileUpdate>> {
        let guard = self.inner.lock().await;
        Ok(guard
            .get(room)
            .map(|r| r.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn put(&self, room: &str, update: &FileUpdate) -> Result<()> {
        let mut guard = self.inner.lock().await;
        guard
            .entry(room.to_string())
            .or_default()
            .insert(update.path.clone(), update.clone());
        Ok(())
    }

    async fn rooms(&self) -> Result<Vec<String>> {
        let guard = self.inner.lock().await;
        Ok(guard.keys().cloned().collect())
    }
}

/// SQLite-backed store. Used when the server is started with `--persist`.
///
/// Schema:
/// ```sql
/// CREATE TABLE IF NOT EXISTS file_state (
///     room    TEXT NOT NULL,
///     path    TEXT NOT NULL,
///     payload TEXT NOT NULL,    -- JSON-encoded FileUpdate
///     updated INTEGER NOT NULL, -- unix ms
///     PRIMARY KEY (room, path)
/// );
/// ```
pub struct SqliteStore {
    conn: Mutex<rusqlite::Connection>,
}

impl SqliteStore {
    /// Open or create the database at `path`. The bundled SQLite is used
    /// so no system library is required.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| SyncError::Store(format!("open {}: {e}", path.display())))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS file_state (
                room    TEXT NOT NULL,
                path    TEXT NOT NULL,
                payload TEXT NOT NULL,
                updated INTEGER NOT NULL,
                PRIMARY KEY (room, path)
             );
             CREATE INDEX IF NOT EXISTS file_state_room ON file_state(room);",
        )
        .map_err(|e| SyncError::Store(format!("schema: {e}")))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl Store for SqliteStore {
    async fn get(&self, room: &str, path: &str) -> Result<Option<FileUpdate>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT payload FROM file_state WHERE room = ?1 AND path = ?2")
            .map_err(|e| SyncError::Store(format!("prepare get: {e}")))?;
        let mut rows = stmt
            .query(rusqlite::params![room, path])
            .map_err(|e| SyncError::Store(format!("query get: {e}")))?;
        match rows
            .next()
            .map_err(|e| SyncError::Store(format!("row get: {e}")))?
        {
            Some(row) => {
                let payload: String = row
                    .get(0)
                    .map_err(|e| SyncError::Store(format!("col get: {e}")))?;
                let upd: FileUpdate = serde_json::from_str(&payload)
                    .map_err(|e| SyncError::Store(format!("decode stored payload: {e}")))?;
                Ok(Some(upd))
            }
            None => Ok(None),
        }
    }

    async fn snapshot(&self, room: &str) -> Result<Vec<FileUpdate>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT payload FROM file_state WHERE room = ?1")
            .map_err(|e| SyncError::Store(format!("prepare snapshot: {e}")))?;
        let rows = stmt
            .query_map(rusqlite::params![room], |row| {
                let payload: String = row.get(0)?;
                Ok(payload)
            })
            .map_err(|e| SyncError::Store(format!("query snapshot: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            let payload = r.map_err(|e| SyncError::Store(format!("row snapshot: {e}")))?;
            let upd: FileUpdate = serde_json::from_str(&payload)
                .map_err(|e| SyncError::Store(format!("decode snapshot row: {e}")))?;
            out.push(upd);
        }
        Ok(out)
    }

    async fn put(&self, room: &str, update: &FileUpdate) -> Result<()> {
        let payload = serde_json::to_string(update)
            .map_err(|e| SyncError::Store(format!("encode for store: {e}")))?;
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO file_state (room, path, payload, updated) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(room, path) DO UPDATE SET payload = excluded.payload, updated = excluded.updated",
            rusqlite::params![room, update.path, payload, update.ts_ms as i64],
        )
        .map_err(|e| SyncError::Store(format!("put: {e}")))?;
        Ok(())
    }

    async fn rooms(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT DISTINCT room FROM file_state")
            .map_err(|e| SyncError::Store(format!("prepare rooms: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                let r: String = row.get(0)?;
                Ok(r)
            })
            .map_err(|e| SyncError::Store(format!("query rooms: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| SyncError::Store(format!("row rooms: {e}")))?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(path: &str, body: &[u8]) -> FileUpdate {
        FileUpdate::from_text(path, body, "tester")
    }

    #[tokio::test]
    async fn mem_store_put_then_get() {
        let s = MemStore::new();
        let u = sample(".dipralix/memory/a.md", b"hi");
        s.put("r1", &u).await.unwrap();
        let got = s.get("r1", ".dipralix/memory/a.md").await.unwrap();
        assert_eq!(got, Some(u));
    }

    #[tokio::test]
    async fn mem_store_get_missing_returns_none() {
        let s = MemStore::new();
        assert!(s.get("r1", "nope.md").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn mem_store_snapshot_includes_all_paths() {
        let s = MemStore::new();
        s.put("r1", &sample(".dipralix/memory/a.md", b"1"))
            .await
            .unwrap();
        s.put("r1", &sample(".dipralix/memory/b.md", b"22"))
            .await
            .unwrap();
        s.put("r2", &sample(".dipralix/memory/c.md", b"333"))
            .await
            .unwrap();
        let snap = s.snapshot("r1").await.unwrap();
        assert_eq!(snap.len(), 2);
        let r2 = s.snapshot("r2").await.unwrap();
        assert_eq!(r2.len(), 1);
    }

    #[tokio::test]
    async fn mem_store_put_overwrites() {
        let s = MemStore::new();
        s.put("r1", &sample(".dipralix/memory/a.md", b"v1"))
            .await
            .unwrap();
        s.put("r1", &sample(".dipralix/memory/a.md", b"v2-longer"))
            .await
            .unwrap();
        let got = s.get("r1", ".dipralix/memory/a.md").await.unwrap().unwrap();
        assert_eq!(got.size, 9);
    }

    #[tokio::test]
    async fn mem_store_rooms_lists_unique_rooms() {
        let s = MemStore::new();
        s.put("r1", &sample(".dipralix/memory/a.md", b"x"))
            .await
            .unwrap();
        s.put("r2", &sample(".dipralix/memory/b.md", b"y"))
            .await
            .unwrap();
        let mut rooms = s.rooms().await.unwrap();
        rooms.sort();
        assert_eq!(rooms, vec!["r1".to_string(), "r2".to_string()]);
    }

    #[tokio::test]
    async fn sqlite_store_round_trip() {
        let dir = std::env::temp_dir().join(format!("dipralix-sync-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        if path.exists() {
            std::fs::remove_file(&path).ok();
        }
        let s = SqliteStore::open(&path).unwrap();
        s.put("r1", &sample(".dipralix/memory/a.md", b"hello"))
            .await
            .unwrap();
        let got = s.get("r1", ".dipralix/memory/a.md").await.unwrap().unwrap();
        assert_eq!(got.size, 5);
        let snap = s.snapshot("r1").await.unwrap();
        assert_eq!(snap.len(), 1);
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[tokio::test]
    async fn sqlite_store_persists_across_reopen() {
        let dir = std::env::temp_dir().join(format!("dipralix-sync-test2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test2.db");
        if path.exists() {
            std::fs::remove_file(&path).ok();
        }
        {
            let s = SqliteStore::open(&path).unwrap();
            s.put("r1", &sample(".dipralix/memory/a.md", b"persisted"))
                .await
                .unwrap();
        }
        let s2 = SqliteStore::open(&path).unwrap();
        let got = s2
            .get("r1", ".dipralix/memory/a.md")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.size, 9);
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }
}
