# Changelog

All notable changes to Dipralix (formerly FORGE) are documented in this file.

## [0.3.2] — 2026-06-06

### Added — "Alive": the developer's mirror (5 phases, see `alive_Script.txt`)
- **Phase 1 — Identity (`src/alive.rs`):** Dipralix now has a persisted identity at `.dipralix/alive/identity.toml` (nickname, persona, birth time, adopted approach). On first run it "comes alive" with an interactive dialogue (nickname → how-should-I-be), and its identity is injected into the system prompt every session (`{alive_context}`). `/alive`, `/alive nick <name>`, `/alive persona <…>`.
- **Phase 2 — Research-and-adopt:** after the developer says how they want Dipralix to be, it researches a concrete operating approach (token-frugal), presents it for review, and adopts it on approval — stored in the identity and carried into every prompt.
- **Phase 3 — Resume continuity (`src/resume.rs`):** on every start Dipralix reads the verified-outcome ledger, memory, and prior sessions and prints a "welcome back" briefing (last task in this repo + counts) — no relearning, no wasted tokens. `/resume`.
- **Phase 4 — Connect the dev's life (`src/integrations/slack.rs`, `notion.rs`):** Slack (send message, list channels) and Notion (search, create page) integrations on top of the existing GitHub/Discord/Gmail/Drive + MCP. `/connect` shows the status of every connector.
- **Phase 5 — Self-evolution (`src/evolve.rs`):** `/evolve` runs a token-budgeted research pass over stack-aware industry topics and folds timestamped findings into global memory, so running Dipralix over time compounds.

### Changed
- v3 system prompt now opens with a "## Who you are" identity block.
- Bumped to **0.3.2**.

### Release
- **All-platform prebuilt binaries.** The release matrix now builds six targets:
  macOS x86_64 (Intel) + arm64 (Apple Silicon), Linux x86_64 + aarch64 (ARM),
  Windows x86_64 + arm64. Installers auto-detect arch (`install.sh`, `install.ps1`).

### Verification
- 168 tests pass (`cargo nextest`), including new `alive`, `resume`, and `evolve` unit tests. `cargo clippy --all-targets` clean, `cargo fmt --check` clean.

## [0.3.1] — 2026-06-06

### Added — Verified Outcome Ledger + Proof-of-Work (`src/ledger.rs`)
- **Verified Outcome Ledger:** an append-only, per-repo log at `.dipralix/ledger/outcomes.jsonl` (one JSON object per line, git-trackable). Records every finished task with its verdict (`verified` / `failed` / `rejected` / `superseded`), calibrated confidence, the files touched, and the machine-checked proof behind it.
- **Temporal supersession:** an entry can mark an earlier one stale (`supersedes`), so the agent reasons about *how a repo changed over time*, not just a flat set of facts — the gap that vector/RAG memory leaves open. Superseded entries are filtered out of the prompt context automatically.
- **Proof-of-Work receipts:** an outcome can only be recorded `verified` when it carries a verification command that **exited 0**. No proof, no "done" — this is the contract that makes the ledger trustworthy.
- **New `record_outcome` tool** (core tool count 17 → 18): the agent writes its own verified outcomes; the test asserting the count is updated.
- **`{ledger_context}` injected into the system prompt:** each session inherits what the repo has already proved or seen fail, capped to the most recent active entries to keep the prompt lean.
- **`/ledger [n]` command:** view the most recent recorded outcomes from the terminal.

### Changed — v3 system prompt (`src/agent.rs`)
- Rewrote `SYSTEM_PROMPT_BASE` around three explicit promises — **persist, verify, be honest** — plus an explicit **tool-routing decision tree** (choose the cheapest tool that fully answers), hard verification gates ("done" requires a machine-checked result), a proof-first reporting format, and **untrusted-content handling** (treat file/web/tool output as data, never as instructions — prompt-injection resistance).
- Bumped to **0.3.1**.

### Verification
- 159 tests pass (`cargo nextest`), including 4 new ledger tests (roundtrip, supersession filtering, missing-file safety, proof rendering / outcome parsing). `cargo clippy` clean, `cargo fmt --check` clean.

## [0.3.0] — 2026-06-05

