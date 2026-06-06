//! Phase 3: serverless P2P mesh.
//!
//! This module provides a real, no-server sync path for the
//! "two or three devs on the same network" case (§7):
//!
//! - [`SyncTransport`] — async send/recv trait shared by every transport.
//! - [`WsTransport`] — wraps a `tokio_tungstenite` stream (server path).
//! - [`MeshTransport`] — in-process `mpsc` loopback, used by unit tests.
//! - [`TcpTransport`] — a **real** TCP link whose every frame is sealed with
//!   a Noise (`NNpsk0`) session (see [`super::crypto`]); length-prefixed
//!   framing, split read/write halves so a peer can be read and written
//!   concurrently.
//! - [`MeshNode`] — the top-level node: it binds a TCP listener, advertises
//!   itself over mDNS ([`super::discovery`]), dials discovered (and manually
//!   seeded) peers, runs the Noise handshake on each link, watches
//!   `.dipralix/`, and gossips [`FileUpdate`]s across the mesh.
//!
//! There is **no central server, STUN, or TURN**: peers must already share
//! the LAN (mDNS is link-local) and the room secret (the Noise PSK). That is
//! the entire trust boundary.

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, info, warn};

use super::crypto::{Handshake, Psk, Transport as NoiseTransport};
use super::discovery::Discovery;
use super::error::{Result, SyncError};
use super::fileio::FileSync;
use super::protocol::SyncMessage;
use super::watcher;

/// Largest single frame on a mesh link (8 MiB). Bounds memory per read and
/// makes a hostile/garbled length prefix fail fast instead of allocating wild.
const MAX_FRAME: usize = 8 * 1024 * 1024;

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

/// Role in a P2P link. The **initiator** dials and speaks first in the
/// Noise handshake (`-> e`); the **responder** accepts and replies
/// (`<- e, ee`). The role is decided by who connects to whom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshRole {
    /// The peer that dialed out (Noise initiator).
    Initiator,
    /// The peer that accepted the connection (Noise responder).
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

// ─── Length-prefixed framing ────────────────────────────────────────────

/// Write one length-prefixed frame: a 4-byte big-endian length, then the
/// payload. Flushed so the peer can read it immediately.
async fn write_frame<W: AsyncWriteExt + Unpin>(w: &mut W, payload: &[u8]) -> Result<()> {
    let len = u32::try_from(payload.len())
        .map_err(|_| SyncError::Transport(format!("frame too large: {} bytes", payload.len())))?;
    w.write_all(&len.to_be_bytes())
        .await
        .map_err(SyncError::Io)?;
    w.write_all(payload).await.map_err(SyncError::Io)?;
    w.flush().await.map_err(SyncError::Io)?;
    Ok(())
}

/// Read one length-prefixed frame. Returns `Ok(None)` on a clean EOF (the
/// peer closed the connection between frames).
async fn read_frame<R: AsyncReadExt + Unpin>(r: &mut R) -> Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match r.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(SyncError::Io(e)),
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME {
        return Err(SyncError::Transport(format!(
            "frame length {len} exceeds {MAX_FRAME}"
        )));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await.map_err(SyncError::Io)?;
    Ok(Some(buf))
}

/// Drive the Noise handshake over a freshly-connected TCP stream, returning
/// the live transport cipher state on success.
async fn tcp_handshake(
    stream: &mut TcpStream,
    initiator: bool,
    psk: &Psk,
) -> Result<NoiseTransport> {
    if initiator {
        let mut hs = Handshake::initiator(psk)?;
        let m1 = hs.write_message()?; // -> e
        write_frame(stream, &m1).await?;
        let m2 = read_frame(stream)
            .await?
            .ok_or_else(|| SyncError::Crypto("handshake: peer closed".into()))?;
        hs.read_message(&m2)?; // <- e, ee
        hs.into_transport()
    } else {
        let mut hs = Handshake::responder(psk)?;
        let m1 = read_frame(stream)
            .await?
            .ok_or_else(|| SyncError::Crypto("handshake: peer closed".into()))?;
        hs.read_message(&m1)?; // -> e
        let m2 = hs.write_message()?; // <- e, ee
        write_frame(stream, &m2).await?;
        hs.into_transport()
    }
}

// ─── Encrypted TCP transport ────────────────────────────────────────────

