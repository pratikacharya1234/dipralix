//! Plan Visualizer — terminal-native dependency graph + progress + risk badges.
//!
//! Reads a plan from `.dipralix/plans/current.md` (or any path under `.dipralix/plans/`).
//! Plan format is line-oriented markdown:
//!
//!   - [ ] 1. Task description                          (pending)
//!   - [~] 2. In progress task          [deps: 1]       (in_progress)
//!   - [x] 3. Done task                 [deps: 1,2]     (completed)
//!   - [!] 4. Blocked / failed task     [deps: 3]       (failed)
//!
//! Risk hints in the description trigger badges:
//!   "danger", "rm -rf", "drop", "force", "sudo", "destructive"  → Danger
//!   "deploy", "publish", "push", "migrate"                       → Review
//!   otherwise                                                    → Safe

use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Risk {
    Safe,
    Review,
    Danger,
}

#[derive(Clone, Debug)]
pub struct PlanTask {
    pub id: u32,
    pub description: String,
    pub status: TaskStatus,
    pub deps: Vec<u32>,
    pub risk: Risk,
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub source: Option<PathBuf>,
    pub tasks: Vec<PlanTask>,
}

impl Plan {
    pub fn parse(content: &str) -> Self {
        let mut tasks = Vec::new();
        for line in content.lines() {
            if let Some(task) = parse_line(line) {
                tasks.push(task);
            }
        }
        Self { source: None, tasks }
    }

    pub fn load() -> Option<Self> {
        let path = Path::new(".dipralix/plans/current.md");
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(path).ok()?;
        let mut plan = Self::parse(&content);
        plan.source = Some(path.to_path_buf());
        Some(plan)
    }
}

fn parse_status_prefix(trimmed: &str) -> Option<(TaskStatus, &str)> {
    if let Some(r) = trimmed.strip_prefix("- [ ]") { return Some((TaskStatus::Pending, r)); }
    if let Some(r) = trimmed.strip_prefix("- [~]") { return Some((TaskStatus::InProgress, r)); }
    if let Some(r) = trimmed.strip_prefix("- [x]") { return Some((TaskStatus::Completed, r)); }
    if let Some(r) = trimmed.strip_prefix("- [X]") { return Some((TaskStatus::Completed, r)); }
    if let Some(r) = trimmed.strip_prefix("- [!]") { return Some((TaskStatus::Failed, r)); }
    None
}

fn parse_line(line: &str) -> Option<PlanTask> {
    let trimmed = line.trim_start();
    let (status, rest) = parse_status_prefix(trimmed)?;

    let rest = rest.trim();
    let (id_str, after_id) = rest.split_once('.')?;
    let id: u32 = id_str.trim().parse().ok()?;

    // Extract optional [deps: 1,2,3]
    let (desc, deps) = if let Some(dstart) = after_id.find("[deps:") {
        let pre = &after_id[..dstart];
        let after = &after_id[dstart + "[deps:".len()..];
        let dend = after.find(']').unwrap_or(after.len());
        let deps_str = &after[..dend];
        let deps: Vec<u32> = deps_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        (pre.trim().to_string(), deps)
    } else {
        (after_id.trim().to_string(), Vec::new())
    };

    let risk = classify_risk(&desc);
    Some(PlanTask { id, description: desc, status, deps, risk })
}

fn classify_risk(desc: &str) -> Risk {
    let d = desc.to_lowercase();
    let danger = ["rm -rf", "drop table", "drop database", "force-push", "sudo", "destructive", "wipe"];
    if danger.iter().any(|p| d.contains(p)) { return Risk::Danger; }
    let review = ["deploy", "publish", "push", "migrate", "release", "ship"];
    if review.iter().any(|p| d.contains(p)) { return Risk::Review; }
    Risk::Safe
}

fn risk_badge(r: &Risk) -> colored::ColoredString {
    match r {
        Risk::Safe => "[safe]".green(),
        Risk::Review => "[review]".yellow(),
        Risk::Danger => "[danger]".red().bold(),
    }
}

