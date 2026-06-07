# dipralix

A multi-model terminal coding agent with realtime team sync. Written in Rust, shipped as two small binaries. Free with Gemini's API; brings Claude and OpenAI keys along when I want them.

I built this because the rest of the field made the wrong tradeoffs for me. Cursor logged me out twice a week. Claude Code's billing made me check the dashboard before I started writing. Aider had the right philosophy but the wrong language. So I sat down, wrote my own, and shipped it.

It's not the best at any one thing. It is the one I reach for first.

```
$ dipralix-cli
  [MODEL] auto → claude-4-sonnet  (complex task → Claude balanced reasoning)
  [OK] Loaded .dipralix/safety.toml
  v0.3.2  ·  alive identity + cross-session memory  ·  18 tools  ·  verified-outcome ledger  ·  6 integrations  ·  realtime sync

>>> add rate limiting to the public /api/users endpoint, 60 rpm per IP

  Plan:
   1. Read src/api/users.rs                          (read_file)
   2. Add tower::ServiceBuilder rate limiter         (edit_file)
   3. Wire it in main.rs router                      (edit_file)
   4. Add an integration test                        (write_file)
   5. Build + test                                   (bash)

  Continue? [Y/n]
```

That's the whole loop. It plans, it does, it verifies, it tells you what happened.

---

## ■ Install

Pick the line for your machine. Each one drops the prebuilt v0.3.2 binaries (`dipralix-cli` and `dipralix-server`) on your PATH.

```bash
# macOS · Linux
curl -fsSL https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.sh | bash
```

```powershell
# Windows
irm https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.ps1 | iex
```

```bash
# From source — needs Rust 1.75+
git clone https://github.com/pratikacharya1234/dipralix
cd dipralix && cargo build --release
```

Then get a Gemini key at https://aistudio.google.com/apikey and:

```bash
export DIPRALIX_API_KEY="your-key"
dipralix-cli
```

That's it. No login server. No telemetry. No subscription. You can `dipralix-cli --version` and read every line of code that ran.

---

## ◆ What's in the box

- **Two binaries.** `dipralix-cli` (the agent) and `dipralix-server` (the optional sync relay). Static, no Node, no Python, no Docker.
- **Three providers, one interface.** Gemini, Claude, OpenAI. Default routing is `auto` — it reads your prompt and picks a model. Override with `--model gemini-2.5-pro` when you know better.
- **18 tools.** Read, write, edit, append, bash, list, list_symbols, search, glob, mkdir, delete, move, copy, url_fetch, git_snapshot, memorize_decision, memorize_pattern, record_outcome.
- **Alive — the developer's mirror.** On first run Dipralix comes alive: you give it a nickname and tell it how to be; it researches an approach, you approve it, and that identity is carried into every session (`.dipralix/alive/identity.toml`). On every later start it reads its memory and resumes where you left off — no relearning. `/alive`, `/resume`, and `/evolve` (fold industry changes into memory over time).
- **Verified Outcome Ledger.** An append-only, per-repo log (`.dipralix/ledger/outcomes.jsonl`) of what the agent has actually *verified* — with the build/test proof attached, calibrated confidence, and temporal supersession when a past fact stops being true. The agent can't mark anything "verified" without a command that exited 0. Injected into its context each session, so it inherits what the repo already proved. `/ledger` to view.
- **37 integration calls.** GitHub (12), Discord (7), Gmail (7), Drive (7), Slack (2), Notion (2). OAuth2 / bot-token auth. `/connect` shows what's wired.
- **MCP client.** Speaks JSON-RPC 2.0 over stdio. Auto-discovers tools from any MCP server you start.
- **4-level safety.** Allow, Warn, Confirm, Deny. Configurable per project in `.dipralix/safety.toml`.
- **Approval matrix.** Per-action policy (Auto / Notify / Confirm / Deny) in `.dipralix/approval.toml`.
- **Memory.** Project decisions in `.dipralix/memory/`, cross-project patterns in `~/.dipralix/patterns/`. Plain markdown. Git-trackable.
- **Comment protocol.** Write `// DIPRALIX: refactor this` in any source file. `/tasks execute N` runs it.
- **Static infra scanning.** `/infra security` reads Dockerfile, Kubernetes YAML, and Terraform — points at the obvious issues without calling out to any cloud.
- **1M token context** when you use Gemini. Drop your whole repo in.