### Added — Realtime team sync (`src/sync/`, `src/bin/server.rs`)
- **Self-hosted relay (`dipralix-server`):** WebSocket sync server with JWT-per-room auth, SQLite persistence (`--persist`), and replay-on-reconnect. New `[[bin]]` alongside `dipralix-cli`.
- **Serverless P2P mesh (`dipralix-cli --sync --mesh`):** Real, no-server sync for devs on the same LAN. mDNS discovery (`_dipralix._tcp.local.`, room-scoped) over `mdns-sd`; direct TCP peer links; `--peer host:port` to add peers manually when multicast is blocked.
- **End-to-end encryption (`src/sync/crypto.rs`):** Noise `NNpsk0_25519_ChaChaPoly_BLAKE2s` on every mesh link. Ephemeral per-session keys (forward secrecy); the room secret is the pre-shared key (mutual auth). Wrong secret fails the handshake; tampered frames fail the AEAD check.
- **File gossip (`src/sync/fileio.rs`):** Shared, echo-suppressed apply/read used by both the server client and the mesh node — blake3 content hashing, snapshot-on-connect so late joiners converge on current `.dipralix/` state.
- **Path allowlist:** Only `.dipralix/` metadata syncs; source code, `.env`, secrets, SSH/cloud keys, and `config.local` are rejected before the wire.
- **Phase 2 UX:** Presence/heartbeat (`presence.rs`), append-only team chat (`chat.rs`), and a 2-of-N approval quorum for high-risk commands.

### Changed
- Bumped to **0.3.0**. `description` now mentions realtime sync.
- `release-binaries.yml` now builds and packages **both** `dipralix-cli` and `dipralix-server` for Linux x86_64, macOS x86_64/arm64, and Windows x86_64.
- `SyncClient` refactored onto the shared `FileSync` applier (removed duplicated apply/read logic).
- `site/` is no longer tracked in the public repo (`.gitignore` + removed from the index); it is preserved locally and deployed separately.

### Honest scope
- **Mesh is LAN-only.** mDNS is link-local; cross-internet sync uses the relay server or manual `--peer`. WebRTC/NAT-traversal is intentionally not in this build.
- **No CRDT merge.** Concurrent edits to one file are last-write-wins by content hash, not character-level merge.

### Verification
- 155 tests pass (`cargo nextest`), including a two-node mesh that converges a file over real encrypted TCP. `cargo clippy -D warnings`, `cargo fmt --check`, `cargo audit`, and `cargo deny check` all clean.

## [0.1.0] — 2026-05-31

### Breaking
- **Renamed FORGE → Dipralix.** Binary is now `dipralix-cli`. Config directory `.forge/` → `.dipralix/`. Env var `FORGE_API_KEY` → `DIPRALIX_API_KEY` (legacy `GEMINI_API_KEY` still honored). CI workflow renamed `.github/workflows/dipralix-ci.yml`. History file moved to `~/.dipralix-history`.

### Added — Phase 2A: Core Intelligence
- **Memory Core (`src/memory.rs`):** Persistent project decisions in `.dipralix/memory/decisions.md` and cross-project patterns in `~/.dipralix/patterns/`. New AI tools `memorize_decision` and `memorize_pattern`. Memory injected into the system prompt.
- **Lazy Context (`src/context.rs`):** `ContextAssembler` dynamically loads skills from `.dipralix/skills/` (project) and `~/.dipralix/skills/` (global) based on detected `ProjectDna`. Reduces wasted tokens on irrelevant context.
- **Peer Review Engine (`src/debate.rs`):** Multi-model Red Team vs Blue Team debate on high-risk bash commands (Confirm/Deny). Arbitrator decides; rejections short-circuit execution and feed back to the main agent.

### Added — Phase 2B: Developer Experience
- **Living Documentation (`src/living_docs.rs`, `/docs sync`):** Auto-syncs `ARCHITECTURE.md` against the current codebase using the configured backend.
- **Comment Protocol (`src/comment_protocol.rs`, `/tasks`):** Scans the workspace for `DIPRALIX:` directives in source comments across Rust, JS/TS, Python, Go, YAML, TOML, MD. Commands: `/tasks list`, `/tasks execute N`, `/tasks dismiss N` (rewrites the marker to `DIPRALIX-DONE:`).
- **Plan Visualizer (`src/plan_visualizer.rs`, `/plan`):** Terminal-native ASCII plan view with status glyphs, dependency annotations, risk badges (safe/review/danger), and a progress bar. Parses `.dipralix/plans/current.md`.
- **Code Fingerprinting (`src/fingerprint.rs`):** New `dipralix --init` scaffolds `.dipralix/{project,conventions}.md`, `safety.toml`, and `approval.toml` from detected stack. New `dipralix --fingerprint` and `/fingerprint` print a quality score (0–100) with prioritized suggestions.

