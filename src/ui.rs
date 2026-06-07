use colored::Colorize;

// ── Context bar ───────────────────────────────────────────────────────────────

pub fn print_context_bar(used_tokens: u32, window_size: u32) {
    if used_tokens == 0 || window_size == 0 {
        return;
    }
    let pct = (used_tokens as f32 / window_size as f32).min(1.0);
    let filled = (pct * 28.0) as usize;
    let empty = 28usize.saturating_sub(filled);
    let bar_f = "█".repeat(filled);
    let bar_e = "░".repeat(empty);
    let pct_s = format!("{:.0}%", pct * 100.0);
    let win_s = format!("{}M", window_size / 1_000_000);

    if pct >= 0.90 {
        println!(
            "  {}  {}{}  {}  [{}]",
            "CTX".bright_black(),
            bar_f.red(),
            bar_e.bright_black(),
            pct_s.red(),
            win_s.dimmed()
        );
    } else if pct >= 0.75 {
        println!(
            "  {}  {}{}  {}  [{}]",
            "CTX".bright_black(),
            bar_f.yellow(),
            bar_e.bright_black(),
            pct_s.yellow(),
            win_s.dimmed()
        );
    } else {
        println!(
            "  {}  {}{}  {}  [{}]",
            "CTX".bright_black(),
            bar_f.green(),
            bar_e.bright_black(),
            pct_s.green(),
            win_s.dimmed()
        );
    }
}

pub fn print_context_warning(pct: f32) {
    if pct >= 0.90 {
        println!(
            "\n  {}  Context at {:.0}% — run {} immediately",
            "▲ WARN".red().bold(),
            pct * 100.0,
            "/compact".yellow()
        );
    } else if pct >= 0.75 {
        println!(
            "\n  {}  Context at {:.0}% — consider {} soon",
            "▲".yellow(),
            pct * 100.0,
            "/compact".yellow()
        );
    }
}

// ── Help screen ───────────────────────────────────────────────────────────────