/// A real TCP peer link with Noise end-to-end encryption.
///
/// Every [`SyncMessage`] is JSON-encoded, sealed with the session's Noise
/// transport, and written as a length-prefixed frame. The read and write
/// halves share one [`NoiseTransport`] behind a mutex (its send and receive
/// cipher states are independent, so this serializes only the brief AEAD op).
pub struct TcpTransport {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    noise: Arc<Mutex<NoiseTransport>>,
}

impl TcpTransport {
    /// Dial `addr` and complete the Noise handshake as the **initiator**.
    ///
    /// # Errors
    /// Returns [`SyncError::Io`] on connect failure or [`SyncError::Crypto`]
    /// if the handshake fails (the dominant cause being a wrong room secret).
    pub async fn connect(addr: SocketAddr, psk: &Psk) -> Result<Self> {
        let mut stream = TcpStream::connect(addr).await.map_err(SyncError::Io)?;
        let _ = stream.set_nodelay(true);
        let noise = tcp_handshake(&mut stream, true, psk).await?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader,
            writer,
            noise: Arc::new(Mutex::new(noise)),
        })
    }

    /// Complete the Noise handshake as the **responder** on an accepted stream.
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] if the peer cannot authenticate.
    pub async fn accept(mut stream: TcpStream, psk: &Psk) -> Result<Self> {
        let _ = stream.set_nodelay(true);
        let noise = tcp_handshake(&mut stream, false, psk).await?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader,
            writer,
            noise: Arc::new(Mutex::new(noise)),
        })
    }

    /// Split into independent reader/writer halves (for the node's per-peer
    /// read and write tasks). Both share the same Noise cipher state.
    #[must_use]
    pub fn into_split(self) -> (TcpReader, TcpWriter) {
        (
            TcpReader {
                reader: self.reader,
                noise: self.noise.clone(),
            },
            TcpWriter {
                writer: self.writer,
                noise: self.noise,
            },
        )
    }
}

#[async_trait]
impl SyncTransport for TcpTransport {
    async fn send(&mut self, msg: &SyncMessage) -> Result<()> {
        let plaintext = msg.encode()?;
        let ciphertext = self.noise.lock().await.encrypt(&plaintext)?;
        write_frame(&mut self.writer, &ciphertext).await
    }

    async fn recv(&mut self) -> Result<Option<SyncMessage>> {
        let Some(ciphertext) = read_frame(&mut self.reader).await? else {
            return Ok(None);
        };
        let plaintext = self.noise.lock().await.decrypt(&ciphertext)?;
        Ok(Some(SyncMessage::decode(&plaintext)?))
    }

    async fn close(&mut self) -> Result<()> {
        self.writer.shutdown().await.map_err(SyncError::Io)
    }
}

/// Read half of a [`TcpTransport`]. Decrypts inbound frames.
pub struct TcpReader {
    reader: OwnedReadHalf,
    noise: Arc<Mutex<NoiseTransport>>,
}

impl TcpReader {
    /// Receive and decrypt the next frame, or `Ok(None)` on clean EOF.
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] on an AEAD failure or
    /// [`SyncError::Protocol`] on a malformed decrypted frame.
    pub async fn recv(&mut self) -> Result<Option<SyncMessage>> {
        let Some(ciphertext) = read_frame(&mut self.reader).await? else {
            return Ok(None);
        };
        let plaintext = self.noise.lock().await.decrypt(&ciphertext)?;
        Ok(Some(SyncMessage::decode(&plaintext)?))
    }
}

/// Write half of a [`TcpTransport`]. Encrypts outbound frames.
pub struct TcpWriter {
    writer: OwnedWriteHalf,
    noise: Arc<Mutex<NoiseTransport>>,
}

impl TcpWriter {
    /// Encrypt and send one frame.
    ///
    /// # Errors
    /// Returns [`SyncError::Crypto`] on encryption failure or
    /// [`SyncError::Io`] on a write failure (peer gone).
    pub async fn send(&mut self, msg: &SyncMessage) -> Result<()> {
        let plaintext = msg.encode()?;
        let ciphertext = self.noise.lock().await.encrypt(&plaintext)?;
        write_frame(&mut self.writer, &ciphertext).await
    }
}

// ─── Mesh node ──────────────────────────────────────────────────────────