---

## ● Realtime team sync

Two developers, two terminals, one shared `.dipralix/` — memory, plans, skills, and approval policy stay in sync as you work. Source code never leaves your machine (only `.dipralix/` metadata syncs; an allowlist rejects source, `.env`, secrets, and keys before they touch the wire).

Two ways to connect, both end-to-end encrypted:

```bash
# ◇ Serverless mesh — same LAN, no server. Peers find each other over mDNS.
#   The shared secret is stretched into a Noise key; only peers who know it join.
dipralix-cli --sync --mesh --room myproject --secret "team-passphrase" --user alice

# ◆ Self-hosted relay — across networks. One person runs the server on any box.
dipralix-server --port 7878 --token-secret "$SYNC_SECRET"
dipralix-cli --sync --server ws://your-host:7878 --token "$JWT" --room myproject --user alice
```

What is real today and tested:

- **Encrypted transport.** Noise `NNpsk0` (X25519 + ChaCha20-Poly1305) on every mesh link; ephemeral keys per session, mutual auth from the room secret. A captured frame is ciphertext.
- **mDNS LAN discovery.** `_dipralix._tcp.local.`, room-scoped. No STUN, no TURN, no central server.
- **File gossip with echo-suppression.** blake3 content hashing; snapshot-on-connect so a late joiner converges on current state.
- **Server relay.** JWT-per-room auth, SQLite persistence (`--persist`), replay-on-reconnect.
- **Presence, team chat, and a 2-of-N approval quorum** for high-risk commands.

What is **not** done yet — said plainly:

- **Mesh is LAN-only.** mDNS is link-local, so the serverless mesh finds peers on the same network. Across the internet, use the relay server or pass `--peer host:port` manually. WebRTC/NAT-traversal is intentionally not in this build.
- **No CRDT merge yet.** Concurrent edits to the same file are last-write-wins by content hash, not a character-level merge.

---

## ▲ What's honest

Some things I don't pretend to have figured out:

- **MCP OAuth.** Servers that need a browser flow still want manual `mcp auth` steps. On the list.
- **The headless browser.** `/fetch <url>` does plain HTTP today with a hand-written HTML→Markdown extractor. Pages that need JS to render don't work. Headless Chromium is in the plan, not in the build.
- **No streaming for OpenAI tool calls.** Claude and Gemini stream, OpenAI gets the whole response at the end. It works, but it's slower to feel.
- **Auto-routing is heuristic.** It picks based on keywords in your prompt. It's right most of the time. When it isn't, `--model` is one flag away.
- **Approval matrix only gates bash today.** The Auto/Notify/Confirm/Deny matrix is wired for `bash` and its variants. File-tool gating ships next.
- **Voice mode is shelved.** The EMBER module exists in the tree behind a feature flag. It's not part of the default build because the cross-platform mic story isn't where I want it.

If you hit something rough, file it: https://github.com/pratikacharya1234/dipralix/issues. Use the bug template. I read all of them.

---

## ◇ Configure

`~/.dipralix/config.toml`:

```toml
# Pick one, or set all three to switch freely
api_key            = "AIza..."     # Gemini (free at aistudio.google.com/apikey)
anthropic_api_key  = "sk-ant-..."
openai_api_key     = "sk-..."

model              = "auto"        # or pin: "claude-4-sonnet", "gpt-4.1", etc.
daily_budget_usd   = 5.00          # warns when you cross it
thinking           = false         # turn on for hard problems

[integrations.github]
token = "ghp_..."

[mcp_servers.postgres]
command = "npx"
args    = ["-y", "@modelcontextprotocol/server-postgres"]

[profiles.serious]
model            = "gemini-2.5-pro"
thinking         = true
daily_budget_usd = 10.0
```

Per-project: `.dipralix/project.md` (instructions the agent reads on startup), `.dipralix/safety.toml` (which commands can run unattended), `.dipralix/approval.toml` (per-action approval level), `.dipralix/memory/` (decisions worth remembering).

`dipralix-cli --init` writes a sensible starter set of all four.

---

## ▸ How to drive it

The agent runs in a REPL. Slash commands cover the things you don't want to say in English:

