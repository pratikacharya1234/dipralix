# FORGE WASM Plugin API Specification (v0.1.0-alpha)

This document outlines the initial specification for the FORGE WASM Plugin API. Plugins allow extending FORGE with new tools, domain-specific knowledge, and custom workflows.

## Architecture

FORGE uses `wasmtime` as its high-performance WASM runtime. Plugins are executed in a sandboxed environment with restricted access to the host system, mediated by FORGE's capability system.

## Entry Points

Every plugin must export a set of standard functions that FORGE calls during the lifecycle of a task.

### `fn metadata() -> String`
Returns a JSON string containing the plugin's name, version, and a list of tools it provides.

### `fn call_tool(name: &str, args: &str) -> String`
Invoked when FORGE executes a tool provided by the plugin. Arguments and returns are JSON-encoded strings.

## Host Imports (FORGE SDK)

The host provides several "hooks" that plugins can use to interact with the environment:

- `forge_read_file(path: &str) -> String`
- `forge_write_file(path: &str, content: &str)`
- `forge_grep(pattern: &str, path: &str) -> String`
- `forge_log(message: &str, level: i32)`

## Data Format

All complex data structures exchanged between the host and the WASM guest are serialized as JSON.

## Example (Rust)

```rust
use serde_json::json;

#[no_mangle]
pub extern "C" fn metadata() -> *mut c_char {
    let meta = json!({
        "name": "my-custom-tool",
        "version": "0.1.0",
        "tools": ["analyze_logs"]
    });
    CString::new(meta.to_string()).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn call_tool(name: *mut c_char, args: *mut c_char) -> *mut c_char {
    // Implementation here...
}
```

## Security

Plugins must be explicitly enabled in `.forge/config.toml`. Each plugin can be granted specific permissions (e.g., `filesystem.read`, `network.access`).
