//! WebSocket sync client.
//!
//! Connects to a `dipralix-server`, performs the JWT handshake, and
//! runs a bidirectional loop:
//! - local filesystem changes → outbound `FileUpdate` frames
//! - inbound `FileUpdate` frames → local file writes (with hash dedup
//!   to suppress server-echo loops)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, info, warn};

use blake3;

use super::allowlist;
use super::error::{Result, SyncError};
use super::protocol::{FileUpdate, SyncMessage};
use super::watcher;

/// One end of a split WebSocket connection.
pub type WsSink = futures_util::stream::SplitSink<
    WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;
/// Other end of a split WebSocket connection.
pub type WsStream =
    futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>;

/// Configuration for [`SyncClient::connect`].
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// WebSocket URL, e.g. `ws://127.0.0.1:7878`.
    pub server: String,
    /// JWT bearer token.
    pub token: String,
    /// Room (project) name to join.
    pub room: String,
    /// Identity advertised to other clients.
    pub user: String,
    /// Project root. The watcher scopes to `<root>/.dipralix/`.
    pub project_root: PathBuf,
}

/// Sync client. One instance = one room connection.
pub struct SyncClient {
    cfg: ClientConfig,
    /// `path → last-known blake3 hash`. Used to skip watcher events
    /// whose content already matches the last value we sent *or*
    /// received — the server-echo case.
    last_known: Arc<Mutex<HashMap<String, String>>>,
}

