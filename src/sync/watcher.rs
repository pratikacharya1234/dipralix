//! Filesystem watcher for `.dipralix/` subdirectories.
//!
//! Wraps `notify`'s poll watcher with a 150 ms debounce window and
//! scopes events to the synced subpaths (`memory/`, `plans/`,
//! `skills/`). Path validation and hash-based dedup live in the
//! consumer (see `client.rs`).
//!
//! We deliberately use [`notify::PollWatcher`] (not the
//! platform-default recommended watcher) because the FSEvents backend
//! on macOS can silently drop events for paths written in test and
//! CI-style harnesses, and the 1-second DoD latency for sync is
//! easily met by a 100 ms poll interval.

use std::path::{Path, PathBuf};
use std::sync::mpsc as smpsc;
use std::time::{Duration, Instant};

use notify::{
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    Event, EventKind, PollWatcher, RecursiveMode, Watcher,
};
use tokio::sync::mpsc;

use crate::sync::error::{Result, SyncError};

/// Subdirectories of `.dipralix/` that the watcher monitors.
pub const SYNCED_SUBDIRS: &[&str] = &["memory", "plans", "skills"];

/// Debounce window — `notify` is chatty on macOS/Windows, and many
/// editors write twice (truncate, then re-fill). 150 ms is enough to
/// collapse the duplicates without pushing latency past the 1 s DoD.
pub const DEBOUNCE: Duration = Duration::from_millis(150);

/// How often the poll watcher checks the watched directories.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// A single filesystem change, after debounce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeEvent {
    /// Path relative to the project root, POSIX-style.
    pub rel_path: String,
    /// What happened.
    pub kind: ChangeKind,
}

/// Coarse change kind — we only care about the existence/category, not
/// the exact `notify` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// File was created or modified (content may have changed).
    Upsert,
    /// File was removed.
    Remove,
}

/// Begin watching `project_root/.dipralix/{memory,plans,skills}`.
///
/// The returned receiver yields debounced [`ChangeEvent`]s. The
/// underlying `notify` handle is held by a background task; dropping
/// the receiver eventually stops the task and releases the handle.
pub async fn start_watching(project_root: PathBuf) -> Result<mpsc::Receiver<ChangeEvent>> {
    // Bridge notify's worker thread (non-tokio) to the tokio mpsc
    // consumer via a std::sync::mpsc + dedicated relay thread.
    let (raw_tx, raw_rx) = smpsc::channel::<Event>();
    let (out_tx, out_rx) = mpsc::channel::<ChangeEvent>(256);

    let mut notify_watcher: PollWatcher = notify::PollWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(ev) = res {
                let _ = raw_tx.send(ev);
            }
        },
        notify::Config::default().with_poll_interval(POLL_INTERVAL),
    )
    .map_err(|e| SyncError::Transport(format!("notify init: {e}")))?;

    let dip = project_root.join(crate::sync::allowlist::DIPRALIX_DIR);
    if !dip.exists() {
        std::fs::create_dir_all(&dip).map_err(SyncError::Io)?;
    }
    for sub in SYNCED_SUBDIRS {
        let p = dip.join(sub);
        if !p.exists() {
            std::fs::create_dir_all(&p).map_err(SyncError::Io)?;
        }
        notify_watcher
            .watch(&p, RecursiveMode::Recursive)
            .map_err(|e| SyncError::Transport(format!("watch {}: {e}", p.display())))?;
    }

    tokio::spawn(async move {
        // Hold the watcher for the lifetime of the task. When the
        // consumer drops `out_rx`, we stop reading `raw_rx`, the
        // notify callback's `tx.send` starts erroring, and we exit —
        // dropping the watcher, which stops polling.
        let _w = notify_watcher;
        relay_loop(project_root, raw_rx, out_tx).await;
    });

    Ok(out_rx)
}