/// How often the node re-browses mDNS and retries seed peers.
const CONNECT_INTERVAL: Duration = Duration::from_secs(3);

/// A serverless mesh node: listener + mDNS + peer links + file gossip.
///
/// Construct with [`MeshNode::new`], optionally add manual peers with
/// [`MeshNode::with_seed_peers`] (useful when mDNS is firewalled), then drive
/// it with [`MeshNode::run`].
pub struct MeshNode {
    room: String,
    user: String,
    psk: Psk,
    project_root: PathBuf,
    bind_port: u16,
    seed_peers: Vec<SocketAddr>,
}

impl MeshNode {
    /// Build a node. `secret` is the shared room secret; it is stretched into
    /// the Noise PSK via BLAKE3, so any peer that knows the same secret (and
    /// is on the same LAN) can join. `bind_port` of 0 picks an ephemeral port.
    #[must_use]
    pub fn new(
        room: impl Into<String>,
        user: impl Into<String>,
        secret: &str,
        project_root: PathBuf,
        bind_port: u16,
    ) -> Self {
        Self {
            room: room.into(),
            user: user.into(),
            psk: Psk::derive(secret.as_bytes()),
            project_root,
            bind_port,
            seed_peers: Vec::new(),
        }
    }

    /// Add manually-specified peer addresses to dial (in addition to anything
    /// found via mDNS). Lets the mesh work across subnets or when multicast
    /// discovery is blocked.
    #[must_use]
    pub fn with_seed_peers(mut self, peers: Vec<SocketAddr>) -> Self {
        self.seed_peers = peers;
        self
    }

    /// Run the node until Ctrl-C. Binds, advertises, connects, and gossips.
    ///
    /// # Errors
    /// Returns an error only on a fatal setup failure (cannot bind the
    /// listener or start the watcher). Transient peer/discovery failures are
    /// logged and retried.
    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, self.bind_port))
            .await
            .map_err(SyncError::Io)?;
        let local_port = listener.local_addr().map_err(SyncError::Io)?.port();
        info!(room = %self.room, user = %self.user, port = local_port, "mesh node up");

        // mDNS is best-effort: if there is no multicast-capable interface we
        // still run with manually-seeded peers.
        let discovery = match Discovery::advertise(&self.room, local_port, &self.user) {
            Ok(d) => Some(Arc::new(d)),
            Err(e) => {
                warn!(error = %e, "mDNS unavailable; relying on seed peers");
                None
            }
        };

        let peers: Arc<Mutex<Vec<mpsc::Sender<SyncMessage>>>> = Arc::new(Mutex::new(Vec::new()));
        let connected: Arc<Mutex<HashSet<SocketAddr>>> = Arc::new(Mutex::new(HashSet::new()));
        let (inbound_tx, mut inbound_rx) = mpsc::channel::<SyncMessage>(256);

        // Accept loop: inbound dials become responder-side peers.
        {
            let psk = self.psk;
            let peers = peers.clone();
            let inbound_tx = inbound_tx.clone();
            let root = self.project_root.clone();
            let user = self.user.clone();
            tokio::spawn(async move {
                loop {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            debug!(%addr, "incoming peer");
                            match TcpTransport::accept(stream, &psk).await {
                                Ok(t) => {
                                    register_peer(
                                        t,
                                        &peers,
                                        inbound_tx.clone(),
                                        root.clone(),
                                        user.clone(),
                                    )
                                    .await;
                                }
                                Err(e) => warn!(%addr, error = %e, "peer handshake failed"),
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "accept failed");
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                    }
                }
            });
        }

        // Connector loop: dial mDNS-discovered + seeded peers we have not yet
        // connected to, on an interval.
        {
            let psk = self.psk;
            let peers = peers.clone();
            let connected = connected.clone();
            let inbound_tx = inbound_tx.clone();
            let discovery = discovery.clone();
            let seed = self.seed_peers.clone();
            let root = self.project_root.clone();
            let user = self.user.clone();
            tokio::spawn(async move {
                loop {
                    let mut candidates: Vec<SocketAddr> = seed.clone();
                    if let Some(d) = &discovery {
                        match d.browse(Duration::from_millis(1200)).await {
                            Ok(found) => candidates.extend(found),
                            Err(e) => debug!(error = %e, "browse failed"),
                        }
                    }
                    for addr in candidates {
                        if connected.lock().await.contains(&addr) {
                            continue;
                        }
                        match TcpTransport::connect(addr, &psk).await {
                            Ok(t) => {
                                connected.lock().await.insert(addr);
                                info!(%addr, "dialed peer");
                                register_peer(
                                    t,
                                    &peers,
                                    inbound_tx.clone(),
                                    root.clone(),
                                    user.clone(),
                                )
                                .await;
                            }
                            Err(e) => debug!(%addr, error = %e, "dial failed (will retry)"),
                        }
                    }
                    tokio::time::sleep(CONNECT_INTERVAL).await;
                }
            });
        }

        // File gossip: local changes out, remote changes applied + relayed.
        let mut changes = watcher::start_watching(self.project_root.clone()).await?;
        let mut fs = FileSync::new(self.project_root.clone());

        loop {
            tokio::select! {
                ev = changes.recv() => {
                    let Some(ev) = ev else { break };
                    match fs.local_update(&ev.rel_path, &self.user) {
                        Ok(Some(upd)) => {
                            broadcast(&peers, &SyncMessage::FileUpdate(upd)).await;
                            info!(path = %ev.rel_path, "broadcast local change");
                        }
                        Ok(None) => {}
                        Err(e) => warn!(path = %ev.rel_path, error = %e, "local read failed"),
                    }
                }
                inbound = inbound_rx.recv() => {
                    let Some(msg) = inbound else { break };
                    self.handle_inbound(msg, &mut fs, &peers).await;
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("shutting down mesh node");
                    break;
                }
            }
        }

        if let Some(d) = discovery {
            if let Ok(d) = Arc::try_unwrap(d) {
                d.shutdown();
            }
        }
        Ok(())
    }

    /// Apply an inbound frame. `FileUpdate`s that actually change disk are
    /// re-gossiped to the other peers; everything else is logged.
    async fn handle_inbound(
        &self,
        msg: SyncMessage,
        fs: &mut FileSync,
        peers: &Arc<Mutex<Vec<mpsc::Sender<SyncMessage>>>>,
    ) {
        match msg {
            SyncMessage::FileUpdate(upd) => match fs.apply(&upd) {
                Ok(true) => {
                    info!(path = %upd.path, author = %upd.author, "applied peer change");
                    broadcast(peers, &SyncMessage::FileUpdate(upd)).await;
                }
                Ok(false) => {}
                Err(e) => warn!(path = %upd.path, error = %e, "apply failed"),
            },
            SyncMessage::Chat { user, text, .. } => info!(%user, "chat: {text}"),
            SyncMessage::Presence { user, status, .. } => debug!(%user, ?status, "presence"),
            other => debug!(kind = other.kind(), "ignoring frame"),
        }
    }
}

