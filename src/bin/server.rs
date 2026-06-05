//! `dipralix-server` — the realtime sync server binary.
//!
//! Lightweight WebSocket server that authenticates clients via JWT,
//! joins them to a room, and broadcasts `FileUpdate` frames to every
//! other member of the room. Persists the last known state per path
//! in memory (default) or SQLite (when `--persist` is given).
//!
//! The server has no compile-time dependency on the `dipralix-cli`
//! binary's modules — it implements the wire protocol directly. The
//! shared contract is the JSON schema in `src/sync/protocol.rs`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio::time::interval;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn, Level};
use tracing_subscriber::EnvFilter;

// ─── shared wire schema (mirrors src/sync/protocol.rs) ──────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    Text,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileUpdate {
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub kind: ContentKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_b64: Option<String>,
    pub author: String,
    pub ts_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Wire {
    Join {
        token: String,
        room: String,
        user: String,
    },
    JoinAck {
        ok: bool,
        #[serde(default)]
        snapshot: Vec<FileUpdate>,
        #[serde(default)]
        error: Option<String>,
    },
    FileUpdate(FileUpdate),
    Ack {
        path: String,
        seq: u64,
    },
    Error {
        message: String,
        fatal: bool,
    },
    Ping {
        ts_ms: u64,
    },
    Pong {
        ts_ms: u64,
    },
}

impl Wire {
    fn kind(&self) -> &'static str {
        match self {
            Wire::Join { .. } => "join",
            Wire::JoinAck { .. } => "join_ack",
            Wire::FileUpdate(_) => "file_update",
            Wire::Ack { .. } => "ack",
            Wire::Error { .. } => "error",
            Wire::Ping { .. } => "ping",
            Wire::Pong { .. } => "pong",
        }
    }
}

fn encode_frame(w: &Wire) -> Result<Vec<u8>> {
    serde_json::to_vec(w).context("encode wire frame")
}

fn decode_frame(b: &[u8]) -> Result<Wire> {
    serde_json::from_slice(b).context("decode wire frame")
}

// ─── path allowlist (mirrors src/sync/allowlist.rs) ──────────────────────────

const DIPRALIX_DIR: &str = ".dipralix";

fn is_allowed(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if path.starts_with('/') || path.contains("..") {
        return false;
    }
    if path == DIPRALIX_DIR || path.starts_with(&format!("{DIPRALIX_DIR}/")) {
        return true;
    }
    matches!(
        path,
        "dipralix-chat.log" | "dipralix-sync.log" | "dipralix-sync.db"
    )
}

// ─── store ──────────────────────────────────────────────────────────────────

pub struct MemStore(RwLock<HashMap<String, HashMap<String, FileUpdate>>>);

impl MemStore {
    fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }
}

/// Sum type of the two store backends, used to keep dispatch dyn
/// compatible via a single concrete enum while still allowing
/// per-backend implementations.
pub enum StoreKind {
    Mem(MemStore),
    Sqlite(SqliteStore),
}

impl StoreKind {
    #[allow(dead_code)]
    async fn get(&self, room: &str, path: &str) -> Result<Option<FileUpdate>> {
        match self {
            StoreKind::Mem(s) => s.get(room, path).await,
            StoreKind::Sqlite(s) => s.get(room, path).await,
        }
    }
    async fn snapshot(&self, room: &str) -> Result<Vec<FileUpdate>> {
        match self {
            StoreKind::Mem(s) => s.snapshot(room).await,
            StoreKind::Sqlite(s) => s.snapshot(room).await,
        }
    }
    async fn put(&self, room: &str, u: &FileUpdate) -> Result<()> {
        match self {
            StoreKind::Mem(s) => s.put(room, u).await,
            StoreKind::Sqlite(s) => s.put(room, u).await,
        }
    }
}

impl MemStore {
    #[allow(dead_code)]
    pub async fn get(&self, room: &str, path: &str) -> Result<Option<FileUpdate>> {
        Ok(self
            .0
            .read()
            .await
            .get(room)
            .and_then(|r| r.get(path))
            .cloned())
    }
    pub async fn snapshot(&self, room: &str) -> Result<Vec<FileUpdate>> {
        let g = self.0.read().await;
        let out = g
            .get(room)
            .map(|r| r.values().cloned().collect())
            .unwrap_or_default();
        Ok(out)
    }
    pub async fn put(&self, room: &str, u: &FileUpdate) -> Result<()> {
        self.0
            .write()
            .await
            .entry(room.to_string())
            .or_default()
            .insert(u.path.clone(), u.clone());
        Ok(())
    }
}

pub struct SqliteStore {
    conn: TokioMutex<Connection>,
}

