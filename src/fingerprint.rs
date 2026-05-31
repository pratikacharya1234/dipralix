//! Code Fingerprinting — `dipralix init`.
//!
//! Inspired by Caliber. Scans the current working directory, infers the tech
//! stack via ProjectDna, writes `.dipralix/project.md`, `.dipralix/safety.toml`,
//! `.dipralix/conventions.md`, and prints a quality score.

use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::learning::ProjectDna;

pub struct Fingerprint {
    pub dna: ProjectDna,
    pub files_scanned: usize,
    pub unwrap_count: usize,
    pub has_tests: bool,
    pub has_ci: bool,
    pub has_readme: bool,
    pub has_license: bool,
    pub has_contributing: bool,
}

impl Fingerprint {
    pub fn capture() -> Self {
        let dna = ProjectDna::detect();
        let mut fp = Fingerprint {
            dna,
            files_scanned: 0,
            unwrap_count: 0,
            has_tests: false,
            has_ci: Path::new(".github/workflows").exists() || Path::new(".gitlab-ci.yml").exists(),
            has_readme: Path::new("README.md").exists() || Path::new("readme.md").exists(),
            has_license: Path::new("LICENSE").exists() || Path::new("LICENSE.md").exists() || Path::new("LICENSE.txt").exists(),
            has_contributing: Path::new("CONTRIBUTING.md").exists(),
        };

        let root = std::env::current_dir().unwrap_or_default();
        let src = if fp.dna.language == "rust" { root.join("src") } else { root.clone() };
        if src.exists() {
            for entry in walkdir::WalkDir::new(&src).max_depth(8).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                let s = p.to_string_lossy();
                if s.contains("/target/") || s.contains("/node_modules/") || s.contains("/.") {
                    continue;
                }
                if !p.is_file() { continue; }
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go") { continue; }

                fp.files_scanned += 1;
                if let Ok(content) = fs::read_to_string(p) {
                    fp.unwrap_count += content.matches(".unwrap()").count();
                    if content.contains("#[test]") || content.contains("def test_") || content.contains("describe(") {
                        fp.has_tests = true;
                    }
                }
            }
        }

        fp
    }

    /// 0–100. Subtracts points for issues, adds points for hygiene.
    pub fn quality_score(&self) -> u32 {
        let mut score: i32 = 100;
        if !self.has_readme       { score -= 10; }
        if !self.has_license      { score -= 5; }
        if !self.has_ci           { score -= 10; }
        if !self.has_contributing { score -= 5; }
        if !self.has_tests        { score -= 15; }
        // Penalize unwraps in Rust (max -15)
        if self.dna.language == "rust" {
            let penalty = (self.unwrap_count as i32).min(15);
            score -= penalty;
        }
        score.clamp(0, 100) as u32
    }

    pub fn issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if !self.has_readme { issues.push("Missing README.md".into()); }
        if !self.has_license { issues.push("Missing LICENSE".into()); }
        if !self.has_ci { issues.push("No CI pipeline (.github/workflows or .gitlab-ci.yml)".into()); }
        if !self.has_contributing { issues.push("Missing CONTRIBUTING.md".into()); }
        if !self.has_tests { issues.push("No tests detected".into()); }
        if self.dna.language == "rust" && self.unwrap_count > 5 {
            issues.push(format!("{} `.unwrap()` calls — consider `?` or `expect(\"why\")`", self.unwrap_count));
        }
        issues
    }
}

pub fn run_fingerprint() {
    println!("\n  {} Capturing project fingerprint...", "[FP]".cyan());
    let fp = Fingerprint::capture();
    println!("  Files scanned: {}", fp.files_scanned.to_string().bright_white());
    println!("  Language:      {}", if fp.dna.language.is_empty() { "unknown".dimmed().to_string() } else { fp.dna.language.cyan().to_string() });
    println!("  Indent:        {} ({})", fp.dna.indent_style.cyan(), fp.dna.indent_width);
    println!("  Tests:         {}", if fp.has_tests { "yes".green() } else { "no".red() });
    println!("  CI:            {}", if fp.has_ci { "yes".green() } else { "no".red() });
    println!("  README:        {}", if fp.has_readme { "yes".green() } else { "no".red() });
    if fp.dna.language == "rust" {
        println!("  .unwrap() count: {}", fp.unwrap_count.to_string().yellow());
    }
    println!("\n  Quality Score: {}/100", fp.quality_score().to_string().bright_white().bold());
    let issues = fp.issues();
    if !issues.is_empty() {
        println!("\n  {}", "Suggestions:".yellow());
        for i in &issues {
            println!("    - {}", i);
        }
    }
    println!();
}