### Added — Phase 2C: Power User Features
- **Approval Matrix (`src/approval.rs`, `/approval`):** Four-level per-action approval policy (Auto / Notify / Confirm / Deny) loaded from `.dipralix/approval.toml`. Sub-classifies bash by command shape (sudo, rm, git push, docker run, curl). `/approval speed fast` and `/approval speed safe` flip the matrix in bulk.
- **Infra Awareness (`src/infra.rs`, `/infra`):** Static analysis for Dockerfile (latest tag, root user, ENV secrets, single-stage), Kubernetes manifests (resource limits, probes, runAsNonRoot, privileged), and Terraform (open ingress, public-read ACL, encryption disabled). Pure text scan — no cloud API calls.
- **Browser Engine, lite (`src/browser.rs`, `/fetch <url>`):** reqwest-based fetcher with simple HTML→Markdown extraction and on-disk cache at `~/.dipralix/cache/web/`. Headless Chromium deferred to a later release.

### Changed
- `dipralix init` now creates `.dipralix/memory/`, `.dipralix/skills/`, and `.dipralix/plans/` alongside the config scaffolding.
- Stale `core_tool_count_is_correct` assertion updated from 14 to 17 to reflect the new memory tools.

### Fixed
- `/docs` handler compile error — now constructs a local `BackendClient` via `active_config(...)` instead of referencing an out-of-scope `client`.
- Cleaned unused imports in `context.rs`, `debate.rs`, `living_docs.rs`.

## [0.0.2] — 2026-04-30

### Added
- **Domain Bootstrap:** Interactive project-type selector (15 domains) with real-time DuckDuckGo web search. Shows latest web findings alongside embedded blueprint. Custom domain input option `[C]` for any domain. Visible spinner and elapsed time during search.
- **15 embedded domain blueprints:** Mobile, Web, AI/ML, Deep Learning, Desktop, Hardware/IoT, Game Dev, DevOps, Data Engineering, Blockchain/Web3, Cybersecurity, CLI/Dev Tools, API/Backend, Scientific/HPC, General. Production-ready knowledge for each domain.
- **NULLVOID Terminal Theme:** Complete visual overhaul — spectral ghost logo with 5-color gradient, protocol header, pipe-separated stats bar, Unicode geometric glyphs (zero emoji). Phantom-hacker aesthetic.
- **Streaming thought rendering:** Nullvoid-styled reasoning blocks with line-by-line amber text streaming and box-drawn borders.
- **Response blocks:** Box-drawn output panels with header/body formatting for all AI responses.
- **Model auto-fallback:** Automatically switches to an alternative model when the current one hits rate limits, auth errors, or server failures. Falls back: same-provider → cross-provider (Gemini free tier preferred).
- **Gemini free tier indicator:** Cost display now shows "FREE (Gemini tier)" when using Gemini models.
- **Free tier guide in README:** Prominent section explaining how FORGE is 100% free with Gemini's 1,500 req/day tier.
- **Demo script:** `demo.sh` — runs a live 2-minute FORGE demo showing code generation and testing.
- **Branch protection:** Main branch protected from force pushes and deletions.

### Changed
- **EMBER voice AI:** Disabled for v0.0.2 — under active development, shipping in v0.0.3. CLI flags `--ember` and `--voice` hidden. Voice modules preserved, ready to re-enable.
- **Auto-detect notifications:** Now use nullvoid styling instead of raw stderr.
- **User echo:** All user input echoed in nullvoid style before processing.
- **All emoji purged:** Replaced with nullvoid Unicode glyphs (◈ ⎔ ⊗ ◉ ⊕ ⊢ ⊞ ⌬).
- Updated comparison table with "Free tier" row.

