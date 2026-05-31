/// FORGE Auto-Learning System (ALICE)
///
/// DIPRALIX learns from its own mistakes across sessions.
/// When an error is detected during tool execution, the system:
/// 1. Analyzes the error pattern
/// 2. Extracts a learning
/// 3. Saves to .dipralix/learnings.md
/// 4. Auto-injects learnings into future system prompts
///
/// Unlike /memorize (manual), this is automatic and self-improving.
///
/// Additionally detects project conventions on startup:
/// - Coding style (tabs/spaces, quote style, indent width)
/// - Language versions and frameworks
/// - Test command patterns
/// - Build system preferences

use std::collections::HashMap;
use std::path::Path;

/// A learning extracted from a tool error or user interaction
#[derive(Debug, Clone)]
pub struct Learning {
    #[allow(dead_code)]
    pub pattern: String,    // What to detect (error message, context)
    pub lesson: String,     // What to learn
    pub category: String,   // "style", "convention", "dependency", "security", "test"
    pub count: u32,         // How many times encountered
}

/// Project DNA — auto-detected conventions and patterns
#[derive(Debug, Clone, Default)]
pub struct ProjectDna {
    pub language: String,
    pub build_command: String,
    pub test_command: String,
    pub lint_command: String,
    pub indent_style: String,      // "tabs" or "spaces"
    pub indent_width: usize,
    #[allow(dead_code)]
    pub quote_style: String,       // "single" or "double"
    pub semicolons: bool,
    #[allow(dead_code)]
    pub framework: String,
    pub conventions: Vec<String>,
}

impl ProjectDna {
    pub fn detect() -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut dna = ProjectDna::default();

        // Detect language
        if cwd.join("Cargo.toml").exists() {
            dna.language = "rust".into();
            dna.build_command = "cargo build".into();
            dna.test_command = "cargo test".into();
            dna.lint_command = "cargo clippy".into();
            dna.semicolons = true;
        } else if cwd.join("package.json").exists() {
            dna.language = "typescript".into();
            dna.build_command = "npm run build".into();
            dna.test_command = "npm test".into();
            dna.lint_command = "npm run lint".into();
        } else if cwd.join("go.mod").exists() {
            dna.language = "go".into();
            dna.build_command = "go build ./...".into();
            dna.test_command = "go test ./...".into();
            dna.lint_command = "go vet ./...".into();
        } else if cwd.join("requirements.txt").exists() || cwd.join("pyproject.toml").exists() {
            dna.language = "python".into();
            dna.build_command = "python -m compileall .".into();
            dna.test_command = "pytest".into();
        }

        // Detect indent style from existing code
        dna.detect_indent_style(&cwd);

        // Detect conventions
        dna.detect_conventions(&cwd);

