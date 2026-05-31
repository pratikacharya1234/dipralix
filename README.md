# DIPRALIX ◈ NULLVOID

<p align="center">
  <img src="https://img.shields.io/badge/price-FREE-green.svg" alt="100% Free">
  <img src="https://img.shields.io/badge/version-0.1.0-blue.svg" alt="Version 0.1.0">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License">
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange.svg" alt="Rust 1.75+">
  <img src="https://img.shields.io/badge/context-1M%20tokens-green.svg" alt="1M Token Context">
  <img src="https://img.shields.io/badge/models-Gemini%20%7C%20Claude%20%7C%20GPT-purple.svg" alt="Multi-model">
  <img src="https://img.shields.io/badge/binary-12MB-lightgrey.svg" alt="Binary 12MB">
</p>

<p align="center">
  <img src="assets/demo-banner.png" alt="DIPRALIX NULLVOID Terminal" width="90%">
</p>

<p align="center">
  <img src="assets/demo-chat.png" alt="DIPRALIX Domain Bootstrap &amp; Chat" width="90%">
</p>

---

**DIPRALIX** is the open-source, multi-model terminal AI coding agent. 1M token context. Built in Rust. Works with Gemini, Claude, and GPT — routing each task to the best model automatically. Free. No subscriptions. No lock-in.

### What Makes DIPRALIX Different

DIPRALIX is the **only** coding agent that:
- **Has NULLVOID** — spectral terminal theme, zero emoji, pure Unicode geometric glyphs
- **Domain bootstrap** — pre-loads tech stack, architecture, security patterns before you code
- **Gets smarter every session** — ALICE learns from errors and auto-injects lessons
- **Auto-detects project conventions** — language, indentation, build system, linter config
- **Decomposes tasks across AI models** — routes each subtask to the best provider
- **Runs parallel subagents** — critical work to reasoning models, routine to fast models
- **Verifies with a second model** — cross-provider consensus on critical changes
- **Auto-researches before coding** — web searches for docs, APIs, best practices
- **Auto-escalates on failure** — starts cheap, upgrades automatically

### Comparison

| | DIPRALIX | Claude Code | Cursor | Copilot |
|---|---|---|---|---|
| **Price** | Free | $20-200/mo | $20/mo | $10/mo |
| **Open source** | MIT | Proprietary | Proprietary | Proprietary |
| **Multi-model** | Gemini + Claude + GPT | Claude only | Multi-model | GPT only |
| **Max context** | 1M tokens | 200K | ~200K | ~64K |
| **Task decomposition** | Automatic + multi-model | Manual subagents | No | No |
| **Consensus verification** | Cross-provider | No | No | No |
| **Auto-escalation** | Yes | No | No | No |
| **Pre-execution research** | Yes | No | No | No |
| **Auto-learning (ALICE)** | **Yes** | No | No | No |
| **Project DNA detection** | **Yes** | No | No | No |
| **Interface** | Terminal | Terminal | VS Code | VS Code |
| **MCP support** | Yes | Yes | Yes | No |
| **Native integrations** | GitHub, Discord, Gmail, Drive | GitHub | None | GitHub |
| **Terminal theme** | **NULLVOID** spectral | None | VS Code | VS Code |
| **Free tier** | ✅ Gemini 1,500 req/day | ❌ | ❌ | ❌ |

> **EMBER Voice AI** is under active development — real-time voice coding with TTS responses. Shipping in a future release.

## What's New in v0.1.0

Ten native features that other agents pile on through plugins and external services — built into one 12 MB Rust binary.

### Core Intelligence
- **Memory Core** — Persistent project decisions in `.dipralix/memory/decisions.md` + cross-project patterns. Two new tools (`memorize_decision`, `memorize_pattern`) the agent uses without prompting.
- **Lazy Context** — Skills assembled per-request from `.dipralix/skills/` and `~/.dipralix/skills/`. Cuts wasted tokens on irrelevant boilerplate.
- **Peer Review Engine** — Red Team / Blue Team / Arbitrator debate triggers automatically on high-risk bash. Rejections short-circuit execution.

