# Gemini CLI to FORGE Migration Guide

Welcome to FORGE! This guide will help you transition your workflow from Gemini CLI to FORGE, the open-source, local-first alternative.

## Why FORGE?

FORGE was created to provide a more robust, extensible, and locally-controlled environment for agentic software engineering. While Gemini CLI provided a great foundation, FORGE extends these capabilities with:

- **Local-First Execution:** Support for local LLMs via `llama.cpp` and Ollama.
- **WASM Plugin System:** A high-performance, language-agnostic plugin architecture.
- **Enhanced Domain Knowledge:** Advanced bootstrapping for deep codebase understanding.
- **Open Governance:** No vendor lock-in; you own your tools.

## Installation

If you haven't already, install FORGE using the provided script:

```bash
./install.sh
```

This will compile the Rust core and install the `forge-cli` binary.

## Key Command Equivalents

If your muscle memory is tuned to Gemini CLI, here's how to map those commands to FORGE:

| Gemini CLI | FORGE Equivalent | Note |
|------------|------------------|------|
| `gemini --prompt "fix bug"` | `forge-cli --prompt "fix bug"` | Direct prompt execution |
| `gemini --auto-approve` | `forge-cli --auto-apply` | Skip diff reviews |
| `gemini --skill <name>` | *(Coming soon via WASM Plugins)* | FORGE uses domain bootstrapping |
| `gemini /history` | `forge-cli /history` | Same syntax for built-in slash commands |

## Configuration

FORGE uses a similar configuration philosophy but moves away from `.geminiignore` in favor of standard `.gitignore` and `.forge/config.toml`.

- **Gemini:** `~/.gemini/`
- **FORGE:** `~/.forge/`

## The "You Own It" Philosophy

Google may turn off their CLI, but no one can turn off FORGE. It is built to compile locally, run local open-source models, and never lock you into a single vendor's ecosystem.

Welcome home.