        dna
    }

    fn detect_indent_style(&mut self, cwd: &Path) {
        // Sample the first source file to detect indentation
        let src_dir = if self.language == "rust" {
            cwd.join("src")
        } else {
            cwd.to_path_buf()
        };

        if !src_dir.exists() { return; }

        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() { continue; }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let tab_lines = content.lines().filter(|l| l.starts_with('\t')).count();
                    let space_lines = content.lines().filter(|l| l.starts_with("    ")).count();
                    let double_space = content.lines().filter(|l| l.starts_with("  ")).count();

                    if tab_lines > space_lines && tab_lines > double_space {
                        self.indent_style = "tabs".into();
                        self.indent_width = 1;
                    } else if space_lines > double_space {
                        self.indent_style = "spaces".into();
                        self.indent_width = 4;
                    } else if double_space > 0 {
                        self.indent_style = "spaces".into();
                        self.indent_width = 2;
                    }
                    break;
                }
            }
        }
    }

    fn detect_conventions(&mut self, cwd: &Path) {
        // Detect from existing config files
        if cwd.join(".eslintrc.json").exists() || cwd.join(".eslintrc.js").exists() {
            self.conventions.push("ESLint configured — respect rules".into());
        }
        if cwd.join(".prettierrc").exists() {
            self.conventions.push("Prettier configured — match formatting".into());
        }
        if cwd.join("rustfmt.toml").exists() {
            self.conventions.push("rustfmt configured — match formatting".into());
        }
        if cwd.join(".editorconfig").exists() {
            if let Ok(content) = std::fs::read_to_string(cwd.join(".editorconfig")) {
                for line in content.lines() {
                    if line.contains("indent_style") {
                        self.conventions.push(format!("EditorConfig: {}", line.trim()));
                    }
                }
            }
        }
        if cwd.join(".github/workflows").exists() {
            self.conventions.push("CI/CD pipeline configured".into());
        }
    }

    pub fn to_prompt_context(&self) -> String {
        if self.language.is_empty() {
            return String::new();
        }

        let mut ctx = String::from("\n## Project DNA (Auto-Detected)\n\n");
        ctx.push_str(&format!("- Language: {}\n", self.language));
        ctx.push_str(&format!("- Build: `{}`\n", self.build_command));
        ctx.push_str(&format!("- Test: `{}`\n", self.test_command));

        if !self.indent_style.is_empty() {
            ctx.push_str(&format!("- Indentation: {} (width: {})\n", self.indent_style, self.indent_width));
        }
        for convention in &self.conventions {
            ctx.push_str(&format!("- {}\n", convention));
        }

        ctx
    }
}

/// Load learnings from .dipralix/learnings.md
pub fn load_learnings() -> Vec<Learning> {
    let path = Path::new(".dipralix/learnings.md");
    if !path.exists() { return Vec::new(); }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    parse_learnings(&content)
}

/// Parse learnings from markdown content
fn parse_learnings(content: &str) -> Vec<Learning> {
    let mut learnings = Vec::new();
    let mut current_category = "general".to_string();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            current_category = trimmed[3..].to_lowercase();
        } else if trimmed.starts_with("- [") {
            // Format: - [count] pattern → lesson
            if let Some(rest) = trimmed.strip_prefix("- [") {
                if let Some(bracket_end) = rest.find(']') {
                    let count: u32 = rest[..bracket_end].parse().unwrap_or(1);
                    let rest = &rest[bracket_end + 1..].trim();
                    if let Some(arrow) = rest.find("->") {
                        let pattern = rest[..arrow].trim().to_string();
                        let lesson = rest[arrow + 2..].trim().to_string();
                        learnings.push(Learning {
                            pattern,
                            lesson,
                            category: current_category.clone(),
                            count,
                        });
                    }
                }
            }
        }
    }

    learnings
}

/// Record a new learning from a tool error
pub fn record_learning(error_output: &str, tool_name: &str, success_after: bool) {
    let path = Path::new(".dipralix/learnings.md");
    let mut content = std::fs::read_to_string(path).unwrap_or_default();

    // Analyze the error to extract a learning
    let (pattern, lesson, category) = analyze_error(error_output, tool_name);

    if pattern.is_empty() || lesson.is_empty() { return; }

    // Check if this pattern already exists — increment count
    if let Some(line) = content.lines().position(|l| l.contains(&pattern)) {
        // Update count in existing line
        let lines: Vec<&str> = content.lines().collect();
        let mut new_content = String::new();
        for (i, l) in lines.iter().enumerate() {
            if i == line && l.contains("- [") {
                if let Some(bracket_end) = l.find(']') {
                    let count_str = &l[3..bracket_end];
                    if let Ok(count) = count_str.parse::<u32>() {
                        let new_line = l.replace(&format!("[{}]", count), &format!("[{}]", count + 1));
                        new_content.push_str(&new_line);
                        new_content.push('\n');
                        continue;
                    }
                }
            }
            new_content.push_str(l);
            new_content.push('\n');
        }
        content = new_content;
    } else {
        // Add category header if needed
        if !content.contains(&format!("## {}", category)) {
            if !content.is_empty() { content.push('\n'); }
            content.push_str(&format!("## {}\n\n", category));
        }

        // Add learning
        let success_marker = if success_after { " [fixed]" } else { "" };
        content.push_str(&format!("- [1] {} -> {}{}\n", pattern, lesson, success_marker));
    }

    let _ = std::fs::write(path, &content);
}