fn status_glyph(s: &TaskStatus) -> colored::ColoredString {
    match s {
        TaskStatus::Pending    => "○".dimmed(),
        TaskStatus::InProgress => "◐".cyan(),
        TaskStatus::Completed  => "●".green(),
        TaskStatus::Failed     => "✗".red().bold(),
    }
}

fn progress_bar(plan: &Plan) -> String {
    let total = plan.tasks.len();
    if total == 0 { return String::new(); }
    let done = plan.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
    let pct = (done as f32 / total as f32 * 100.0) as u32;
    let filled = (done * 10) / total;
    let bar: String = (0..10).map(|i| if i < filled { '█' } else { '░' }).collect();
    format!("[{}] {}/{}  ({}%)", bar, done, total, pct)
}

pub fn view() {
    let Some(plan) = Plan::load() else {
        println!("\n  {} No plan found at {}",
            "[PLAN]".yellow(),
            ".dipralix/plans/current.md".dimmed());
        println!("  Create one with:\n    {}", "echo '- [ ] 1. First task' > .dipralix/plans/current.md".dimmed());
        return;
    };

    println!("\n  {} {}", "[PLAN]".cyan().bold(), plan.source.as_ref().map(|p| p.display().to_string()).unwrap_or_default().dimmed());
    println!("  Progress: {}", progress_bar(&plan));
    println!();

    for task in &plan.tasks {
        let deps_str = if task.deps.is_empty() {
            String::new()
        } else {
            format!(" ← deps: {}", task.deps.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(","))
        };
        println!("  {} {}. {}  {}{}",
            status_glyph(&task.status),
            task.id.to_string().bright_white(),
            task.description,
            risk_badge(&task.risk),
            deps_str.dimmed(),
        );
    }
    println!();
}

pub fn risk_report() {
    let Some(plan) = Plan::load() else {
        println!("\n  {} No plan loaded.", "[PLAN]".yellow());
        return;
    };

    let danger: Vec<_> = plan.tasks.iter().filter(|t| t.risk == Risk::Danger).collect();
    let review: Vec<_> = plan.tasks.iter().filter(|t| t.risk == Risk::Review).collect();

    println!("\n  {}", "Plan Risk Report".cyan().bold());
    println!("    {} danger    {} review    {} safe",
        danger.len().to_string().red().bold(),
        review.len().to_string().yellow(),
        plan.tasks.iter().filter(|t| t.risk == Risk::Safe).count().to_string().green());

    if !danger.is_empty() {
        println!("\n  {}", "Danger items:".red().bold());
        for t in &danger {
            println!("    {} {}", t.id.to_string().red(), t.description);
        }
    }
    if !review.is_empty() {
        println!("\n  {}", "Review items:".yellow().bold());
        for t in &review {
            println!("    {} {}", t.id.to_string().yellow(), t.description);
        }
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_task() {
        let plan = Plan::parse("- [ ] 1. Add OAuth2 endpoints");
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].id, 1);
        assert_eq!(plan.tasks[0].status, TaskStatus::Pending);
        assert_eq!(plan.tasks[0].risk, Risk::Safe);
    }

    #[test]
    fn parses_deps() {
        let plan = Plan::parse("- [~] 3. Wire middleware [deps: 1,2]");
        assert_eq!(plan.tasks[0].deps, vec![1, 2]);
        assert_eq!(plan.tasks[0].status, TaskStatus::InProgress);
    }

    #[test]
    fn classifies_danger() {
        let plan = Plan::parse("- [ ] 1. Run rm -rf node_modules");
        assert_eq!(plan.tasks[0].risk, Risk::Danger);
    }

    #[test]
    fn classifies_review() {
        let plan = Plan::parse("- [ ] 1. Deploy to staging");
        assert_eq!(plan.tasks[0].risk, Risk::Review);
    }
}
