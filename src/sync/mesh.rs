//! Phase 3: P2P mesh transport.
//!
//! Provides a [`SyncTransport`] trait that abstracts over the
//! transport so [`crate::sync::client::SyncClient`] is unchanged
//! when switching between WebSocket (server-mediated) and direct
//! peer-to-peer.
//!
//! In Phase 3 the actual WebRTC data-channel wiring is left as a
//! stub: [`MeshTransport`] is a `tokio::sync::mpsc`-backed loopback
//! channel that two in-process peers can use to verify the
//! protocol refactor without pulling in a real WebRTC stack.
//! Adding a real `webrtc` data-channel implementation only
//! requires another `impl SyncTransport` and a different
//! [`MeshConfig`].
//!
//! Public surface (Phase 3 minimum):
//! - [`SyncTransport`] — async send/recv trait (object-safe).
//! - [`WsTransport`] — wraps a `tokio_tungstenite` stream so the
//!   server path keeps working through the trait.
//! - [`MeshTransport`] — bidirectional mpsc channel for in-process
//!   loopback tests.
//! - [`MeshSession`] / [`MeshConfig`] / [`MeshPeer`] / [`MeshRole`]
//!   — high-level P2P session that connects a `SyncClient` to a
//!   transport.
//!
//! The trait uses `async_trait` so it stays dyn-compatible; tests
//! use concrete `MeshTransport` and skip the dyn layer.

use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use super::error::{Result, SyncError};
use super::protocol::SyncMessage;

/// Abstraction over a frame-oriented transport. Implemented by
/// [`WsTransport`] (Phase 1, server path) and [`MeshTransport`]
/// (Phase 3, in-process loopback). A real WebRTC data-channel
/// implementation only needs another `impl`.
#[async_trait]
pub trait SyncTransport: Send {
    /// Send one frame. Returns `Err(SyncError::Transport)` if the
    /// channel is closed.
    async fn send(&mut self, msg: &SyncMessage) -> Result<()>;

    /// Receive the next frame. Returns `Ok(None)` if the channel
    /// closed cleanly.
    async fn recv(&mut self) -> Result<Option<SyncMessage>>;

    /// Half-close the channel. After this call `send` will fail
    /// but `recv` may still drain buffered frames.
    async fn close(&mut self) -> Result<()>;
}

/// Wrap an existing WebSocket stream so it implements
/// [`SyncTransport`]. This is what the server-mediated path uses
/// once `SyncClient` is generalized to a transport trait.
pub struct WsTransport<S> {
    inner: S,
}

impl<S> WsTransport<S> {
    /// Wrap a tungstenite WebSocket stream.
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<S> SyncTransport for WsTransport<S>
where
    S: futures_util::Sink<WsMessage, Error = WsError>
        + futures_util::Stream<Item = std::result::Result<WsMessage, WsError>>
        + Unpin
        + Send,
{
    async fn send(&mut self, msg: &SyncMessage) -> Result<()> {
        let bytes = msg.encode()?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        self.inner
            .send(WsMessage::Text(text))
            .await
            .map_err(|e| SyncError::Transport(format!("ws send: {e}")))
    }

    async fn recv(&mut self) -> Result<Option<SyncMessage>> {
        loop {
            match self.inner.next().await {
                Some(Ok(WsMessage::Text(t))) => {
                    return Ok(Some(SyncMessage::decode(t.as_bytes())?))
                }
                Some(Ok(WsMessage::Binary(b))) => return Ok(Some(SyncMessage::decode(&b)?)),
                Some(Ok(WsMessage::Close(_))) | None => return Ok(None),
                Some(Ok(_)) => continue, // Ping/Pong dropped at this layer
                Some(Err(e)) => return Err(SyncError::Transport(format!("ws recv: {e}"))),
            }
        }
    }

    async fn close(&mut self) -> Result<()> {
        let _ = self.inner.send(WsMessage::Close(None)).await;
        Ok(())
    }
}

/// A bidirectional loopback transport backed by `tokio::sync::mpsc`.
/// Two `MeshTransport` halves point at each other and can be used
/// in tests to exercise the protocol without a real network.
#[derive(Debug)]
pub struct MeshTransport {
    rx: mpsc::Receiver<SyncMessage>,
    tx: Option<mpsc::Sender<SyncMessage>>,
}

impl MeshTransport {
    /// Construct a connected pair.
    pub fn pair() -> (MeshTransport, MeshTransport) {
        let (a_tx, a_rx) = mpsc::channel(64);
        let (b_tx, b_rx) = mpsc::channel(64);
        (
            MeshTransport {
                rx: a_rx,
                tx: Some(b_tx),
            },
            MeshTransport {
                rx: b_rx,
                tx: Some(a_tx),
            },
        )
    }
}

#[async_trait]
impl SyncTransport for MeshTransport {
    async fn send(&mut self, msg: &SyncMessage) -> Result<()> {
        let Some(tx) = self.tx.as_ref() else {
            return Err(SyncError::Transport("mesh send: closed".into()));
        };
        tx.send(msg.clone())
            .await
            .map_err(|e| SyncError::Transport(format!("mesh send: {e}")))
    }

