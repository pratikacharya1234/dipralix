use anyhow::Result;
use colored::Colorize;

use crate::backend::BackendClient;
use crate::config::Config;
use crate::types::*;

/// Run a security sweep on the current working directory.
///
/// Steps:
/// 1. Run `cargo audit` if Cargo.lock is present.
/// 2. Run `npm audit` if package-lock.json is present.
/// 3. Use the active AI backend (with Google Search grounding) to review
///    findings and search for known CVEs relevant to the detected dependencies.
pub async fn sweep(config: &Config) -> Result<()> {
    println!();
    println!(
        "{}",
        "╔══════════════════════════════════════════════════╗".bright_red()
    );
    println!(
        "{}",
        "║  FORGE SecuritySweep                       ║".bright_red()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════╝".bright_red()
    );
    println!();

    let mut findings: Vec<String> = Vec::new();

    // ── cargo audit ───────────────────────────────────────────────────────────
    let cargo_lock = std::path::Path::new("Cargo.lock");
    if cargo_lock.exists() {
        println!("{} Running cargo audit...", "[BUSY]".bright_yellow());

        let out = tokio::process::Command::new("cargo")
            .args(["audit", "--color", "never"])
            .output()
            .await;

        match out {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();

                if !o.status.success() || stdout.contains("error[") {
                    println!("{} cargo audit found issues!", "[ERR]".red());
                    findings.push(format!("=== cargo audit ===\n{}{}", stdout, stderr));
                } else {
                    println!("{} cargo audit: no known vulnerabilities", "[OK]".green());
                }
            }
            Err(_) => {
                println!(
                    "  {} cargo-audit not installed. Install with: {}",
                    "[WARN]".yellow(),
                    "cargo install cargo-audit".cyan()
                );
                // Still proceed — Gemini can search for CVEs manually
                if let Ok(lock_content) = std::fs::read_to_string(cargo_lock) {
                    findings.push(format!(
                        "=== Cargo.lock ===\n{}",
                        &lock_content[..lock_content.len().min(8000)]
                    ));
                }
            }
        }
    }

    // ── npm audit ─────────────────────────────────────────────────────────────
    let npm_lock = std::path::Path::new("package-lock.json");
    if npm_lock.exists() {
        println!("{} Running npm audit...", "[BUSY]".bright_yellow());

        let out = tokio::process::Command::new("npm")
            .args(["audit", "--json"])
            .output()
            .await;

        match out {
            Ok(o) if !o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                println!("{} npm audit found issues!", "[ERR]".red());
                findings.push(format!(
                    "=== npm audit ===\n{}",
                    &stdout[..stdout.len().min(4000)]
                ));
            }
            Ok(_) => {
                println!("{} npm audit: no known vulnerabilities", "[OK]".green());
            }
            Err(_) => {
                println!("  {} npm not found in PATH", "[WARN]".yellow());
            }
        }
    }

    // ── Static code patterns ──────────────────────────────────────────────────
    println!(
        "{} Scanning for hardcoded secrets / risky patterns...",
        "[BUSY]".bright_yellow()
    );
    let patterns = static_scan();
    if !patterns.is_empty() {
        for p in &patterns {
            println!("  {} {}", "[WARN]".yellow(), p);
        }
        findings.push(format!(
            "=== Static scan findings ===\n{}",
            patterns.join("\n")
        ));
    } else {
        println!("{} No obvious hardcoded secrets found", "[OK]".green());
    }

    // ── AI grounded analysis ──────────────────────────────────────────────────
    println!();
    println!(
        "{} Searching for CVEs and security issues...",
        "[BUSY]".bright_yellow()
    );

    let prompt = if findings.is_empty() {
        "Perform a security audit of the current project. \
         Search the web for any known CVEs for common Rust/Node/Python dependencies. \
         Check OWASP Top 10 2025. \
         List any issues found, their severity, and recommended fixes. \
         If the project seems clean, confirm that."
            .to_string()
    } else {
        format!(
            "Security scan findings from this project:\n\n{}\n\n\
             For each finding:\n\
             1. Search the web for the CVE details and severity\n\
             2. Explain the risk in plain terms\n\
             3. Provide the exact fix command or code change\n\
             4. Check for any additional issues these might hint at.\n\
             Also run a general OWASP Top 10 2025 check on the codebase.",
            findings.join("\n\n")
        )
    };

    // Use grounding if the model supports it
    let mut tools = vec![];
    tools.push(serde_json::json!({ "googleSearch": {} }));

    let request = GenerateContentRequest {
        contents: vec![Content {
            role: "user".to_string(),
            parts: vec![Part::text(prompt)],
        }],
        tools,
        tool_config: None,
        system_instruction: Some(SystemContent {
            parts: vec![Part::text(
                "You are a senior security engineer. Be specific, actionable, and concise. \
                 Use real CVE numbers when available. Format output with clear severity labels: \
                 CRITICAL, HIGH, MEDIUM, LOW.",
            )],
        }),
        generation_config: Some(GenerationConfig {
            temperature: Some(0.3),
            max_output_tokens: Some(4096),
            thinking_config: None,
        }),
    };

    let client = match BackendClient::new(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\n{} Could not initialize backend: {}", "[ERR]".red(), e);
            return Ok(());
        }
    };

    println!();
    print!("{} ", "◆ Security Report".bright_red().bold());

    let mut first = true;
    let mut no_thought = |_: &str| {};
    let mut on_text = |chunk: &str| {
        if first {
            println!();
            first = false;
        }
        print!("{}", chunk);
        let _ = std::io::Write::flush(&mut std::io::stdout());
    };

    match client
        .generate_streaming(&request, &mut on_text, &mut no_thought)
        .await
    {
        Ok(resp) => {
            println!("\n");
            if let Some(usage) = resp.usage_metadata {
                if let Some(t) = usage.total_token_count {
                    println!("  {} {} tokens used", "◦".dimmed(), t.to_string().dimmed());
                }
            }
        }
        Err(e) => {
            eprintln!("\n{} AI analysis failed: {}", "[ERR]".red(), e);
            eprintln!("  Tip: run with --grounding or enable Google Search for CVE lookups.");
        }
    }

    // Save report
    let report_path = format!("security_audit_{}.md", chrono_now());
    if let Ok(full_report) = build_report(&findings) {
        let _ = std::fs::write(&report_path, full_report);
        println!("  {} Report saved: {}", "[OK]".green(), report_path.cyan());
    }

    Ok(())
}

