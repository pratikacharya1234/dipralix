# Contributing to DIPRALIX

Thanks for your interest in contributing. This document covers how to set up, build, and submit changes.

## Setup

```bash
git clone https://github.com/pratikacharya1234/dipralix.git
cd dipralix
```

## Build

```bash
cargo build --release
```

The binary will be at `target/release/dipralix`.

## Development

```bash
# Watch mode (install cargo-watch first)
cargo watch -x check -x test

# Run with dev profile
cargo run

# Run with specific model
cargo run -- --model gemini-2.5-pro
```

## Project Structure

```
src/
  main.rs           CLI entry point, argument parsing
  agent.rs          Agentic loop, slash commands, streaming orchestration
  gemini.rs         Gemini API client (SSE streaming, function calling)
  tools.rs          16 built-in tools + dispatch + caching
  safety.rs         4-level risk classifier + policy engine
  diff_view.rs      Unified diff generation + per-hunk interactive review
  snapshot.rs       In-memory undo stack
  session.rs        Session save/restore persistence
  token_counter.rs  Cost tracking + budget management
  audit.rs          JSON audit log
  config.rs         Config loading, profiles, context window sizes
  project.rs        Directory loading, git clone for /learn
  security.rs       Security sweep (cargo/npm audit, CVE analysis)
  ui.rs             Terminal UI, help output, context bar
  mcp.rs            MCP client (JSON-RPC 2.0 over stdio)
  models.rs         Model resolution and discovery
  integrations/
    mod.rs          Registry + dispatch
    github.rs       12 GitHub API tools
    discord.rs      7 Discord API tools
    google.rs       OAuth2 engine + 7 Drive + 7 Gmail tools
```

## Code Style

- Follow Rust idioms and clippy recommendations
- No emoji in code, strings, or comments
- No placeholder implementations, TODOs, or dummy data
- Real production code only
- Use `anyhow::Result` for fallible functions
- Async functions use `tokio`

## Adding a New Tool

1. Add the function declaration in `tools.rs::get_tool_declarations()`
2. Implement the handler in `tools.rs::execute_tool()`
3. Add safety classification if destructive
4. Test with `cargo check`

## Adding a New Integration

1. Create `src/integrations/newservice.rs`
2. Implement `IntegrationService` trait from `integrations/mod.rs`
3. Register in the `IntegrationRegistry`
4. Add config fields to `Config` struct

## Adding a Slash Command

1. Add the match arm in `agent.rs` (look for the slash command dispatch)
2. Add help text in `ui.rs`
3. Document in README.md

## Pull Requests

- Keep PRs focused on one feature or fix
- Run `cargo check` and `cargo build --release` before submitting
- Ensure 0 warnings
- Update CHANGELOG.md for user-facing changes

## Questions

Open an issue on GitHub.
