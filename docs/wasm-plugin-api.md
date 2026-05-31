# WASM plugin API — design notes

**Status: not implemented.** This file describes the shape I want for a plugin system. The runtime hooks don't exist in `dipralix-cli` v0.1.0. I'm posting the design so anyone interested can argue with it before I start writing.

## Why a plugin system at all

Today, adding a new tool means editing `src/tools.rs`, recompiling, and shipping a new binary. That's fine for me, but it means you can't extend the agent without forking. A WASM plugin layer would let people drop a `.wasm` file in `~/.dipralix/plugins/` and have it show up as a tool the next time the agent runs.

## Why WASM, not native plugins or scripts

- **Sandbox by default.** No filesystem, no network, no syscalls unless I explicitly grant them. Native plugins or shell scripts would either be unsafe or require their own sandbox.
- **Language-agnostic.** Rust, Go, TypeScript, Zig, C — anything that compiles to wasm32.
- **One binary still works.** The dipralix binary would embed `wasmtime`. No external runtime to install.

## Plugin contract

Every plugin exports two functions and may import a small set of host functions for the things it can't do on its own.

```
// Required exports
fn metadata() -> String        // JSON: { name, version, tools: [...] }
fn call_tool(name: &str, args: &str) -> String   // JSON in, JSON out

// Optional host imports (provided by dipralix to the plugin)
fn dipralix_read_file(path: &str) -> String
fn dipralix_write_file(path: &str, content: &str)
fn dipralix_grep(pattern: &str, path: &str) -> String
fn dipralix_log(message: &str, level: i32)
```

A plugin that only computes on its inputs doesn't need to import anything. A plugin that touches the workspace asks for the imports it needs and the user approves the capabilities at install time.

## Distribution

There's no marketplace plan. Plugins are `.wasm` files. You install one by dropping it in `~/.dipralix/plugins/` (or `.dipralix/plugins/` in a repo). The agent lists them on startup, the user can disable any of them with `/plugins disable <name>`.

## Timeline

No commitment. The Phase 2 features in v0.1.0 (Memory Core, Comment Protocol, Approval Matrix, etc.) come first. If you want to help shape this, open an issue.