/// Generate the `.dipralix/` scaffolding from a captured fingerprint.
pub fn run_init() -> anyhow::Result<()> {
    println!("\n  {} Initializing Dipralix project...", "[INIT]".cyan());
    let fp = Fingerprint::capture();

    fs::create_dir_all(".dipralix")?;
    fs::create_dir_all(".dipralix/memory")?;
    fs::create_dir_all(".dipralix/skills")?;
    fs::create_dir_all(".dipralix/plans")?;

    // project.md
    let project_md = render_project_md(&fp);
    write_if_missing(".dipralix/project.md", &project_md)?;

    // conventions.md
    let conventions_md = render_conventions_md(&fp);
    write_if_missing(".dipralix/conventions.md", &conventions_md)?;

    // safety.toml — minimal default
    let safety_toml = render_safety_toml();
    write_if_missing(".dipralix/safety.toml", &safety_toml)?;

    // approval.toml — defaults from approval matrix
    let approval_toml = render_approval_toml();
    write_if_missing(".dipralix/approval.toml", &approval_toml)?;

    println!("  {} Wrote .dipralix/project.md", "[OK]".green());
    println!("  {} Wrote .dipralix/conventions.md", "[OK]".green());
    println!("  {} Wrote .dipralix/safety.toml", "[OK]".green());
    println!("  {} Wrote .dipralix/approval.toml", "[OK]".green());
    println!("\n  Quality Score: {}/100\n", fp.quality_score().to_string().bright_white().bold());

    Ok(())
}

fn write_if_missing(path: &str, content: &str) -> anyhow::Result<()> {
    if Path::new(path).exists() {
        println!("  {} skipping {} (already exists)", "[--]".dimmed(), path.dimmed());
        return Ok(());
    }
    fs::write(path, content)?;
    Ok(())
}

fn render_project_md(fp: &Fingerprint) -> String {
    let mut s = String::from("# Project DNA\n\n");
    s.push_str("Generated by `dipralix init`. Edit freely — this file is git-tracked.\n\n");
    s.push_str("## Stack\n\n");
    s.push_str(&format!("- Language: **{}**\n", if fp.dna.language.is_empty() { "unknown" } else { &fp.dna.language }));
    s.push_str(&format!("- Build:    `{}`\n", fp.dna.build_command));
    s.push_str(&format!("- Test:     `{}`\n", fp.dna.test_command));
    s.push_str(&format!("- Lint:     `{}`\n", fp.dna.lint_command));
    s.push_str("\n## Style\n\n");
    s.push_str(&format!("- Indent: {} × {}\n", fp.dna.indent_style, fp.dna.indent_width));
    s.push_str(&format!("- Semicolons: {}\n", fp.dna.semicolons));
    s.push_str("\n## Quality Snapshot\n\n");
    s.push_str(&format!("- Files scanned: {}\n", fp.files_scanned));
    s.push_str(&format!("- Tests present: {}\n", fp.has_tests));
    s.push_str(&format!("- CI configured: {}\n", fp.has_ci));
    s.push_str(&format!("- README present: {}\n", fp.has_readme));
    s.push_str(&format!("- LICENSE present: {}\n", fp.has_license));
    s.push_str(&format!("- Quality score: {}/100\n", fp.quality_score()));
    s
}

fn render_conventions_md(fp: &Fingerprint) -> String {
    let mut s = String::from("# Coding Conventions\n\n");
    s.push_str("Auto-detected from the workspace.\n\n");
    if fp.dna.conventions.is_empty() {
        s.push_str("- (none detected — add manual rules below)\n");
    } else {
        for c in &fp.dna.conventions {
            s.push_str(&format!("- {}\n", c));
        }
    }
    s.push_str("\n## Manual rules\n\n- Edit this section freely; Dipralix loads it as context.\n");
    s
}

fn render_safety_toml() -> String {
    r#"# Dipralix safety policy — generated by `dipralix init`.
# Levels: allow, warn, confirm, deny.

[permissions]
destructive_commands = "confirm"
network_commands     = "warn"
git_destructive       = "confirm"
sudo_commands        = "deny"
publish_commands     = "confirm"

[trusted_commands]
allow = [
  "cargo check",
  "cargo build",
  "cargo test",
  "cargo clippy",
  "git status",
  "git diff",
  "git log",
]

[blocked_commands]
deny = [
  "rm -rf /",
  "mkfs",
]
"#.to_string()
}

fn render_approval_toml() -> String {
    r#"# Dipralix per-action approval matrix.
# Levels: Auto, Notify, Confirm, Deny.

[actions]
read_file       = "Auto"
write_file      = "Notify"
edit_file       = "Confirm"
bash            = "Confirm"
bash_rm         = "Deny"
bash_git_push   = "Deny"
bash_docker_run = "Confirm"
bash_curl       = "Confirm"
bash_sudo       = "Deny"
"#.to_string()
}
