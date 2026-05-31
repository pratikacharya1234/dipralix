# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

**Do not file a public issue for security vulnerabilities.** Send an email to **acharyapratik214@gmail.com** with:

- A description of the issue
- Steps to reproduce
- The affected component (binary, agent loop, MCP layer, integrations, etc.)
- Your assessment of impact

You'll get an acknowledgement within 72 hours. Patches for confirmed issues are landed on `main` and shipped in the next release. CVE assignment is offered for valid findings.

## Threat Model — What's In Scope

- Command injection through the `bash` tool (the 4-level safety classifier in `src/safety.rs` is the primary defense)
- Path traversal in file tools (`read_file`, `write_file`, `edit_file`, `delete_file`)
- Credential leakage from `.dipralix/`, `~/.dipralix/`, or audit logs
- MCP server escapes (parsing bugs in `src/mcp.rs`)
- Tampering with the `// DIPRALIX:` comment protocol to inject prompts

## Out of Scope

- Misuse by a user who has explicitly bypassed their own safety policy (`safety.toml` set to `allow` everywhere)
- Issues that only reproduce when running with a leaked or malicious API key
- Social engineering against the human at the keyboard

## Hardening Tips

- Pin `permissions.destructive_commands = "deny"` in `.dipralix/safety.toml`
- Set `bash_sudo = "Deny"` in `.dipralix/approval.toml`
- Review the `audit.log` in `.dipralix/` after long sessions
- Run untrusted projects under a non-privileged user account
