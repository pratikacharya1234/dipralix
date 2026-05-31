use colored::Colorize;
use std::sync::{OnceLock, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RiskLevel {
    Allow,
    Warn,
    Confirm,
    Deny,
}

// ── Per-project safety.toml support ────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct SafetyPolicy {
    pub destructive_commands: RiskLevelOverride,
    pub network_commands: RiskLevelOverride,
    pub git_destructive: RiskLevelOverride,
    pub sudo_commands: RiskLevelOverride,
    pub publish_commands: RiskLevelOverride,
    pub allowed_commands: Vec<String>,
    pub blocked_commands: Vec<String>,
}

#[derive(Clone, Debug)]
#[derive(Default)]
pub enum RiskLevelOverride {
    #[default]
    Unset,
    Override(RiskLevel),
}


static SAFETY_POLICY: OnceLock<Mutex<SafetyPolicy>> = OnceLock::new();

fn policy() -> &'static Mutex<SafetyPolicy> {
    SAFETY_POLICY.get_or_init(|| {
        let policy = load_safety_toml();
        if !policy.is_default() {
            eprintln!("  {} Loaded .dipralix/safety.toml", "[OK]".green());
        }
        Mutex::new(policy)
    })
}

fn load_safety_toml() -> SafetyPolicy {
    let path = std::path::Path::new(".dipralix/safety.toml");
    let Ok(content) = std::fs::read_to_string(path) else { return SafetyPolicy::default() };
    #[derive(serde::Deserialize)]
    struct Raw {
        permissions: Option<PermissionsRaw>,
        #[serde(rename = "trusted_commands")]
        trusted_commands: Option<TrustedRaw>,
        #[serde(rename = "blocked_commands")]
        blocked_commands: Option<BlockedRaw>,
    }
    #[derive(serde::Deserialize)]
    struct PermissionsRaw {
        destructive_commands: Option<String>,
        network_commands: Option<String>,
        git_destructive: Option<String>,
        sudo_commands: Option<String>,
        publish_commands: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct TrustedRaw {
        allow: Option<Vec<String>>,
    }
    #[derive(serde::Deserialize)]
    struct BlockedRaw {
        deny: Option<Vec<String>>,
    }

    let raw: Raw = match toml::from_str(&content) {
        Ok(r) => r,
        Err(_) => return SafetyPolicy::default(),
    };

    let mut policy = SafetyPolicy::default();

    fn parse_level(s: Option<&str>) -> RiskLevelOverride {
        match s {
            Some("deny")    => RiskLevelOverride::Override(RiskLevel::Deny),
            Some("confirm") => RiskLevelOverride::Override(RiskLevel::Confirm),
            Some("warn")    => RiskLevelOverride::Override(RiskLevel::Warn),
            Some("allow")   => RiskLevelOverride::Override(RiskLevel::Allow),
            _                => RiskLevelOverride::Unset,
        }
    }

    if let Some(p) = raw.permissions {
        policy.destructive_commands = parse_level(p.destructive_commands.as_deref());
        policy.network_commands     = parse_level(p.network_commands.as_deref());
        policy.git_destructive       = parse_level(p.git_destructive.as_deref());
        policy.sudo_commands        = parse_level(p.sudo_commands.as_deref());
        policy.publish_commands     = parse_level(p.publish_commands.as_deref());
    }

    if let Some(t) = raw.trusted_commands {
        if let Some(allow) = t.allow {
            policy.allowed_commands = allow;
        }
    }

    if let Some(b) = raw.blocked_commands {
        if let Some(deny) = b.deny {
            policy.blocked_commands = deny;
        }
    }

    policy
}

impl SafetyPolicy {
    fn is_default(&self) -> bool {
        self.allowed_commands.is_empty()
            && self.blocked_commands.is_empty()
            && matches!(self.destructive_commands, RiskLevelOverride::Unset)
            && matches!(self.network_commands, RiskLevelOverride::Unset)
            && matches!(self.git_destructive, RiskLevelOverride::Unset)
            && matches!(self.sudo_commands, RiskLevelOverride::Unset)
            && matches!(self.publish_commands, RiskLevelOverride::Unset)
    }
}

pub fn classify(cmd: &str) -> RiskLevel {
    let c = cmd.trim().to_lowercase();
    let p = policy().lock().unwrap();

    // 1. Check blocked commands first
    for blocked in &p.blocked_commands {
        if c.contains(&blocked.to_lowercase()) {
            return RiskLevel::Deny;
        }
    }

    // 2. Check allowed commands (bypass all other checks)
    for allowed in &p.allowed_commands {
        if c.contains(&allowed.to_lowercase()) {
            return RiskLevel::Allow;
        }
    }
    drop(p); // Release lock before IO

    // Pipe-to-shell: always deny
    if (c.contains("| sh") || c.contains("| bash") || c.contains("| zsh"))
        && (c.contains("curl ") || c.contains("wget "))
    {
        return RiskLevel::Deny;
    }

    const DENY: &[&str] = &[
        "rm -rf /",
        "mkfs",
        "dd if=/dev/zero",
        "dd if=/dev/null of=/dev/",
        "> /dev/sda",
        ":(){ :|:& };:",
        "chmod 000 /",
        "shred /dev/",
    ];
    for p in DENY {
        if c.contains(p) {
            return RiskLevel::Deny;
        }
    }

    // Check policy for destructive
    {
        let p = policy().lock().unwrap();
        if let RiskLevelOverride::Override(level) = p.destructive_commands {
            return classify_with_override(c, level, &[
                "rm -rf", "rm -r ", "rm -fr ",
            ]);
        }
        if let RiskLevelOverride::Override(level) = p.sudo_commands {
            if c.contains("sudo ") {
                return level;
            }
        }
    }

    const CONFIRM: &[&str] = &[
        "rm -rf",
        "rm -r ",
        "rm -fr ",
        "git reset --hard",
        "git push --force",
        "git push -f ",
        "git clean -f",
        "drop database",
        "drop table",
        "truncate table",
        "delete from ",
        "sudo ",
        "cargo publish",
        "npm publish",
        "yarn publish",
        "chmod 777",
        "chown -r",
    ];

    {
        let p = policy().lock().unwrap();
        if let RiskLevelOverride::Override(level) = p.git_destructive {
            return classify_with_override(c, level, &[
                "git reset --hard", "git push --force", "git push -f ", "git clean -f",
            ]);
        }
        if let RiskLevelOverride::Override(level) = p.publish_commands {
            return classify_with_override(c, level, &[
                "cargo publish", "npm publish", "yarn publish",
            ]);
        }
        if let RiskLevelOverride::Override(level) = p.network_commands {
            return classify_with_override(c, level, &[
                "curl ", "wget ", "npm install", "pip install",
            ]);
        }
    }

    for p in CONFIRM {
        if c.contains(p) {
            return RiskLevel::Confirm;
        }
    }

    const WARN: &[&str] = &[
        "git push",
        "git stash drop",
        "git branch -D",
        "git tag -d",
        "npm install -g",
        "apt install",
        "apt-get install",
        "brew install",
        "pip install",
        "systemctl",
        "killall",
        "pkill",
        "kill -9",
    ];
    for p in WARN {
        if c.contains(p) {
            return RiskLevel::Warn;
        }
    }

    RiskLevel::Allow
}

fn classify_with_override(c: String, level: RiskLevel, triggers: &[&str]) -> RiskLevel {
    for t in triggers {
        if c.contains(t) {
            return level;
        }
    }
    RiskLevel::Allow // let normal classification handle non-trigger commands
}

/// Check a bash command. Returns true if execution should proceed.
/// Must be called from sync context (or via block_in_place from async).
pub fn check_bash(cmd: &str) -> bool {
    match classify(cmd) {
        RiskLevel::Allow => true,
        RiskLevel::Warn => {
            println!(
                "  {} {}",
                "[WARN] side-effects:".yellow(),
                cmd.chars().take(80).collect::<String>().yellow().dimmed()
            );
            true
        }
        RiskLevel::Confirm => {
            println!();
            println!("  {} Potentially destructive command:", "[WARN]".yellow().bold());
            println!("  {}  {}", "│".dimmed(), cmd.yellow());
            prompt_yn()
        }
        RiskLevel::Deny => {
            println!(
                "  {} {}",
                "ERR  blocked (safety):".red().bold(),
                cmd.chars().take(80).collect::<String>().red()
            );
            false
        }
    }
}

/// Check a delete operation. Always requires confirmation.
pub fn check_delete(path: &str) -> bool {
    println!();
    println!("  {} delete: {}", "[WARN]".yellow().bold(), path.yellow());
    prompt_yn()
}

fn prompt_yn() -> bool {
    use std::io::Write as _;
    print!("  Proceed? [y/N] ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    let a = line.trim().to_lowercase();
    if a == "y" || a == "yes" {
        println!("  {}", "OK allowed".green());
        true
    } else {
        println!("  {}", "ERR cancelled".red());
        false
    }
}