/// All three `live_*` slices are optional — pass `None` to fall back to built-in lists.
pub fn print_help(
    live_gemini: Option<&[(String, String)]>,
    live_claude: Option<&[(String, String)]>,
    live_openai: Option<&[(String, String)]>,
) {
    fn section(title: &str) {
        // "  ── TITLE ──...──" filling to col 80
        // 2 (indent) + 2 (──) + 1 (sp) + title + 1 (sp) + dashes = 80
        let dashes = "─".repeat(74usize.saturating_sub(title.len()));
        println!();
        println!(
            "  {} {} {}",
            "──".bright_blue(),
            title.bright_white().bold(),
            dashes.bright_blue()
        );
        println!();
    }

    section("COMMANDS");

    let cmds: &[(&str, &str)] = &[
        ("/quit  /exit  /q", "exit DIPRALIX"),
        ("/clear", "clear conversation history"),
        ("", ""),
        (
            "/model <name|auto|list|info>",
            "switch model, list all providers, or view info",
        ),
        ("/models", "fetch live model list from Gemini API"),
        ("/think [on|off|budget=N]", "toggle extended reasoning mode"),
        ("/web", "toggle Google Search grounding"),
        ("/apply [on|off]", "toggle auto-apply (skip diff review)"),
        (
            "/explain [on|off]",
            "preview planned actions before executing",
        ),
        ("", ""),
        (
            "/task <requirement>",
            "research → decompose → dispatch → consensus",
        ),
        ("/test-fix <cmd> [cycles]", "test → fix → retest loop"),
        ("/pr <title>", "push branch and open GitHub PR"),
        ("/compact", "summarize history to reclaim context"),
        ("/undo [N]", "revert last N file changes"),
        ("/snapshot [label]", "create git stash snapshot"),
        ("/rollback", "restore from last git stash"),
        ("/diff", "list pending file snapshots"),
        ("", ""),
        ("/memorize <fact>", "save fact to persistent memory"),
        ("/forget <keyword>", "remove matching memory entries"),
        ("/memory", "view all memorized facts"),
        (
            "/alive [nick|persona ...]",
            "view or set Dipralix's identity",
        ),
        ("/ledger [N]", "view the verified-outcome ledger"),
        ("/learnings", "view auto-learned error patterns"),
        ("/dna", "show auto-detected project conventions"),
        ("/resume", "show where we left off in this repo"),
        ("/evolve", "research industry changes into memory"),
        ("/connect", "show connected developer tools"),
        ("", ""),
        ("/load [path]", "load project files into context"),
        ("/learn <git-url>", "clone and load any OSS repo"),
        ("/tokens", "show context window usage"),
        ("/cost", "show session token costs and daily budget"),
        ("", ""),
        ("/session save|load|list|del", "manage named sessions"),
        ("/save [file]", "export conversation as Markdown"),
        ("/history [N]", "show last N turns"),
        ("", ""),
        ("/security", "security sweep with CVE scan"),
        ("/audit [N]", "show last N actions from audit log"),
        ("/profile <name>", "apply config profile"),
        ("/screenshot <path>", "vision analysis — find and fix bugs"),
        ("/debug", "toggle debug output"),
        ("/cd <dir>", "change working directory"),
        ("/help", "show this help"),
    ];

    for (cmd, desc) in cmds {
        if cmd.is_empty() {
            println!();
        } else {
            println!("  {:<42} {}", cmd.yellow(), desc.dimmed());
        }
    }

    section("MODELS");

    println!(
        "  {}  {}:",
        "▸".bright_blue(),
        "Gemini".bright_white().bold()
    );
    if let Some(models) = live_gemini {
        // Real-time list from the API
        for (name, desc) in models {
            println!(
                "      {:<34} {}",
                name.as_str().cyan(),
                desc.as_str().dimmed()
            );
        }
    } else {
        // Fallback when offline or key unavailable
        let gemini: &[(&str, &str)] = &[
            ("gemini-3.1-pro", "latest — 80.6% SWE-bench"),
            ("gemini-3-flash", "latest fast model"),
            ("gemini-2.5-pro", "deep reasoning, 1M context"),
            ("gemini-2.5-flash", "fast, recommended"),
            ("gemini-2.5-flash-lite", "cheapest — $0.10/M input"),
        ];
        for (m, d) in gemini {
            println!("      {:<34} {}", m.cyan(), d.dimmed());
        }
    }

    println!();
    println!(
        "  {}  {}:",
        "▸".bright_blue(),
        "Claude".bright_white().bold()
    );
    if let Some(models) = live_claude {
        for (name, desc) in models {
            println!(
                "      {:<34} {}",
                name.as_str().cyan(),
                desc.as_str().dimmed()
            );
        }
    } else {
        let claude: &[(&str, &str)] = &[
            ("claude-4-opus", "200K ctx  max reasoning"),
            ("claude-4-sonnet", "200K ctx  balanced"),
            ("claude-3.5-sonnet", "200K ctx  fast, capable"),
        ];
        for (m, d) in claude {
            println!("      {:<34} {}", m.cyan(), d.dimmed());
        }
    }

    println!();
    println!(
        "  {}  {}:",
        "▸".bright_blue(),
        "OpenAI".bright_white().bold()
    );
    if let Some(models) = live_openai {
        for (name, desc) in models {
            println!(
                "      {:<34} {}",
                name.as_str().cyan(),
                desc.as_str().dimmed()
            );
        }
    } else {
        let openai: &[(&str, &str)] = &[
            ("gpt-4.1", "1M ctx  strong code generation"),
            ("gpt-4o", "128K ctx  fast, multimodal"),
            ("o3", "200K ctx  advanced reasoning"),
            ("o4-mini", "200K ctx  reasoning, cost-efficient"),
        ];
        for (m, d) in openai {
            println!("      {:<34} {}", m.cyan(), d.dimmed());
        }
    }

    println!();
    println!(
        "  {}  {}",
        "Tip".dimmed(),
        "/model auto — route each task to the best available model".dimmed()
    );

    section("SAFETY");

    let levels: &[(&str, &str, &str)] = &[
        ("ALLOW  ", "green", "safe commands run silently"),
        ("WARN   ", "yellow", "side-effect commands print a warning"),
        (
            "CONFIRM",
            "bright_red",
            "destructive commands require confirmation",
        ),
        ("DENY   ", "red", "catastrophic commands blocked outright"),
    ];
    for (label, color, desc) in levels {
        let colored_label = match *color {
            "green" => label.green().bold().to_string(),
            "yellow" => label.yellow().bold().to_string(),
            "bright_red" => label.bright_red().bold().to_string(),
            _ => label.red().bold().to_string(),
        };
        println!("  {}   {}", colored_label, desc.dimmed());
    }

    section("TOOLS");

    println!("  {}", "File system:".bright_white());
    println!(
        "    {}",
        "read_file  write_file  edit_file  append_file  delete_file  copy_file  move_file".cyan()
    );
    println!();
    println!("  {}", "Search:".bright_white());
    println!(
        "    {}",
        "list_files  glob  search_files  create_directory".cyan()
    );
    println!();
    println!("  {}", "Shell:".bright_white());
    println!(
        "    {}",
        "bash  (streaming output, safety-checked, timeout up to 600s)".cyan()
    );
    println!();
    println!("  {}", "Web:".bright_white());
    println!(
        "    {}",
        "url_fetch  (HTML → plain text, 1-hour cache, 128KB max)".cyan()
    );
    println!();
    println!("  {}", "Git:".bright_white());
    println!(
        "    {}",
        "git_snapshot  (creates stash for /rollback)".cyan()
    );
    println!();
    println!(
        "  {}",
        "edit_file uses fuzzy (whitespace-agnostic) matching; occurrence=N for duplicates."
            .dimmed()
    );

    section("CONFIG");

    println!(
        "  {:<42} {}",
        "~/.dipralix/config.toml".cyan(),
        "global settings and API keys".dimmed()
    );
    println!(
        "  {:<42} {}",
        ".dipralix/project.md".cyan(),
        "per-project instructions (auto-loaded)".dimmed()
    );
    println!(
        "  {:<42} {}",
        ".dipralix/safety.toml".cyan(),
        "per-project permission policy overrides".dimmed()
    );
    println!(
        "  {:<42} {}",
        ".dipralix/memory.md".cyan(),
        "persistent facts written by /memorize".dimmed()
    );
    println!();
    println!(
        "  {}",
        "auto_apply = false          # ask before overwriting files".bright_black()
    );
    println!(
        "  {}",
        "max_iterations = 50         # pause when tool loop exceeds limit".bright_black()
    );
    println!(
        "  {}",
        "context_warn = 0.75         # warn when context reaches 75%".bright_black()
    );
    println!(
        "  {}",
        "daily_budget_usd = 5.00     # alert when session cost exceeds".bright_black()
    );
    println!();
    println!(
        "  {}",
        "╚══════════════════════════════════════════════════════════════════════════════╝"
            .bright_blue()
    );
    println!();
}
pub mod nullvoid;
