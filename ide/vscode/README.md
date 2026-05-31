# Dipralix for VS Code

Run the Dipralix terminal agent from inside VS Code. Two features:

1. **CodeLens action above every `// DIPRALIX:` comment.** Click "▶ Run with Dipralix" and the comment description is sent as a one-shot `dipralix-cli --prompt` task in an integrated terminal.
2. **Command palette commands** for the most common Dipralix invocations:
   - `Dipralix: Open terminal session` — drops you straight into `dipralix-cli`
   - `Dipralix: Fingerprint project` — runs `dipralix-cli --fingerprint`
   - `Dipralix: Initialize` — runs `dipralix-cli --init`
   - `Dipralix: Run DIPRALIX: task on this line` — same as the CodeLens

## Requirements

`dipralix-cli` on your PATH. Install it with the one-liner from the [main README](../../README.md#quick-start). If you keep the binary somewhere unusual, set `dipralix.binaryPath` in VS Code settings.

## Build + install locally

```bash
cd ide/vscode
npm install
npm run compile

# Package and install
npx @vscode/vsce package           # produces dipralix-vscode-0.1.0.vsix
code --install-extension dipralix-vscode-0.1.0.vsix
```

Or open the `ide/vscode/` folder in VS Code and hit F5 to launch a debug instance with the extension loaded.

## Comment grammar

The CodeLens matches these forms (the matcher is language-agnostic):

```rust
// DIPRALIX: refactor this to use async/await
```
```python
# DIPRALIX: add proper error handling
```
```sql
-- DIPRALIX: index user_id for the lookup
```
```css
/* DIPRALIX: replace this color with the brand token */
```

Lines containing `DIPRALIX-DONE:` are skipped — that's the marker the agent rewrites them to when a task completes.
