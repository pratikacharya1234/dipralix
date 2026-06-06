//! Phase 2 acceptance test for the team-policy approval quorum.
//!
//! Exercises the full WebSocket pipeline against a live
//! `dipralix-server` process: a client opens an approval request,
//! two other clients vote `Approve`, the server must broadcast a
//! single `ApprovalDecision { approved: true }`. A second scenario
//! shows that one `Deny` immediately resolves as `Denied` and that
//! subsequent votes for the same request are dropped.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;

use dipralix::sync::protocol::SyncMessage;

fn server_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("dipralix-server");
    p
}

fn free_port() -> u16 {
    use std::net::TcpListener;
    // Bind via std so we don't have to spin up a runtime just to
    // discover a port.
    let l = TcpListener::bind("127.0.0.1:0").expect("bind 0");
    let port = l.local_addr().expect("local_addr").port();
    drop(l);
    port
}

async fn spawn_server(port: u16) -> std::process::Child {
    let log_path = std::env::temp_dir().join(format!("dipralix-approval-{port}.log"));
    let log = std::fs::File::create(&log_path).expect("create log");

    let mut cmd = std::process::Command::new(server_bin());
    cmd.arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--token-secret")
        .arg("approval-quorum-test-secret")
        .arg("--print-port")
        .stdout(Stdio::from(log.try_clone().expect("clone log")))
        .stderr(Stdio::from(log))
        .stdin(Stdio::null());
    let mut child = cmd.spawn().expect("spawn server");

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port)))
            .await
            .is_ok()
        {
            return child;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Best-effort cleanup before panicking; we don't want a leaked
    // server process to interfere with the next test.
    let _ = child.kill();
    let _ = child.wait();
    panic!("server never bound to {port}");
}

async fn issue_token(secret: &str, sub: &str, room: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let claims = json!({
        "sub": sub,
        "room": room,
        "iat": now,
        "exp": now + 600,
    });
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode jwt")
}

#[derive(Default, Clone)]
struct Captured {
    frames: Arc<Mutex<Vec<Value>>>,
}

impl Captured {
    async fn snapshot(&self) -> Vec<Value> {
        self.frames.lock().await.clone()
    }
}

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type Sink = futures::stream::SplitSink<Ws, tokio_tungstenite::tungstenite::Message>;

async fn connect_client(port: u16, token: String, room: &str, user: &str) -> (Sink, Captured) {
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    let url = format!("ws://127.0.0.1:{port}");
    let (ws, _) = connect_async(&url).await.expect("ws connect");
    let (mut sink, stream) = ws.split();
    let mut stream = stream;

    let join = SyncMessage::Join {
        token,
        room: room.to_string(),
        user: user.to_string(),
    };
    let join_bytes = serde_json::to_vec(&join).expect("encode join");
    sink.send(Message::Text(String::from_utf8(join_bytes).expect("utf8")))
        .await
        .expect("send join");

    let captured = Captured::default();
    let ack = timeout(Duration::from_secs(3), stream.next())
        .await
        .expect("ack timeout")
        .expect("ack none")
        .expect("ack err");
    let ack_text = match ack {
        Message::Text(t) => t,
        other => panic!("expected text, got {other:?}"),
    };
    let ack_val: Value = serde_json::from_str(&ack_text).expect("ack json");
    assert_eq!(ack_val["type"], "join_ack");
    assert_eq!(ack_val["ok"], Value::Bool(true));
    captured.frames.lock().await.push(ack_val);

    let cap2 = captured.clone();
    tokio::spawn(async move {
        use tokio_tungstenite::tungstenite::Message;
        while let Some(Ok(msg)) = stream.next().await {
            let text = match msg {
                Message::Text(t) => t,
                Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
                Message::Ping(_) | Message::Pong(_) | Message::Close(_) => continue,
                _ => continue,
            };
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                cap2.frames.lock().await.push(v);
            }
        }
    });

    (sink, captured)
}