impl SqliteStore {
    fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("open {}", path.display()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS file_state (
                room TEXT NOT NULL,
                path TEXT NOT NULL,
                payload TEXT NOT NULL,
                updated INTEGER NOT NULL,
                PRIMARY KEY (room, path)
             )",
        )?;
        Ok(Self {
            conn: TokioMutex::new(conn),
        })
    }
}

impl SqliteStore {
    #[allow(dead_code)]
    pub async fn get(&self, room: &str, path: &str) -> Result<Option<FileUpdate>> {
        let conn = self.conn.lock().await;
        let mut stmt =
            conn.prepare("SELECT payload FROM file_state WHERE room = ?1 AND path = ?2")?;
        let mut rows = stmt.query(params![room, path])?;
        if let Some(row) = rows.next()? {
            let payload: String = row.get(0)?;
            Ok(Some(serde_json::from_str(&payload)?))
        } else {
            Ok(None)
        }
    }
    pub async fn snapshot(&self, room: &str) -> Result<Vec<FileUpdate>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT payload FROM file_state WHERE room = ?1")?;
        let rows = stmt.query_map(params![room], |row| {
            let p: String = row.get(0)?;
            Ok(p)
        })?;
        let mut out = Vec::new();
        for r in rows {
            let payload = r?;
            out.push(serde_json::from_str(&payload)?);
        }
        Ok(out)
    }
    pub async fn put(&self, room: &str, u: &FileUpdate) -> Result<()> {
        let payload = serde_json::to_string(u)?;
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO file_state (room, path, payload, updated) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(room, path) DO UPDATE SET payload = excluded.payload, updated = excluded.updated",
            params![room, u.path, payload, u.ts_ms as i64],
        )?;
        Ok(())
    }
}

// ─── JWT ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    room: String,
    exp: usize,
}

fn make_token(secret: &str, sub: &str, room: &str, ttl: Duration) -> Result<String> {
    let exp = (SystemTime::now() + ttl)
        .duration_since(UNIX_EPOCH)?
        .as_secs() as usize;
    let claims = Claims {
        sub: sub.to_string(),
        room: room.to_string(),
        exp,
    };
    Ok(encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

fn verify_token(secret: &str, token: &str, expected_room: &str) -> Result<Claims> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_required_spec_claims(&["exp"]);
    v.leeway = 0;
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &v)
        .context("jwt decode")?;
    if data.claims.room != expected_room {
        anyhow::bail!(
            "token room mismatch: {} != {}",
            data.claims.room,
            expected_room
        );
    }
    Ok(data.claims)
}

// ─── room registry ──────────────────────────────────────────────────────────

type RoomTx = mpsc::UnboundedSender<Arc<Wire>>;

struct Registry {
    rooms: RwLock<HashMap<String, Vec<RoomTx>>>,
}

impl Registry {
    fn new() -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
        }
    }
    async fn add(&self, room: &str, tx: RoomTx) {
        self.rooms
            .write()
            .await
            .entry(room.to_string())
            .or_default()
            .push(tx);
    }
    async fn remove(&self, room: &str, tx: &RoomTx) {
        if let Some(list) = self.rooms.write().await.get_mut(room) {
            list.retain(|t| !t.same_channel(tx));
            if list.is_empty() {
                self.rooms.write().await.remove(room);
            }
        }
    }
    async fn broadcast(&self, room: &str, msg: Arc<Wire>, skip: Option<&RoomTx>) {
        let list = self.rooms.read().await.get(room).cloned();
        if let Some(list) = list {
            for t in list {
                if skip.map(|s| s.same_channel(&t)).unwrap_or(false) {
                    continue;
                }
                let _ = t.send(msg.clone());
            }
        }
    }
}

// ─── CLI ────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[clap(
    name = "dipralix-server",
    about = "DIPRALIX realtime sync server",
    version = "0.1.0"
)]
struct Args {
    /// Port to listen on.
    #[clap(long, default_value = "7878")]
    port: u16,

    /// Bind address.
    #[clap(long, default_value = "0.0.0.0")]
    bind: String,

    /// HMAC secret used to verify JWTs. If omitted, a random secret is
    /// generated and printed once at startup (dev mode).
    #[clap(long)]
    token_secret: Option<String>,

    /// Persist last-known state to this SQLite file. Default is in-memory.
    #[clap(long)]
    persist: Option<PathBuf>,

    /// Print a fresh demo JWT for `<sub>/<room>` and exit. Useful for
    /// `dipralix-cli realtime --token ...` smoke tests.
    #[clap(long)]
    issue_token: Option<String>,

    /// `sub` for `--issue-token` (the user identity to embed).
    #[clap(long)]
    sub: Option<String>,

    /// `room` for `--issue-token`.
    #[clap(long, default_value = "default")]
    room: String,

    /// TTL for the demo token, in seconds.
    #[clap(long, default_value_t = 86_400)]
    token_ttl: u64,