```
/model auto                 route per task (default)
/model claude-4-sonnet      pin a model
/think on                   turn thinking mode on
/explain                    preview each tool call before it runs
/tasks                      list // DIPRALIX: comments waiting in the codebase
/plan view                  render .dipralix/plans/current.md as a graph
/docs sync                  regenerate ARCHITECTURE.md
/approval                   show the approval matrix
/infra security             scan Dockerfile / K8s / Terraform
/ledger                     view the verified-outcome ledger (recent first)
/alive [nick|persona ...]   view or set Dipralix's identity
/resume                     show where we left off in this repo
/evolve                     research industry changes into memory
/connect                    show connected developer tools
/fetch https://docs.rs/...  fetch + extract markdown, cached locally
/fingerprint                detect stack, print quality score
/cost                       what this session has cost
/undo                       reverse the last file change
/help                       everything
```

Or hit it once, headless:

```bash
dipralix-cli --prompt "find every TODO older than 3 months and group by file"
dipralix-cli --prompt "/test-fix 'cargo test' 5"
dipralix-cli --ci                 # JSON output, exit code = success
```

---

## ▪ Why I wrote this

I write Rust most days. I want my tools to be Rust too. I want them to be one binary I can put on a Pi, on a server, on my laptop, the same binary. I want to read the code that's running. I want a free tier I can introduce my friends to without asking for a credit card. I want to not get logged out.

The big AI shops are doing fine. They don't need my money to ship the next model. What I needed was a layer that sits *on top of* their APIs, owned by me, doing what I tell it.

So that's what this is.

---

## □ Architecture (one page)

```
src/
  main.rs            CLI entry. Resolves api keys, model, profile, then hands off.
  bin/server.rs      dipralix-server: WebSocket relay, JWT auth, SQLite persistence.
  agent.rs           The REPL, the agentic loop, the slash commands. Streams output.
  backend.rs         Provider dispatch. Gemini SSE, Anthropic SSE, OpenAI non-stream.
  orchestrator.rs    Decompose → dispatch → consensus → merge. Multi-model pipeline.
  tools.rs           17 tools. Function declarations + handlers.
  safety.rs          Risk classifier. Loads .dipralix/safety.toml.
  approval.rs        Per-action approval matrix. .dipralix/approval.toml.
  memory.rs          Persistent decisions + cross-project patterns.
  ledger.rs          Verified Outcome Ledger — append-only proof-of-work log.
  alive.rs           Identity layer — nickname, persona, "coming alive" first run.
  resume.rs          Welcome-back briefing — resume from memory, no relearning.
  evolve.rs          Self-evolution — research industry changes into memory.
  context.rs         Lazy skill assembly from .dipralix/skills/.
  debate.rs          Red/Blue peer review on high-risk bash.
  comment_protocol.rs  Scans for // DIPRALIX: directives.
  plan_visualizer.rs   ASCII dependency graph for .dipralix/plans/.
  living_docs.rs     Regenerates ARCHITECTURE.md.
  fingerprint.rs     Project DNA detection + quality score.
  infra.rs           Static analysis for Dockerfile, K8s, Terraform.
  browser.rs         Plain HTTP fetch + HTML→Markdown.
  mcp.rs             MCP client. Protocol 2025-03-26.
  integrations/      GitHub, Discord, Gmail, Drive, Slack, Notion.
  session.rs         Session save/restore.
  audit.rs           JSON audit log of every tool call.
  sync/              Realtime sync: protocol, server client, mesh (mDNS + Noise
                     TCP), crypto, discovery, presence, chat, approval quorum.
```

If you want to add a tool, `CONTRIBUTING.md` walks you through it.

---

## ▶ IDE bridge

There's a VS Code extension at [`ide/vscode/`](ide/vscode/). It adds one feature: a "▶ Run with Dipralix" CodeLens above every `// DIPRALIX:` comment in your source. Click it, the task runs in a terminal. Build it with `cd ide/vscode && npm install && npm run compile`. Package and install per the [extension README](ide/vscode/README.md).

---

## ◈ License

MIT.

## ◎ Thanks

The model providers built the APIs. The Rust ecosystem built the rest — `tokio`, `reqwest`, `clap`, `serde`, `rustyline`, `walkdir`, `colored`, `anyhow`, `thiserror`, plus `snow` (Noise), `mdns-sd`, `tokio-tungstenite`, `blake3`, and `rusqlite` for the sync layer. I just glued them together with opinions.

---

If this saves you an hour, star it. If it doesn't, tell me what broke.

— [pratikacharya1234](https://github.com/pratikacharya1234)