async fn send(sink: &mut Sink, msg: SyncMessage) {
    use tokio_tungstenite::tungstenite::Message;
    let bytes = serde_json::to_vec(&msg).expect("encode");
    sink.send(Message::Text(String::from_utf8(bytes).expect("utf8")))
        .await
        .expect("send");
    // tokio_tungstenite's sink is buffered; the message is queued
    // for write but not yet on the wire. Force a flush by closing
    // the send side is too aggressive — instead, send a tiny
    // ping/pong round-trip. A simpler approach: rely on the
    // `incoming` side receiving the broadcast as the synchronizer
    // (see `wait_for`).
}

/// Wait for `cap` to contain a frame matching `kind` (e.g.
/// `"approval_request"`). Polls every 25ms up to 3 seconds.
async fn wait_for(cap: &Captured, kind: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if cap.frames.lock().await.iter().any(|v| v["type"] == kind) {
            return;
        }
        if std::time::Instant::now() > deadline {
            panic!("timed out waiting for {kind}");
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_of_three_approves_resolves_approved() {
    let port = free_port();
    let mut child = spawn_server(port).await;

    let secret = "approval-quorum-test-secret";
    let room = "approval-room-2of3";

    let t_req = issue_token(secret, "alice", room).await;
    let t_b1 = issue_token(secret, "bob", room).await;
    let t_b2 = issue_token(secret, "carol", room).await;

    // Three clients connected in order: alice, bob, carol. All
    // must stay connected for the duration of the test so the
    // background frame forwarder keeps pushing to `captured`.
    let (mut sink_alice, cap_alice) = connect_client(port, t_req, room, "alice").await;
    let (mut sink_bob, cap_bob) = connect_client(port, t_b1, room, "bob").await;
    let (mut sink_carol, cap_carol) = connect_client(port, t_b2, room, "carol").await;

    let req_id = "req-2of3-1".to_string();
    send(
        &mut sink_alice,
        SyncMessage::ApprovalRequest {
            request_id: req_id.clone(),
            action: "deploy-staging".into(),
            payload: "{}".into(),
            requester: "alice".into(),
            required_approvers: 2,
            ts_ms: dipralix::sync::now_ms(),
        },
    )
    .await;

    // Wait for both voters to receive the request before they
    // cast their votes. This eliminates the wire-order race that
    // would otherwise let a vote arrive before the server has
    // inserted the request into its tally.
    wait_for(&cap_bob, "approval_request").await;
    wait_for(&cap_carol, "approval_request").await;

    send(
        &mut sink_bob,
        SyncMessage::ApprovalVote {
            request_id: req_id.clone(),
            voter: "bob".into(),
            vote: dipralix::sync::protocol::ApprovalVoteKind::Approve,
            reason: None,
            ts_ms: dipralix::sync::now_ms(),
        },
    )
    .await;

    send(
        &mut sink_carol,
        SyncMessage::ApprovalVote {
            request_id: req_id.clone(),
            voter: "carol".into(),
            vote: dipralix::sync::protocol::ApprovalVoteKind::Approve,
            reason: None,
            ts_ms: dipralix::sync::now_ms(),
        },
    )
    .await;

    // Wait for the decision to make its way to all three clients.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let alice_frames = cap_alice.snapshot().await;
        if alice_frames
            .iter()
            .any(|v| v["type"] == "approval_decision")
        {
            break;
        }
        if std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;

    let alice_frames = cap_alice.snapshot().await;
    let decisions: Vec<&Value> = alice_frames
        .iter()
        .filter(|v| v["type"] == "approval_decision")
        .collect();
    assert_eq!(
        decisions.len(),
        1,
        "alice should have seen exactly 1 decision; frames={alice_frames:?}"
    );
    let d = decisions[0];
    assert_eq!(d["request_id"], req_id);
    assert_eq!(d["approved"], Value::Bool(true));
    let approvals: HashSet<String> = d["approvals"]
        .as_array()
        .expect("approvals array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(
        approvals.contains("bob"),
        "approvals missing bob: {approvals:?}"
    );
    assert!(
        approvals.contains("carol"),
        "approvals missing carol: {approvals:?}"
    );

    let bob_frames = cap_bob.snapshot().await;
    assert!(
        bob_frames.iter().any(|v| v["type"] == "approval_decision"
            && v["approved"] == Value::Bool(true)
            && v["request_id"] == req_id),
        "bob missing decision: {bob_frames:?}"
    );

    let carol_frames = cap_carol.snapshot().await;
    assert!(
        carol_frames
            .iter()
            .any(|v| v["type"] == "approval_decision" && v["approved"] == Value::Bool(true)),
        "carol missing decision: {carol_frames:?}"
    );

    child.kill().expect("kill server");
    let _ = child.wait();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn single_deny_resolves_denied() {
    let port = free_port();
    let mut child = spawn_server(port).await;

    let secret = "approval-quorum-test-secret";
    let room = "approval-room-deny";

    let t_req = issue_token(secret, "dave", room).await;
    let t_v1 = issue_token(secret, "erin", room).await;
    let t_v2 = issue_token(secret, "frank", room).await;

    let (mut sink_dave, cap_dave) = connect_client(port, t_req, room, "dave").await;
    let (mut sink_erin, _cap_erin) = connect_client(port, t_v1, room, "erin").await;
    let (mut sink_frank, cap_frank) = connect_client(port, t_v2, room, "frank").await;

    let req_id = "req-deny-1".to_string();
    send(
        &mut sink_dave,
        SyncMessage::ApprovalRequest {
            request_id: req_id.clone(),
            action: "drop-tables".into(),
            payload: "{}".into(),
            requester: "dave".into(),
            required_approvers: 2,
            ts_ms: dipralix::sync::now_ms(),
        },
    )
    .await;

    // Wait for both voters to receive the request before they
    // cast their votes. This is the same wire-order race the
    // 2-of-3 test guards against.
    wait_for(&cap_frank, "approval_request").await;

    send(
        &mut sink_erin,
        SyncMessage::ApprovalVote {
            request_id: req_id.clone(),
            voter: "erin".into(),
            vote: dipralix::sync::protocol::ApprovalVoteKind::Deny,
            reason: Some("no can do".into()),
            ts_ms: dipralix::sync::now_ms(),
        },
    )
    .await;

    // Frank's late approve should be dropped (no second decision).
    send(
        &mut sink_frank,
        SyncMessage::ApprovalVote {
            request_id: req_id.clone(),
            voter: "frank".into(),
            vote: dipralix::sync::protocol::ApprovalVoteKind::Approve,
            reason: None,
            ts_ms: dipralix::sync::now_ms(),
        },
    )
    .await;

    // Wait for the decision to make its way to all three clients.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let dave_frames = cap_dave.snapshot().await;
        if dave_frames.iter().any(|v| v["type"] == "approval_decision") {
            break;
        }
        if std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;

    let frames = cap_dave.snapshot().await;
    let decisions: Vec<&Value> = frames
        .iter()
        .filter(|v| v["type"] == "approval_decision")
        .collect();
    assert_eq!(
        decisions.len(),
        1,
        "dave should have seen exactly 1 decision; frames={frames:?}"
    );
    let d = decisions[0];
    assert_eq!(d["approved"], Value::Bool(false));
    let denials: HashSet<String> = d["denials"]
        .as_array()
        .expect("denials array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(denials.contains("erin"));

    // Frank should also see the single Denied decision.
    let frank_frames = cap_frank.snapshot().await;
    let decisions_f: Vec<&Value> = frank_frames
        .iter()
        .filter(|v| v["type"] == "approval_decision")
        .collect();
    assert_eq!(
        decisions_f.len(),
        1,
        "frank should see the single Denied decision; got {decisions_f:?}"
    );
    assert_eq!(decisions_f[0]["approved"], Value::Bool(false));

    child.kill().expect("kill server");
    let _ = child.wait();
}