/// Spawn the read and write tasks for one connected peer and register its
/// outbound channel so [`broadcast`] reaches it. The new peer is first sent a
/// snapshot of our current `.dipralix/` state so it converges immediately.
async fn register_peer(
    transport: TcpTransport,
    peers: &Arc<Mutex<Vec<mpsc::Sender<SyncMessage>>>>,
    inbound_tx: mpsc::Sender<SyncMessage>,
    project_root: PathBuf,
    user: String,
) {
    let (mut reader, mut writer) = transport.into_split();
    let (out_tx, mut out_rx) = mpsc::channel::<SyncMessage>(512);

    // Seed the peer with our current state before live changes flow.
    for upd in crate::sync::fileio::scan_snapshot(&project_root, &user) {
        if out_tx.send(SyncMessage::FileUpdate(upd)).await.is_err() {
            break;
        }
    }
    peers.lock().await.push(out_tx);

    // Writer task: drain the outbound queue onto the wire.
    tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if let Err(e) = writer.send(&msg).await {
                debug!(error = %e, "peer write ended");
                break;
            }
        }
    });

    // Reader task: forward decrypted frames to the node's inbound channel.
    tokio::spawn(async move {
        loop {
            match reader.recv().await {
                Ok(Some(msg)) => {
                    if inbound_tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    debug!(error = %e, "peer read ended");
                    break;
                }
            }
        }
    });
}

