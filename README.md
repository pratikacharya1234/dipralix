# DIPRALIX ◈ NULLVOID

<p align="center">
  <img src="https://img.shields.io/badge/price-FREE-green.svg" alt="100% Free">
  <img src="https://img.shields.io/badge/version-0.0.2-blue.svg" alt="Version 0.0.2">
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

> **EMBER Voice AI** is under active development — real-time voice coding with TTS responses. Shipping in v0.0.3.

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

```bash
# One-liner install
curl -fsSL https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.sh | bash

# Or clone and build
git clone https://github.com/pratikacharya1234/dipralix.git
cd dipralix
cargo build --release

# Get a free API key (30 seconds, no credit card)
# → https://aistudio.google.com/apikey

export GEMINI_API_KEY="your-free-key"
dipralix-cli
```

## Usage

```bash
# Interactive session
dipralix-cli

# Full task pipeline — research, decompose, dispatch, verify
dipralix-cli --prompt "/task add rate limiting to the API endpoints"

# With specific models
dipralix-cli --model claude-4-sonnet --anthropic-api-key "sk-ant-..."
dipralix-cli --model gpt-4.1 --openai-api-key "sk-..."
dipralix-cli --model auto     # auto-select best model per task

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

### Built-in Tools (16)
`read_file`, `write_file`, `edit_file` (fuzzy matching), `append_file`, `bash` (streaming), `list_files`, `search_files` (regex), `glob`, `create_directory`, `delete_file`, `move_file`, `copy_file`, `url_fetch` (cached), `git_snapshot`

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
ource. Free forever.</b>
</p>
