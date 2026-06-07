# Contributing

Thanks for looking. Short doc — that's the whole intent.

## Setup

```bash
git clone https://github.com/Zyferon/dipralix
cd dipralix
cargo build --release
./target/release/dipralix-cli --version
```

## Dev loop

```bash
cargo watch -x check -x clippy -x test
```

If you don't have `cargo-watch`: `cargo install cargo-watch`.

For a one-shot run while developing:

```bash
cargo run -- --prompt "your prompt here"
```

## What I expect in a PR

- One feature or fix per PR. Don't bundle.
- `cargo check` clean. `cargo clippy -- -D warnings` clean.
- `cargo test` passes.
- No emoji in code, strings, or comments.
- No `TODO`/`FIXME`/`unimplemented!()` in the diff. Real code only.
- Errors return `anyhow::Result<T>`. Async runs on `tokio`.
- If the change is user-facing, add a line under the unreleased section of `CHANGELOG.md`.

I'd rather rebuild a small PR three times than land a big one once.

## Where things live

```
src/main.rs            CLI entry, argument parsing
src/agent.rs           REPL, agentic loop, slash command dispatch
src/backend.rs         Provider dispatch (Gemini SSE, Claude SSE, OpenAI)
src/orchestrator.rs    Multi-model task decomposition + consensus
src/tools.rs           17 built-in tools (declarations + handlers)
src/safety.rs          4-level risk classifier (.dipralix/safety.toml)
src/approval.rs        Per-action approval matrix (.dipralix/approval.toml)
src/memory.rs          Persistent project decisions + cross-project patterns
src/context.rs         Lazy skill assembly from .dipralix/skills/
src/debate.rs          Red/Blue peer review on high-risk bash
src/comment_protocol.rs   // DIPRALIX: directive scanner
src/plan_visualizer.rs    .dipralix/plans/current.md → ASCII graph
src/living_docs.rs     ARCHITECTURE.md auto-sync
src/fingerprint.rs     dipralix --init scaffolding + quality score
src/infra.rs           Dockerfile / K8s / Terraform static analysis
src/browser.rs         /fetch — plain HTTP + HTML→Markdown
src/mcp.rs             MCP client (JSON-RPC 2.0 over stdio)
src/diff_view.rs       Unified diff + per-hunk interactive review
src/session.rs         Save/restore session state
src/audit.rs           JSON audit log of every tool call
src/integrations/      GitHub (12), Discord (7), Gmail (7), Drive (7)
ide/vscode/            VS Code extension (CodeLens for DIPRALIX: comments)
.github/workflows/     CI and release binaries
site/                  Astro single-page site, deployed to Vercel
```

## Adding a tool

1. Add a `FunctionDeclaration` to `tools.rs::get_tool_declarations()`.
2. Implement the handler in `tools.rs::execute_tool()`.
3. If it can modify state or run shell, classify it in `safety.rs`.
4. Bump the expected count in `src/test_harness.rs::core_tool_count_is_correct`.
5. `cargo test`.

## Adding a slash command

1. Add a `match` arm in `agent.rs::run_interactive` (search for `"/help"` to find the dispatch).
2. Add it to the help text in `ui.rs`.
3. Mention it in `README.md`'s "How to drive it" section.

## Adding an integration

1. Create `src/integrations/<service>.rs`.
2. Implement the `IntegrationService` trait from `integrations/mod.rs`.
3. Register it in `IntegrationRegistry::from_config`.
4. Add the config fields to `Config` in `src/config.rs`.

## Questions

Open an issue. I read all of them.