impl SyncClient {
    /// Build a new client. Does not connect.
    pub fn new(cfg: ClientConfig) -> Self {
        Self {
            cfg,
            last_known: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Connect, authenticate, and run the sync loop until the user
    /// interrupts (Ctrl-C) or the server closes the connection.
    pub async fn run(&self) -> Result<()> {
        loop {
            match self.run_once().await {
                Ok(()) => {
                    info!("sync session ended cleanly");
                    return Ok(());
                }
                Err(e) if e.is_fatal() => {
                    warn!(error = %e, "fatal sync error, giving up");
                    return Err(e);
                }
                Err(e) => {
                    warn!(error = %e, "sync error, reconnecting in 2s");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn run_once(&self) -> Result<()> {
        let url = normalize_ws_url(&self.cfg.server)?;
        info!(server = %url, room = %self.cfg.room, user = %self.cfg.user, "connecting");

        let (ws, _resp) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| SyncError::Transport(format!("connect: {e}")))?;
        let (mut sink, mut stream) = ws.split();

        let join = SyncMessage::Join {
            token: self.cfg.token.clone(),
            room: self.cfg.room.clone(),
            user: self.cfg.user.clone(),
        };
        send_frame(&mut sink, &join).await?;

        let ack = wait_for_kind(&mut stream, "join_ack").await?;
        let snapshot = match ack {
            SyncMessage::JoinAck {
                ok: false, error, ..
            } => {
                return Err(SyncError::Auth(
                    error.unwrap_or_else(|| "join rejected".into()),
                ));
            }
            SyncMessage::JoinAck { snapshot, .. } => snapshot,
            other => {
                return Err(SyncError::Protocol(format!(
                    "expected JoinAck, got {}",
                    other.kind()
                )));
            }
        };
        info!(snapshot_size = snapshot.len(), "joined room");

        for upd in &snapshot {
            self.apply_remote_update(upd).await?;
        }

        let mut changes = watcher::start_watching(self.cfg.project_root.clone()).await?;

        loop {
            tokio::select! {
                biased;
                incoming = stream.next() => {
                    let Some(msg_res) = incoming else {
                        info!("server closed the connection");
                        return Ok(());
                    };
                    let msg_text = match msg_res {
                        Ok(Message::Text(t)) => t,
                        Ok(Message::Binary(b)) => String::from_utf8_lossy(&b).into_owned(),
                        Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
                        Ok(Message::Close(_)) => return Ok(()),
                        Ok(other) => {
                            debug!(?other, "ignoring non-text frame");
                            continue;
                        }
                        Err(e) => return Err(SyncError::Transport(format!("recv: {e}"))),
                    };
                    let parsed = SyncMessage::decode(msg_text.as_bytes())?;
                    self.handle_inbound(parsed, &mut sink).await?;
                }
                local = changes.recv() => {
                    let Some(ev) = local else { return Ok(()); };
                    self.handle_local_change(ev, &mut sink).await?;
                }
            }
        }
    }

    async fn handle_inbound(&self, msg: SyncMessage, sink: &mut WsSink) -> Result<()> {
        match msg {
            SyncMessage::FileUpdate(upd) => self.apply_remote_update(&upd).await,
            SyncMessage::Ping { ts_ms } => send_frame(sink, &SyncMessage::Pong { ts_ms }).await,
            SyncMessage::Pong { .. } | SyncMessage::Ack { .. } => Ok(()),
            SyncMessage::Error { message, fatal } => {
                warn!(message, fatal, "server error");
                if fatal {
                    Err(SyncError::Protocol(format!("server fatal: {message}")))
                } else {
                    Ok(())
                }
            }
            SyncMessage::Join { .. } | SyncMessage::JoinAck { .. } => Err(SyncError::Protocol(
                "unexpected Join/JoinAck after handshake".into(),
            )),
            // Phase 2: presence / chat / approval frames are
            // currently observed but not yet surfaced in the CLI.
            // The server is authoritative for them; the client
            // only needs to acknowledge receipt so the sender's
            // window can advance. We log at debug to keep the
            // production log clean.
            SyncMessage::Presence { user, status, .. } => {
                debug!(%user, ?status, "presence update");
                Ok(())
            }
            SyncMessage::Chat { user, text, ts_ms } => {
                info!(%user, ts_ms, "chat: {text}");
                Ok(())
            }
            SyncMessage::ApprovalRequest {
                request_id,
                action,
                requester,
                required_approvers,
                ..
            } => {
                info!(%request_id, %action, %requester, required_approvers, "approval request");
                Ok(())
            }
            SyncMessage::ApprovalVote {
                request_id,
                voter,
                vote,
                ..
            } => {
                info!(?vote, %request_id, %voter, "approval vote");
                Ok(())
            }
            SyncMessage::ApprovalDecision {
                request_id,
                approved,
                ..
            } => {
                info!(%request_id, approved, "approval decision");
                Ok(())
            }
        }
    }

    async fn handle_local_change(&self, ev: watcher::ChangeEvent, sink: &mut WsSink) -> Result<()> {
        if !allowlist::is_allowed(&ev.rel_path) {
            debug!(path = %ev.rel_path, "skipping non-allowed local change");
            return Ok(());
        }
        let abs = self.cfg.project_root.join(&ev.rel_path);
        let body = match std::fs::read(&abs) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if ev.kind == watcher::ChangeKind::Remove {
                    debug!(path = %ev.rel_path, "remote-only remove (skip in Phase 1)");
                }
                return Ok(());
            }
            Err(e) => return Err(SyncError::Io(e)),
        };

        let hash = blake3::hash(&body).to_hex().to_string();
        {
            let mut cache = self.last_known.lock().await;
            if cache.get(&ev.rel_path) == Some(&hash) {
                debug!(path = %ev.rel_path, "skipping echo / unchanged content");
                return Ok(());
            }
            cache.insert(ev.rel_path.clone(), hash.clone());
        }

        let upd = FileUpdate::from_text(&ev.rel_path, &body, &self.cfg.user);
        send_frame(sink, &SyncMessage::FileUpdate(upd)).await?;
        info!(path = %ev.rel_path, size = body.len(), "sent local change");
        Ok(())
    }

    async fn apply_remote_update(&self, upd: &FileUpdate) -> Result<()> {
        if !allowlist::is_allowed(&upd.path) {
            return Err(SyncError::PathNotAllowed(upd.path.clone()));
        }
        let abs = self.cfg.project_root.join(&upd.path);
        if let Ok(existing) = std::fs::read(&abs) {
            let existing_hash = blake3::hash(&existing).to_hex().to_string();
            if existing_hash == upd.hash {
                debug!(path = %upd.path, "remote update already matches local");
                return Ok(());
            }
        }
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).map_err(SyncError::Io)?;
        }
        let bytes = match &upd.content {
            Some(text) => text.as_bytes().to_vec(),
            None => match &upd.content_b64 {
                Some(b64) => {
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
                        .map_err(|e| SyncError::Protocol(format!("base64: {e}")))?
                }
                None => Vec::new(),
            },
        };
        std::fs::write(&abs, &bytes).map_err(SyncError::Io)?;
        {
            let mut cache = self.last_known.lock().await;
            cache.insert(upd.path.clone(), upd.hash.clone());
        }
        info!(
            path = %upd.path,
            size = bytes.len(),
            author = %upd.author,
            "applied remote change"
        );
        Ok(())
    }
}

async fn send_frame(sink: &mut WsSink, msg: &SyncMessage) -> Result<()> {
    let bytes = msg.encode()?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    sink.send(Message::Text(text))
        .await
        .map_err(|e| SyncError::Transport(format!("send {}: {e}", msg.kind())))
}

async fn wait_for_kind(stream: &mut WsStream, kind: &'static str) -> Result<SyncMessage> {
    while let Some(next) = stream.next().await {
        let raw = match next? {
            Message::Text(t) => t,
            Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => {
                return Err(SyncError::Transport("server closed before reply".into()));
            }
            other => {
                debug!(?other, "ignoring non-text frame in wait_for_kind");
                continue;
            }
        };
        let parsed = SyncMessage::decode(raw.as_bytes())?;
        if parsed.kind() == kind {
            return Ok(parsed);
        }
        debug!(
            expected = kind,
            got = parsed.kind(),
            "skipping out-of-order frame"
        );
    }
    Err(SyncError::Transport("stream ended unexpectedly".into()))
}

fn normalize_ws_url(s: &str) -> Result<String> {
    if s.starts_with("ws://") || s.starts_with("wss://") {
        Ok(s.to_string())
    } else if let Some(rest) = s.strip_prefix("http://") {
        Ok(format!("ws://{rest}"))
    } else if let Some(rest) = s.strip_prefix("https://") {
        Ok(format!("wss://{rest}"))
    } else {
        Err(SyncError::InvalidPath(format!("bad server url: {s}")))
    }
}

/// Project-root convenience for callers that don't need a full
/// [`ClientConfig`].
pub fn project_root_from(p: impl AsRef<Path>) -> PathBuf {
    p.as_ref().to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ws_url_passthrough() {
        assert_eq!(normalize_ws_url("ws://x:1").unwrap(), "ws://x:1");
        assert_eq!(normalize_ws_url("wss://x:1").unwrap(), "wss://x:1");
    }

    #[test]
    fn normalize_ws_url_http_to_ws() {
        assert_eq!(normalize_ws_url("http://x:1").unwrap(), "ws://x:1");
        assert_eq!(normalize_ws_url("https://x:1").unwrap(), "wss://x:1");
    }

    #[test]
    fn normalize_ws_url_rejects_garbage() {
        assert!(normalize_ws_url("ftp://x:1").is_err());
    }
}