/// Bridge `std::sync::mpsc` → `tokio::sync::mpsc` and debounce.
async fn relay_loop(
    project_root: PathBuf,
    raw_rx: smpsc::Receiver<Event>,
    out_tx: mpsc::Sender<ChangeEvent>,
) {
    use std::collections::HashMap;

    let mut last_seen: HashMap<String, Instant> = HashMap::new();
    let mut pending_kind: HashMap<String, ChangeKind> = HashMap::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(25));
    // The first tick fires immediately; skip it so we don't emit on
    // startup before any events have been seen.
    ticker.tick().await;

    // Move the std receiver into a dedicated thread that forwards
    // events into the tokio mpsc. We can't `await raw_rx.recv()`
    // directly (it blocks), and a `spawn_blocking` task would be tied
    // to a single blocking call; the thread model is simpler and the
    // bridge has no async state of its own.
    let (relay_tx, mut relay_rx) = mpsc::channel::<Event>(512);
    std::thread::spawn(move || {
        while let Ok(ev) = raw_rx.recv() {
            if relay_tx.blocking_send(ev).is_err() {
                break;
            }
        }
    });

    loop {
        tokio::select! {
            maybe_ev = relay_rx.recv() => {
                let Some(ev) = maybe_ev else { break };
                for p in &ev.paths {
                    let Some(rel) = to_rel_posix(&project_root, p) else { continue };
                    if is_synced_dir(&rel) {
                        // PollWatcher reports the directory whose
                        // contents changed; expand to per-file events.
                        // The client.rs hash-dedup skips unchanged files.
                        for (file_rel, k) in expand_dir_to_files(&project_root, &rel) {
                            last_seen.insert(file_rel.clone(), Instant::now());
                            pending_kind.insert(file_rel, k);
                        }
                    } else if let Some(k) = map_kind(&ev.kind) {
                        if is_synced(&rel) {
                            last_seen.insert(rel.clone(), Instant::now());
                            pending_kind.insert(rel, k);
                        }
                    }
                }
            }
            _ = ticker.tick() => {
                let now = Instant::now();
                let ready: Vec<(String, ChangeKind)> = last_seen
                    .iter()
                    .filter(|(_, t)| now.duration_since(**t) >= DEBOUNCE)
                    .map(|(p, _)| {
                        let k = pending_kind.remove(p).unwrap_or(ChangeKind::Upsert);
                        (p.clone(), k)
                    })
                    .collect();
                for (p, k) in ready {
                    last_seen.remove(&p);
                    if out_tx.send(ChangeEvent { rel_path: p, kind: k }).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

/// Walk a synced dir and emit one [`ChangeKind::Upsert`] event per
/// regular file inside. Symlinks and other irregular entries are
/// skipped. Empty dirs emit no events.
fn expand_dir_to_files(root: &Path, rel_dir: &str) -> Vec<(String, ChangeKind)> {
    let abs = root.join(rel_dir);
    let Ok(entries) = std::fs::read_dir(&abs) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let rel = if rel_dir.ends_with('/') {
            format!("{rel_dir}{name}")
        } else {
            format!("{rel_dir}/{name}")
        };
        out.push((rel, ChangeKind::Upsert));
    }
    out
}

fn map_kind(kind: &EventKind) -> Option<ChangeKind> {
    match kind {
        EventKind::Create(CreateKind::File) | EventKind::Create(CreateKind::Any) => {
            Some(ChangeKind::Upsert)
        }
        EventKind::Modify(ModifyKind::Data(_))
        | EventKind::Modify(ModifyKind::Any)
        | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => Some(ChangeKind::Upsert),
        EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
            Some(ChangeKind::Remove)
        }
        _ => None,
    }
}

fn to_rel_posix(root: &Path, p: &Path) -> Option<String> {
    let rel = p.strip_prefix(root).ok()?;
    let mut s = String::new();
    for (i, c) in rel.components().enumerate() {
        if i > 0 {
            s.push('/');
        }
        s.push_str(&c.as_os_str().to_string_lossy());
    }
    Some(s)
}

fn is_synced(rel: &str) -> bool {
    let prefix = format!("{}/", crate::sync::allowlist::DIPRALIX_DIR);
    if let Some(rest) = rel.strip_prefix(&prefix) {
        SYNCED_SUBDIRS
            .iter()
            .any(|s| rest.starts_with(&format!("{s}/")) || rest == *s)
    } else {
        false
    }
}

/// True if `rel` names one of the synced subdirs directly
/// (e.g. `.dipralix/memory`). Used to detect dir-level events on
/// platforms (and with poll watchers) that report the parent
/// directory rather than the individual file.
fn is_synced_dir(rel: &str) -> bool {
    let prefix = format!("{}/", crate::sync::allowlist::DIPRALIX_DIR);
    if let Some(rest) = rel.strip_prefix(&prefix) {
        SYNCED_SUBDIRS.contains(&rest)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn to_rel_posix_strips_root() {
        let root = std::path::PathBuf::from("/tmp/abc");
        let p = std::path::PathBuf::from("/tmp/abc/.dipralix/memory/x.md");
        assert_eq!(
            to_rel_posix(&root, &p).as_deref(),
            Some(".dipralix/memory/x.md")
        );
    }

    #[test]
    fn to_rel_posix_returns_none_outside_root() {
        let root = std::path::PathBuf::from("/tmp/abc");
        let p = std::path::PathBuf::from("/etc/passwd");
        assert!(to_rel_posix(&root, &p).is_none());
    }

    #[test]
    fn is_synced_accepts_known_subdirs() {
        assert!(is_synced(".dipralix/memory/x.md"));
        assert!(is_synced(".dipralix/plans/y.md"));
        assert!(is_synced(".dipralix/skills/z.md"));
    }

    #[test]
    fn is_synced_rejects_other_subdirs() {
        assert!(!is_synced(".dipralix/context/x.md"));
        assert!(!is_synced("src/main.rs"));
        assert!(!is_synced(".dipralix"));
    }

    #[test]
    fn map_kind_translates_known_events() {
        assert_eq!(
            map_kind(&EventKind::Create(CreateKind::File)),
            Some(ChangeKind::Upsert)
        );
        assert_eq!(
            map_kind(&EventKind::Remove(RemoveKind::File)),
            Some(ChangeKind::Remove)
        );
        assert_eq!(
            map_kind(&EventKind::Access(notify::event::AccessKind::Open(
                notify::event::AccessMode::Read
            ))),
            None
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_watching_emits_change_for_real_file() {
        let dir = std::path::PathBuf::from("/tmp").join(format!(
            "dipralix-watch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".dipralix/memory")).unwrap();

        let mut rx = start_watching(dir.clone()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;

        std::fs::write(dir.join(".dipralix/memory/x.md"), b"hello").unwrap();
        std::fs::write(dir.join(".dipralix/memory/x.md"), b"hello world").unwrap();

        let mut got: Option<ChangeEvent> = None;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_millis(250), rx.recv()).await {
                Ok(Some(ev)) if ev.rel_path == ".dipralix/memory/x.md" => {
                    got = Some(ev);
                    break;
                }
                Ok(Some(_)) => continue,
                _ => continue,
            }
        }
        assert!(got.is_some(), "expected a change event for x.md");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