    async fn recv(&mut self) -> Result<Option<SyncMessage>> {
        Ok(self.rx.recv().await)
    }

    async fn close(&mut self) -> Result<()> {
        // Dropping the sender closes the send side. The peer can
        // still drain whatever is left in our rx by reading until
        // the channel reports closed.
        self.tx.take();
        Ok(())
    }
}

/// Role in a P2P mesh — initiator or responder. Mostly cosmetic
/// for now; reserved for the WebRTC offer/answer exchange in a
/// real Phase 3 implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshRole {
    Initiator,
    Responder,
}

/// Identity of a remote peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPeer {
    /// Stable user identity.
    pub user: String,
    /// Optional transport-level identifier (room, IP:port, etc).
    pub addr: Option<String>,
}

impl MeshPeer {
    /// Convenience constructor.
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            addr: None,
        }
    }
}

/// Configuration for opening a mesh session.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    /// Room name (used to scope the discovery; the room is
    /// otherwise irrelevant to the transport itself).
    pub room: String,
    /// Local peer identity.
    pub local: MeshPeer,
    /// How long to wait for the first frame from the peer before
    /// giving up.
    pub handshake_timeout: Duration,
}

impl MeshConfig {
    /// Build a config with sensible defaults.
    pub fn new(room: impl Into<String>, local_user: impl Into<String>) -> Self {
        Self {
            room: room.into(),
            local: MeshPeer::new(local_user),
            handshake_timeout: Duration::from_secs(5),
        }
    }
}

/// High-level P2P session that owns a transport and provides a
/// uniform `send_frame` / `recv_frame` API to upper layers
/// (e.g. the future `SyncClient` extension).
pub struct MeshSession<T: SyncTransport> {
    pub config: MeshConfig,
    pub role: MeshRole,
    pub peer: Option<MeshPeer>,
    transport: T,
}

impl<T: SyncTransport> MeshSession<T> {
    /// Wrap a transport in a session.
    pub fn new(config: MeshConfig, role: MeshRole, transport: T) -> Self {
        Self {
            config,
            role,
            peer: None,
            transport,
        }
    }

    /// Send a frame over the mesh transport.
    pub async fn send_frame(&mut self, msg: &SyncMessage) -> Result<()> {
        self.transport.send(msg).await
    }

    /// Receive the next frame.
    pub async fn recv_frame(&mut self) -> Result<Option<SyncMessage>> {
        self.transport.recv().await
    }

    /// Close the session.
    pub async fn close(&mut self) -> Result<()> {
        self.transport.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::protocol::{FileUpdate, SyncMessage};

    #[tokio::test]
    async fn mesh_transport_pair_exchanges_frame() {
        let (mut a, mut b) = MeshTransport::pair();
        let frame = SyncMessage::FileUpdate(FileUpdate::from_text("memory/x.md", b"hi", "alice"));
        a.send(&frame).await.unwrap();
        let back = b.recv().await.unwrap().unwrap();
        match back {
            SyncMessage::FileUpdate(u) => assert_eq!(u.path, "memory/x.md"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn mesh_transport_close_drains() {
        let (mut a, mut b) = MeshTransport::pair();
        a.close().await.unwrap();
        let r = b.recv().await.unwrap();
        assert!(r.is_none());
    }

    #[tokio::test]
    async fn mesh_session_wraps_transport() {
        let (a, b) = MeshTransport::pair();
        let cfg = MeshConfig::new("room1", "alice");
        let mut sa = MeshSession::new(cfg.clone(), MeshRole::Initiator, a);
        let mut sb = MeshSession::new(cfg, MeshRole::Responder, b);
        sb.peer = Some(MeshPeer::new("alice"));

        let frame = SyncMessage::Chat {
            user: "alice".into(),
            text: "hi".into(),
            ts_ms: 0,
        };
        sa.send_frame(&frame).await.unwrap();
        let got = sb.recv_frame().await.unwrap().unwrap();
        match got {
            SyncMessage::Chat { user, text, .. } => {
                assert_eq!(user, "alice");
                assert_eq!(text, "hi");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn mesh_config_has_default_timeout() {
        let cfg = MeshConfig::new("r", "u");
        assert_eq!(cfg.handshake_timeout, Duration::from_secs(5));
        assert_eq!(cfg.room, "r");
    }
}