    /// Print the bound port on stdout as `PORT=<n>` after listening.
    /// Used by integration tests that bind to port 0 and need to
    /// discover the kernel-assigned port.
    #[clap(long)]
    print_port: bool,
}

// ─── main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();

    // Demo-token helper path: print a JWT and exit.
    if let Some(secret) = &args.token_secret {
        if args.issue_token.is_some() {
            let sub = args.sub.clone().unwrap_or_else(|| "demo".to_string());
            let room = args.room.clone();
            let token = make_token(secret, &sub, &room, Duration::from_secs(args.token_ttl))?;
            println!("{token}");
            return Ok(());
        }
    }

    let secret = match &args.token_secret {
        Some(s) => s.clone(),
        None => {
            let s = random_secret();
            warn!("no --token-secret given; generated ephemeral secret: {s}");
            s
        }
    };

    let store: Arc<StoreKind> = match &args.persist {
        Some(p) => Arc::new(StoreKind::Sqlite(SqliteStore::open(p)?)),
        None => Arc::new(StoreKind::Mem(MemStore::new())),
    };
    let registry = Arc::new(Registry::new());

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    if args.print_port {
        // Use a machine-greppable prefix; tests look for `PORT=<n>`.
        println!("PORT={}", bound.port());
    }
    info!(%addr, persist = ?args.persist, "dipralix-server listening");

    loop {
        let (stream, peer) = listener.accept().await?;
        let secret = secret.clone();
        let store = store.clone();
        let registry = registry.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, peer, &secret, store, registry).await {
                warn!(%peer, error = %e, "connection error");
            }
        });
    }
}

async fn handle_conn(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    secret: &str,
    store: Arc<StoreKind>,
    registry: Arc<Registry>,
) -> Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut sink, mut stream) = ws.split();

    // Wait for Join.
    let join = loop {
        let msg = match stream.next().await {
            Some(Ok(Message::Text(t))) => t,
            Some(Ok(Message::Binary(b))) => String::from_utf8_lossy(&b).into_owned(),
            Some(Ok(Message::Close(_))) | None => anyhow::bail!("client closed before join"),
            Some(Ok(other)) => {
                debug!(?other, "ignoring frame before join");
                continue;
            }
            Some(Err(e)) => anyhow::bail!("recv: {e}"),
        };
        let parsed = decode_frame(msg.as_bytes())?;
        if let Wire::Join { token, room, user } = parsed {
            break JoinRequest { token, room, user };
        }
        debug!(kind = parsed.kind(), "expected join, got something else");
    };

    let JoinRequest { token, room, user } = join;
    let claims = match verify_token(secret, &token, &room) {
        Ok(c) => c,
        Err(e) => {
            let frame = encode_frame(&Wire::JoinAck {
                ok: false,
                snapshot: vec![],
                error: Some(format!("auth: {e:#}")),
            })?;
            let _ = sink
                .send(Message::Text(String::from_utf8_lossy(&frame).into_owned()))
                .await;
            let _ = sink.send(Message::Close(None)).await;
            anyhow::bail!("auth failed: {e:#}");
        }
    };
    info!(%peer, room = %room, sub = %claims.sub, user = %user, "client joined");

    let snap = store.snapshot(&room).await?;
    let ack = encode_frame(&Wire::JoinAck {
        ok: true,
        snapshot: snap,
        error: None,
    })?;
    sink.send(Message::Text(String::from_utf8_lossy(&ack).into_owned()))
        .await?;

    let (tx, mut rx) = mpsc::unbounded_channel::<Arc<Wire>>();
    registry.add(&room, tx.clone()).await;

    let mut ping_ticker = interval(Duration::from_secs(15));

    loop {
        tokio::select! {
            biased;
            incoming = stream.next() => {
                let Some(msg_res) = incoming else {
                    info!(%peer, "client disconnected");
                    break;
                };
                let msg = match msg_res? {
                    Message::Text(t) => t,
                    Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
                    Message::Ping(_) | Message::Pong(_) => continue,
                    Message::Close(_) => break,
                    other => {
                        debug!(?other, "ignoring non-text frame");
                        continue;
                    }
                };
                let parsed = decode_frame(msg.as_bytes())?;
                match parsed {
                    Wire::FileUpdate(u) => {
                        if !is_allowed(&u.path) {
                            let err = encode_frame(&Wire::Error {
                                message: format!("path not allowed: {}", u.path),
                                fatal: true,
                            })?;
                            let _ = sink.send(Message::Text(String::from_utf8_lossy(&err).into_owned())).await;
                            break;
                        }
                        store.put(&room, &u).await?;
                        registry.broadcast(&room, Arc::new(Wire::FileUpdate(u)), Some(&tx)).await;
                    }
                    Wire::Pong { .. } => {}
                    other => {
                        debug!(kind = other.kind(), "ignoring unexpected frame after join");
                    }
                }
            }
            outbound = rx.recv() => {
                let Some(w) = outbound else { break };
                let frame = encode_frame(&w)?;
                if sink.send(Message::Text(String::from_utf8_lossy(&frame).into_owned())).await.is_err() {
                    break;
                }
            }
            _ = ping_ticker.tick() => {
                let ts = now_ms();
                let frame = encode_frame(&Wire::Ping { ts_ms: ts })?;
                if sink.send(Message::Text(String::from_utf8_lossy(&frame).into_owned())).await.is_err() {
                    break;
                }
            }
        }
    }

    registry.remove(&room, &tx).await;
    Ok(())
}