### Developer Experience
- **Comment Protocol** (`/tasks`) — Write `// DIPRALIX: refactor this` in any source file. Dipralix scans, queues, executes on demand, marks `// DIPRALIX-DONE:` when finished.
- **Plan Visualizer** (`/plan`) — Terminal-native ASCII dependency graph with risk badges (safe / review / danger) for `.dipralix/plans/current.md`.
- **Living Docs** (`/docs sync`) — Auto-syncs `ARCHITECTURE.md` with the current codebase, including a Mermaid diagram.
- **Code Fingerprinting** (`dipralix --init`) — Detects stack via `ProjectDna`, scaffolds `.dipralix/{project,conventions}.md`, `safety.toml`, `approval.toml`, prints a 0–100 quality score.

### Power User
- **Approval Matrix** (`/approval`) — Per-action policy with four levels (Auto / Notify / Confirm / Deny). `/approval speed fast` and `safe` flip the whole matrix in bulk.
- **Infra Awareness** (`/infra`) — Static analysis for Dockerfile, Kubernetes manifests, and Terraform. No cloud API calls, no leaked state.
- **Browser Engine** (`/fetch <url>`) — Reqwest fetch + HTML→Markdown extractor + on-disk cache at `~/.dipralix/cache/web/`. Headless Chromium deferred to a later release.

**Breaking change:** project was renamed from FORGE to Dipralix. Binary is `dipralix-cli`, config dir is `.dipralix/`, env var is `DIPRALIX_API_KEY` (legacy `GEMINI_API_KEY` still honored).

## 💯 100% Free — No Credit Card, No Subscription

DIPRALIX works with **Gemini's free tier**. That means:

- ✅ **0 dollars.** Forever. No trial that expires.
- ✅ **1,500 requests per day** on Gemini Flash — more than enough for heavy coding sessions
- ✅ **1 MILLION token context window** — 5x Claude Code, 15x Copilot
- ✅ **No rate limit anxiety** — you're on your own API key, not someone's quota
- ✅ **Pay only if you want to** — add Claude or GPT keys for complex tasks, keep Gemini for everything else

**Get your free key in 30 seconds:**
1. Go to https://aistudio.google.com/apikey
2. Click "Create API Key"
3. Copy it. That's it. No billing setup. No credit card.

```bash
export GEMINI_API_KEY="your-free-key-here"
dipralix-cli
# That's it. You're coding with AI. For free.
```

> **Why this matters:** Claude Code costs $20-200/month plus per-token usage. Users report spending $25 in a single session. Cursor costs $20/month. Copilot costs $10/month. DIPRALIX costs nothing. Use the free Gemini tier for 90% of your work, add Claude only when you need it, switch back anytime. No lock-in. No subscription to cancel.

## Quick Start

Pick the install method for your OS — each downloads the prebuilt binary for v0.1.0 from GitHub Releases.

### macOS · Linux

```bash
curl -fsSL https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.sh | bash
```