// ── Static pattern scanner ─────────────────────────────────────────────────────

fn static_scan() -> Vec<String> {
    use std::io::BufRead;

    let patterns: &[(&str, &str)] = &[
        (
            r#"(?i)(api[_-]?key|secret|password|token)\s*=\s*['"][^'"]{8,}"#,
            "Possible hardcoded secret",
        ),
        (
            r"(?i)-----BEGIN (RSA |EC )?PRIVATE KEY-----",
            "Private key in source",
        ),
        (r"(?i)AIza[0-9A-Za-z\-_]{35}", "Google API key pattern"),
        (r"(?i)sk-[a-zA-Z0-9]{20,}", "OpenAI-style API key"),
        (r"(?i)ghp_[a-zA-Z0-9]{36}", "GitHub personal token"),
        (
            r"0\.0\.0\.0:80(?:[^0-9]|$)",
            "HTTP server bound to all interfaces",
        ),
    ];

    let skip_exts = ["lock", "sum", "png", "jpg", "pdf", "gz", "zip"];
    let skip_dirs = ["target", "node_modules", ".git", "packages"];
    // Documentation files that contain example keys
    let skip_files = [
        "README.md",
        "CHANGELOG.md",
        "LICENSE",
        "CONTRIBUTING.md",
        "install.sh",
        "release.sh",
    ];

    let mut findings = Vec::new();

    let compiled: Vec<(regex::Regex, &str)> = patterns
        .iter()
        .filter_map(|(pat, label)| regex::Regex::new(pat).ok().map(|r| (r, *label)))
        .collect();

    for entry in walkdir::WalkDir::new(".")
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let path_str = path.to_string_lossy();

        if skip_dirs
            .iter()
            .any(|d| path_str.contains(&format!("/{}/", d)))
        {
            continue;
        }
        if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
            if skip_files.contains(&fname) {
                continue;
            }
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if skip_exts.contains(&ext) {
            continue;
        }

        let Ok(file) = std::fs::File::open(path) else {
            continue;
        };
        let reader = std::io::BufReader::new(file);

        for (i, line) in reader.lines().enumerate() {
            let Ok(line) = line else { break };
            for (re, label) in &compiled {
                if re.is_match(&line) {
                    findings.push(format!("{}:{} — {}", path_str, i + 1, label));
                }
            }
        }
    }

    findings
}

fn build_report(findings: &[String]) -> Result<String> {
    let mut out = String::new();
    out.push_str(&format!("# FORGE Security Audit — {}\n\n", chrono_now()));
    if findings.is_empty() {
        out.push_str("No automated findings. See Gemini analysis output above.\n");
    } else {
        for f in findings {
            out.push_str(&format!("- {}\n", f));
        }
    }
    Ok(out)
}

fn chrono_now() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