struct JoinRequest {
    token: String,
    room: String,
    user: String,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn random_secret() -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(32);
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut x: u64 = seed as u64 ^ 0x9E3779B97F4A7C15;
    for _ in 0..4 {
        // splitmix64
        x = x.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^= z >> 31;
        for b in z.to_be_bytes() {
            let _ = write!(s, "{b:02x}");
        }
    }
    s
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,dipralix_server=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_max_level(Level::TRACE)
        .with_target(false)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_join() {
        let w = Wire::Join {
            token: "x".into(),
            room: "r".into(),
            user: "u".into(),
        };
        let bytes = encode_frame(&w).unwrap();
        let back = decode_frame(&bytes).unwrap();
        assert_eq!(w, back);
    }

    #[test]
    fn round_trip_file_update() {
        let u = FileUpdate {
            path: ".dipralix/memory/a.md".into(),
            hash: "deadbeef".into(),
            size: 4,
            kind: ContentKind::Text,
            content: Some("hi".into()),
            content_b64: None,
            author: "alice".into(),
            ts_ms: 123,
        };
        let w = Wire::FileUpdate(u);
        let bytes = encode_frame(&w).unwrap();
        let back = decode_frame(&bytes).unwrap();
        assert_eq!(w, back);
    }

    #[test]
    fn allowlist_accepts_dipralix_paths() {
        assert!(is_allowed(".dipralix/memory/x.md"));
        assert!(is_allowed(".dipralix/plans/y.md"));
        assert!(is_allowed(".dipralix/skills/z.md"));
    }

    #[test]
    fn allowlist_rejects_source_and_secrets() {
        for p in [
            "src/main.rs",
            ".env",
            "../etc/passwd",
            "/etc/passwd",
            "config.local",
        ] {
            assert!(!is_allowed(p), "{p} should be rejected");
        }
    }

    #[test]
    fn issue_and_verify_token() {
        let secret = "testsecret";
        let token = make_token(secret, "alice", "myproject", Duration::from_secs(60)).unwrap();
        let claims = verify_token(secret, &token, "myproject").unwrap();
        assert_eq!(claims.sub, "alice");
        assert_eq!(claims.room, "myproject");
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let token = make_token("a", "alice", "r", Duration::from_secs(60)).unwrap();
        assert!(verify_token("b", &token, "r").is_err());
    }

    #[test]
    fn verify_rejects_wrong_room() {
        let secret = "s";
        let token = make_token(secret, "alice", "r1", Duration::from_secs(60)).unwrap();
        assert!(verify_token(secret, &token, "r2").is_err());
    }

    #[test]
    fn verify_rejects_expired() {
        let secret = "s";
        // Make the token expire 2s in the past. With leeway=0 in
        // verify_token, the next call must reject it.
        let token = make_token(secret, "alice", "r", Duration::from_secs(0)).unwrap();
        std::thread::sleep(Duration::from_millis(1500));
        assert!(verify_token(secret, &token, "r").is_err());
    }

    #[tokio::test]
    async fn mem_store_round_trip() {
        let s = MemStore::new();
        let u = FileUpdate {
            path: ".dipralix/memory/a.md".into(),
            hash: "h".into(),
            size: 1,
            kind: ContentKind::Text,
            content: Some("x".into()),
            content_b64: None,
            author: "a".into(),
            ts_ms: 1,
        };
        s.put("r", &u).await.unwrap();
        let got = s.get("r", ".dipralix/memory/a.md").await.unwrap();
        assert_eq!(got, Some(u));
    }

    #[tokio::test]
    async fn sqlite_store_round_trip() {
        let dir = std::env::temp_dir().join(format!("dipralix-srv-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("srv.db");
        let s = SqliteStore::open(&path).unwrap();
        let u = FileUpdate {
            path: ".dipralix/memory/a.md".into(),
            hash: "h".into(),
            size: 1,
            kind: ContentKind::Text,
            content: Some("x".into()),
            content_b64: None,
            author: "a".into(),
            ts_ms: 1,
        };
        s.put("r", &u).await.unwrap();
        let got = s.get("r", ".dipralix/memory/a.md").await.unwrap();
        assert_eq!(got, Some(u));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