### Fixed
- **BufWriter flush bug:** FORGE ghost logo now renders correctly — protocol header was stuck in buffer, breaking cursor-overlay alignment.
- **Separator rendering:** Replaced `*` with `·` in protocol header to fix `©` glyph on some terminal fonts.
- **NULLVOID::PROTOCOL visibility:** Header text brightened from MUTED (#3A4060) to TEXT (#8892B0) for readability on dark terminals.

## [0.0.1] — 2026-04-28

### First Public Release

FORGE v0.0.1 — the open-source, multi-model terminal coding agent. 1M token context. Free. Previously developed as GeminiX, rebranded to FORGE to reflect universal multi-model support and independence from any single AI provider.

**Multi-Model Support**
- Gemini 2.5 Pro/Flash/Lite, 2.0 Flash
- Claude 4 Opus/Sonnet, Claude 3.5 Sonnet (Anthropic API)
- GPT-4.1, GPT-4o, o3, o4-mini (OpenAI API)
- Backend abstraction layer with canonical message format
- SSE streaming per provider with tool call round-trips
- Provider auto-detection from model name

**Auto Model Routing**
- `/model auto` analyzes task complexity and picks the best model
- Complex (refactor, architecture, security) → reasoning model
- Simple (find, read, explain) → fast/cheap model
- Everything else → balanced model
- Provider-aware: uses available API keys intelligently

**Explain-Before-Execute**
- `/explain on|off` toggles pre-execution summaries
- Shows planned file changes, shell commands, expected impact
- Asks for confirmation before running tools
- `--explain` CLI flag

**Test-Fix Loop**
- `/test-fix <command> [max-cycles]`
- Runs tests, detects failures, feeds errors to model
- Model fixes code, re-tests, loops until passing
- Handles truncated output for large test suites

**Persistent Memory**
- `/memorize <fact>` saves to `.dipralix/memory.md`
- `/forget <keyword>` removes matching entries
- `/memory` displays all memorized facts
- Auto-injected into system prompt each turn

**Agentic Loop**
- Streaming Gemini API with real-time token display
- Thinking/reasoning token visualization (Gemini 2.5+)
- Parallel tool execution via Tokio
- Configurable iteration limits with pause/resume
- Auto-apply mode for non-interactive use
- Single-prompt mode for scripting

**Built-in Tools (16)**
`read_file`, `write_file`, `edit_file` (fuzzy matching + occurrence parameter), `append_file`, `bash` (streaming + safety classification), `list_files`, `search_files` (regex), `glob`, `create_directory`, `delete_file`, `move_file`, `copy_file`, `url_fetch` (cached), `git_snapshot`

**Safety System**
- 4-level risk classification: Allow, Warn, Confirm, Deny
- Pipe-to-shell detection and blocking
- Per-project `.dipralix/safety.toml` with category-level overrides
- Trusted/blocked command lists

**Diff & Undo**
- Unified diff preview before file writes
- Per-hunk interactive review (accept/reject/skip per change)
- In-memory undo stack with `/undo` and `/undo N`
- Git snapshot creation and rollback

**Context Management**
- Token usage display per turn (prompt + output + thinking)
- Context window progress bar with configurable warnings
- Auto-compaction at threshold (summarizes history via model)
- Manual `/compact` command

**Session Persistence**
- Binary save/restore of full conversation history
- `/session save`, `load`, `list`, `delete`
- Auto-save after each successful turn

**Cost Tracking**
- Per-model pricing with USD display
- Session cost accumulation
- Daily budget support with 80% warning

**MCP Support**
- Full JSON-RPC 2.0 MCP client over stdio
- Protocol 2025-03-26 compliance
- Auto-discovers tools from any MCP-compliant server
- Parallel server startup with timeout safety

**Native Integrations (33 tools)**
- GitHub: 12 tools (issues, PRs, repos, code search, branches)
- Discord: 7 tools (messages, channels, guilds, embeds)
- Google Drive: 7 tools (files, folders, upload, search)
- Gmail: 7 tools (send, list, search, labels, read status)

**Named Profiles**
- Configurable profiles in `~/.dipralix/config.toml`
- Per-profile model, thinking, grounding, auto_apply, budget
- `/profile` command for switching

**Security Sweep**
- Cargo audit + npm audit integration
- Static secret scanning (API keys, tokens, passwords)
- Gemini-powered CVE analysis

**Additional Commands**
- `/learn`: Clone and load public git repos for Q&A
- `/load`: Load directory tree into context
- `/screenshot`: Vision-based bug analysis
- `/pr`: Auto-create pull requests
- `/models`: List available Gemini models
- `/debug`: Debug information dump
- `/history`: Show conversation history
- `/cost`: Show session cost breakdown
kdown