Detects `x86_64` and `arm64` automatically. Installs to `/usr/local/bin` (or `~/.local/bin` if it can't get sudo). Falls back to `cargo install` if your platform isn't in the prebuilt matrix.

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.ps1 | iex
```

No admin needed — installs to `%LOCALAPPDATA%\Dipralix\bin\` and adds it to your user PATH.

### Build from source (any OS)

```bash
git clone https://github.com/pratikacharya1234/dipralix.git
cd dipralix
cargo build --release
./target/release/dipralix-cli --version
```

### Get a free API key (30 seconds, no credit card)

→ https://aistudio.google.com/apikey

```bash
export DIPRALIX_API_KEY="your-free-key"   # or GEMINI_API_KEY (legacy, also works)
dipralix-cli
```

### Direct download

Prefer to grab the binary by hand? Pick your platform on the [releases page](https://github.com/pratikacharya1234/dipralix/releases/latest):

| Platform | Asset |
|---|---|
| macOS (Apple Silicon) | `dipralix-v0.1.0-macos-arm64.tar.gz` |
| macOS (Intel) | `dipralix-v0.1.0-macos-x86_64.tar.gz` |
| Linux (x86_64) | `dipralix-v0.1.0-linux-x86_64.tar.gz` |
| Windows (x86_64) | `dipralix-v0.1.0-windows-x86_64.zip` |

### VS Code extension

[`ide/vscode/`](ide/vscode/) ships a small extension that adds a "▶ Run with Dipralix" CodeLens above every `// DIPRALIX:` comment in your source — click it to spawn `dipralix-cli --prompt "<the task>"` in an integrated terminal. Build with `cd ide/vscode && npm install && npm run compile`. See the [extension README](ide/vscode/README.md) for packaging instructions.

## Usage

```bash
# Interactive session
dipralix-cli

# Full task pipeline — research, decompose, dispatch, verify
dipralix-cli --prompt "/task add rate limiting to the API endpoints"

# Model selection (auto by default — routes each task to the best available model)
dipralix-cli                                          # auto-route per message
dipralix-cli --model claude-4-sonnet                  # pin a specific model
dipralix-cli --model gpt-4.1 --openai-api-key "sk-..."
dipralix-cli --model gemini-2.5-pro                   # Gemini reasoning model

# With thinking mode and web grounding
dipralix-cli --think --grounding --model gemini-2.5-pro

# Single prompt, auto-apply, exit
dipralix-cli --auto-apply --prompt "fix all compiler warnings"

# Test-fix loop
dipralix-cli --prompt "/test-fix 'cargo test' 5"
```

## Key Commands

### Orchestration
| Command | Action |
|---|---|
| `/task <requirement>` | Full pipeline: research → decompose → dispatch → consensus |
| `/test-fix <cmd> [N]` | Run tests, fix failures, retry until passing |
| `/model <name\|auto>` | Switch or auto-route models |
| `/explain [on\|off]` | Preview planned actions before executing |

### Memory & Context
| Command | Action |
|---|---|
| `/memorize <fact>` | Save fact to persistent memory |
| `/forget <keyword>` | Remove entries from memory |
| `/memory` | View all memorized facts |
| `/compact` | Summarize history to free context |
| `/load [dir]` | Load directory tree into context |
| `/fingerprint` | Scan + show quality score (0–100) |

### v0.1.0 Native Features
| Command | Action |
|---|---|
| `/tasks [list\|execute N\|dismiss N]` | Work the `// DIPRALIX:` comment queue |
| `/plan [view\|risk]` | Render `.dipralix/plans/current.md` as ASCII graph |
| `/docs sync` | Regenerate `ARCHITECTURE.md` against current code |
| `/approval [show\|speed fast\|speed safe]` | Show / flip the approval matrix |
| `/infra [scan\|security\|optimize]` | Static-analyze Dockerfile / K8s / Terraform |
| `/fetch <url>` | Fetch + extract page to markdown, cached locally |

### Code & Safety
| Command | Action |
|---|---|
| `/undo [N]` | Revert last N file changes |
| `/diff` | Show pending change list |
| `/snapshot` / `/rollback` | Create or restore git snapshots |
| `/tokens` | View context window usage |
| `/cost` | Show session cost |
| `/security` | Full security sweep |

### Sessions & Integration
| Command | Action |
|---|---|
| `/session save\|load\|list` | Manage saved sessions |
| `/history [N]` | Show conversation history |
| `/profile <name>` | Apply config profile |
| `/pr <title>` | Auto-create GitHub PR |
| `/screenshot <path>` | Vision-based code analysis |

Full list: `/help`

## Configuration

`~/.dipralix/config.toml`:

```toml
api_key = "AIza..."
model = "gemini-2.5-flash"
daily_budget_usd = 5.00

# Multi-model keys
anthropic_api_key = "sk-ant-..."
openai_api_key = "sk-..."

[thinking]
enabled = false
budget = 8000

[integrations.github]
token = "ghp_..."

[mcp_servers.postgres]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres"]

[profiles.work]
model = "gemini-2.5-pro"
thinking = true
grounding = true
daily_budget_usd = 10.0
```

Per-project: `.dipralix/project.md` (instructions), `.dipralix/safety.toml` (permissions), `.dipralix/memory.md` (persistent facts).

## Architecture

```
src/
  main.rs            CLI entry point (127 lines)
  agent.rs           Agentic loop, slash commands, streaming (1704 lines)
  backend.rs         Multi-model dispatch: Gemini, Anthropic, OpenAI (1215 lines)
  orchestrator.rs    Task decomposition, parallel subagents, consensus (919 lines)
  types.rs           Canonical message types (Content, Part, FunctionCall) (341 lines)
  tools.rs           16 built-in tools + dispatch (887 lines)
  safety.rs          4-level risk classifier + per-project policy engine (315 lines)
  diff_view.rs       Unified diff + per-hunk interactive review (308 lines)
  snapshot.rs        In-memory undo/redo stack (58 lines)
  session.rs         Binary session persistence (259 lines)
  token_counter.rs   Cost tracking + budget management (235 lines)
  audit.rs           JSON audit logging (60 lines)
  config.rs          Config loading, profiles, context windows (172 lines)
  project.rs         Directory loading, git clone (146 lines)
  security.rs        Security sweep: audit + secret scan + CVE analysis (285 lines)
  ui.rs              Terminal UI, help, context bar (329 lines)
  mcp.rs             MCP client: JSON-RPC 2.0 over stdio (572 lines)
  models.rs          Model resolution and discovery (147 lines)
  integrations/
    mod.rs           Registry + dispatch (163 lines)
    github.rs        12 GitHub API tools (639 lines)
    discord.rs       7 Discord API tools (409 lines)
    google.rs        OAuth2 engine + 7 Drive + 7 Gmail tools (992 lines)
```

## Features

### Agentic Loop
Streaming output with real-time token display. Thinking/reasoning token visualization. Parallel tool execution. Configurable iteration limits. Auto-apply mode. Per-hunk diff review. Stuck detection after 3 consecutive errors. In-memory undo stack.

### Multi-Model Support
Gemini 2.5 Pro/Flash/Lite, Claude 4 Opus/Sonnet, GPT-4.1/GPT-4o/o3/o4-mini. Auto-routing by task complexity. Provider-aware model hints. SSE streaming with proper tool call round-trips per provider.

### Task Orchestrator
5-phase pipeline: Research (auto web-search) → Decompose (break into subtasks) → Dispatch (route to best models, run in parallel) → Consensus (cross-model verification) → Merge (combine results). Cost-intelligent auto-escalation on failure.

### Built-in Tools (17)
`read_file`, `write_file`, `edit_file` (fuzzy matching), `append_file`, `bash` (streaming), `list_files`, `list_symbols`, `search_files` (regex), `glob`, `create_directory`, `delete_file`, `move_file`, `copy_file`, `url_fetch` (cached), `git_snapshot`, `memorize_decision`, `memorize_pattern`

### Native Integrations (33 tools)
GitHub (12), Discord (7), Google Drive (7), Gmail (7). OAuth2 with auto-refresh.

### Safety System
4-level classification: Allow, Warn, Confirm, Deny. Pipe-to-shell detection. Per-project `.dipralix/safety.toml`. Trusted/blocked command lists.

### MCP Support
Full JSON-RPC 2.0 MCP client over stdio. Protocol 2025-03-26 compliance. Auto-discovers tools. Parallel server startup with timeout safety.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for full release history.

## License

MIT — see [LICENSE](LICENSE).

---

<p align="center">
  <b>Built with Rust. Open source. Free forever.</b>
</p>