/// Analyze a tool error and extract a learning
fn analyze_error(error: &str, tool_name: &str) -> (String, String, String) {
    let lower = error.to_lowercase();

    // Compilation errors
    if lower.contains("error[") || lower.contains("error:") || lower.contains("could not compile") {
        // Extract specific error type
        if lower.contains("cannot find") {
            return ("missing import/dependency".into(),
                format!("Verify all imports before running `{}` — missing dependencies cause cascading failures", tool_name),
                "compilation".into());
        }
        if lower.contains("borrow") || lower.contains("moved") {
            return ("Rust borrow/move error".into(),
                "Clone values before moving them into closures or async blocks".into(),
                "compilation".into());
        }
        if lower.contains("mismatched types") {
            return ("type mismatch".into(),
                "Check type signatures carefully — mismatched types are the most common compilation error".into(),
                "compilation".into());
        }
        return ("compilation error".into(),
            "Always run the build command after code changes to catch errors early".to_string(),
            "compilation".into());
    }

    // API errors
    if lower.contains("api key") || lower.contains("unauthorized") || lower.contains("403") || lower.contains("401") {
        return ("API authentication error".into(),
            "Verify API key is set correctly in environment or config file".into(),
            "configuration".into());
    }
    if lower.contains("rate limit") || lower.contains("429") || lower.contains("too many requests") {
        return ("API rate limit".into(),
            "API requests are being rate-limited — add delays between requests or use cheaper models".into(),
            "configuration".into());
    }

    // File path errors
    if lower.contains("no such file") || lower.contains("not found") {
        return ("file not found".into(),
            format!("Verify file paths before {}. Use list_files or glob to confirm paths exist.", tool_name),
            "filesystem".into());
    }

    // Shell/command errors
    if lower.contains("command not found") {
        return ("command not found".into(),
            "Check that the required CLI tool is installed before running shell commands".into(),
            "shell".into());
    }
    if lower.contains("permission denied") {
        return ("permission denied".into(),
            "Don't use sudo unless explicitly required — most operations don't need elevated permissions".into(),
            "shell".into());
    }

    // Network errors
    if lower.contains("connection") || lower.contains("timeout") || lower.contains("network") {
        return ("network error".into(),
            "Network issues detected — check connectivity and retry with longer timeouts".into(),
            "network".into());
    }

    // Generic fallback — only record if the error is substantial
    if error.len() > 50 {
        let pattern = error.chars().take(60).collect::<String>();
        return (pattern,
            format!("Error pattern detected during {} — investigate and fix root cause", tool_name),
            "general".into());
    }

    (String::new(), String::new(), String::new())
}

/// Convert learnings to a system prompt context section
pub fn learnings_to_context(learnings: &[Learning]) -> String {
    if learnings.is_empty() { return String::new(); }

    let mut ctx = String::from("\n## Auto-Learned Project Patterns\n\n");
    ctx.push_str("The following patterns have been automatically learned from previous sessions. Apply them.\n\n");

    // Group by category
    let mut by_category: HashMap<String, Vec<&Learning>> = HashMap::new();
    for l in learnings {
        by_category.entry(l.category.clone()).or_default().push(l);
    }

    for (category, items) in &by_category {
        ctx.push_str(&format!("### {}\n", capitalize(category)));
        for l in items {
            if l.count > 1 {
                ctx.push_str(&format!("- {} (encountered {} times)\n", l.lesson, l.count));
            } else {
                ctx.push_str(&format!("- {}\n", l.lesson));
            }
        }
        ctx.push('\n');
    }

    ctx
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