/// Send a frame to every currently-connected peer (best-effort). Dead peers'
/// channels error and are pruned.
async fn broadcast(peers: &Arc<Mutex<Vec<mpsc::Sender<SyncMessage>>>>, msg: &SyncMessage) {
    let mut guard = peers.lock().await;
    let mut alive = Vec::with_capacity(guard.len());
    for tx in guard.drain(..) {
        if tx.send(msg.clone()).await.is_ok() {
            alive.push(tx);
        }
    }
    *guard = alive;
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

    #[test]
    fn frame_roundtrips_through_length_prefix() {
        // Drive write_frame/read_frame over an in-memory duplex.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut a, mut b) = tokio::io::duplex(1024);
            let payload = b"hello frame";
            write_frame(&mut a, payload).await.unwrap();
            let got = read_frame(&mut b).await.unwrap().unwrap();
            assert_eq!(got, payload);
        });
    }

    #[tokio::test]
    async fn tcp_transport_handshakes_and_exchanges_encrypted_frame() {
        let psk = Psk::derive(b"room-secret");
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_psk = psk;
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut t = TcpTransport::accept(stream, &server_psk).await.unwrap();
            // Echo one frame back.
            let got = t.recv().await.unwrap().unwrap();
            t.send(&got).await.unwrap();
        });

        let mut client = TcpTransport::connect(addr, &psk).await.unwrap();
        let frame = SyncMessage::FileUpdate(FileUpdate::from_text(
            ".dipralix/memory/x.md",
            b"encrypted body",
            "alice",
        ));
        client.send(&frame).await.unwrap();
        let echoed = client.recv().await.unwrap().unwrap();
        assert_eq!(echoed, frame);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn tcp_handshake_fails_on_wrong_secret() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            // Responder uses a different room secret -> handshake must reject.
            let wrong = Psk::derive(b"other-room");
            TcpTransport::accept(stream, &wrong).await
        });

        let right = Psk::derive(b"room-secret");
        let client = TcpTransport::connect(addr, &right).await;
        let server_res = server.await.unwrap();
        assert!(
            client.is_err() || server_res.is_err(),
            "mismatched PSK must fail the handshake on at least one side"
        );
    }

    /// Two real mesh nodes on localhost (seeded at each other, mDNS optional)
    /// converge a `.dipralix/` file change end-to-end: watcher → encrypted TCP
    /// → apply on the peer. This is the §10 Phase 3 Definition of Done.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn two_nodes_sync_a_file_over_real_tcp() {
        use std::time::Duration;

        fn mkroot() -> PathBuf {
            let p = std::env::temp_dir().join(format!("dipralix-mesh-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(p.join(".dipralix/memory")).unwrap();
            p
        }

        let root_a = mkroot();
        let root_b = mkroot();

        // Pre-bind to learn ports, then hand the listeners' ports as seeds.
        // We bind ephemeral ports inside the nodes, so instead seed via fixed
        // localhost ports chosen by binding throwaway listeners first.
        let l_a = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port_a = l_a.local_addr().unwrap().port();
        let l_b = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port_b = l_b.local_addr().unwrap().port();
        drop(l_a);
        drop(l_b);

        let peer_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();
        let peer_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();

        let secret = "team-room-secret";
        let node_a = MeshNode::new("proj", "alice", secret, root_a.clone(), port_a)
            .with_seed_peers(vec![peer_b]);
        let node_b = MeshNode::new("proj", "bob", secret, root_b.clone(), port_b)
            .with_seed_peers(vec![peer_a]);

        let h_a = tokio::spawn(node_a.run());
        let h_b = tokio::spawn(node_b.run());

        // Give the nodes a moment to bind, dial, and complete handshakes.
        tokio::time::sleep(Duration::from_millis(800)).await;

        // Alice writes a memory file; Bob should receive it.
        let rel = ".dipralix/memory/decision.md";
        std::fs::write(root_a.join(rel), b"use blake3 for hashing").unwrap();

        let target = root_b.join(rel);
        let mut synced = false;
        // CI can be noisy; allow enough time for a couple of reconnect cycles.
        for _ in 0..180 {
            if let Ok(body) = std::fs::read(&target) {
                if body == b"use blake3 for hashing" {
                    synced = true;
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        h_a.abort();
        h_b.abort();
        let _ = std::fs::remove_dir_all(&root_a);
        let _ = std::fs::remove_dir_all(&root_b);

        assert!(synced, "Bob did not receive Alice's file over the mesh");
    }
}
