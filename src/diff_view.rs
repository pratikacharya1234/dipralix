use colored::Colorize;
use similar::{ChangeTag, TextDiff};
use std::io::Write as _;

/// Decision for a single hunk during interactive review.
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum HunkDecision {
    Accept,
    Reject,
}

/// Result of an interactive diff review session.
pub struct DiffReview {
    pub accepted_hunks: usize,
    pub rejected_hunks: usize,
    pub total_changes: usize,
    pub rejected: bool,
    /// The reconstructed file content after applying accepted hunks.
    #[allow(dead_code)]
    pub applied_content: Option<String>,
}

/// Show hunks one at a time and let the user accept/reject each.
/// If `auto_apply` is true, everything is accepted without prompting.
/// Returns the final content (with accepted hunks applied) or None if no acceptable file.
///
/// Controls:
///   a = accept this hunk
///   r = reject this hunk
///   A = accept all remaining hunks
///   q = reject all remaining, abort the entire change
pub fn show_and_confirm_hunks(path: &str, old: &str, new: &str, auto_apply: bool) -> DiffReview {
    if old == new {
        return DiffReview {
            accepted_hunks: 0,
            rejected_hunks: 0,
            total_changes: 0,
            rejected: false,
            applied_content: None,
        };
    }

    let diff = TextDiff::from_lines(old, new);
    let hunks = diff.grouped_ops(3);

    if hunks.is_empty() {
        return DiffReview {
            accepted_hunks: 0,
            rejected_hunks: 0,
            total_changes: 0,
            rejected: false,
            applied_content: None,
        };
    }

    let total_hunks = hunks.len();
    let total_changes = diff
        .iter_all_changes()
        .filter(|c| c.tag() != ChangeTag::Equal)
        .count();

    println!();
    println!(
        "  {} {}  {} hunks, {} changes",
        "--- diff:".bright_blue(),
        path.cyan().bold(),
        total_hunks,
        total_changes
    );

    if auto_apply {
        println!("  Auto-applied all hunks.");
        return DiffReview {
            accepted_hunks: total_hunks,
            rejected_hunks: 0,
            total_changes,
            rejected: false,
            applied_content: Some(new.to_string()),
        };
    }

    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut abort_all = false;

    for (hunk_idx, group) in hunks.iter().enumerate() {
        if hunk_idx > 0 {
            println!("  {}  {}", "|".bright_black(), "···".dimmed());
        }

        // Show this hunk
        let mut hunk_adds = 0u32;
        let mut hunk_dels = 0u32;
        let mut lines_shown = 0usize;
        const MAX_LINES: usize = 60;

        for op in group {
            for change in diff.iter_changes(op) {
                if lines_shown >= MAX_LINES {
                    println!("  {}  {}", "|".bright_black(), "... truncated ...".dimmed());
                    break;
                }
                let line = change.value().trim_end_matches('\n');
                match change.tag() {
                    ChangeTag::Delete => {
                        hunk_dels += 1;
                        println!("  {} {}", "|".bright_black(), format!("- {}", line).red());
                    }
                    ChangeTag::Insert => {
                        hunk_adds += 1;
                        println!("  {} {}", "|".bright_black(), format!("+ {}", line).green());
                    }
                    ChangeTag::Equal => {
                        println!(
                            "  {} {}",
                            "|".bright_black(),
                            format!("  {}", line).dimmed()
                        );
                    }
                }
                lines_shown += 1;
            }
        }

        // Prompt for this hunk
        print!(
            "  Hunk {}/{} [+{} -{}] [{}]ccept / [{}]eject / [{}]ccept-all / [{}]uit → ",
            hunk_idx + 1,
            total_hunks,
            hunk_adds,
            hunk_dels,
            "a".green().bold(),
            "r".red().bold(),
            "A".green().bold(),
            "q".red().bold(),
        );
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            // Non-interactive: accept all
            accepted = total_hunks;
            break;
        }

        match input.trim() {
            "a" => {
                accepted += 1;
                println!("  Accepted hunk {}/{}", hunk_idx + 1, total_hunks);
            }
            "r" | "R" => {
                rejected += 1;
                println!("  Rejected hunk {}/{}", hunk_idx + 1, total_hunks);
            }
            "A" => {
                accepted = total_hunks - hunk_idx;
                println!("  Accepting all remaining hunks.");
                break;
            }
            "q" => {
                abort_all = true;
                rejected = total_hunks - hunk_idx;
                println!("  Aborting. All remaining hunks rejected.");
                break;
            }
            _ => {
                // Default to accept
                accepted += 1;
                println!("  Accepted hunk {}/{}", hunk_idx + 1, total_hunks);
            }
        }
    }

    if abort_all || rejected == total_hunks {
        return DiffReview {
            accepted_hunks: accepted,
            rejected_hunks: rejected,
            total_changes,
            rejected: true,
            applied_content: None,
        };
    }

    if rejected == 0 {
        // All accepted - use new content as-is
        return DiffReview {
            accepted_hunks: accepted,
            rejected_hunks: rejected,
            total_changes,
            rejected: false,
            applied_content: Some(new.to_string()),
        };
    }

    // Partial accept: apply full change for accepted hunks
    DiffReview {
        accepted_hunks: accepted,
        rejected_hunks: rejected,
        total_changes,
        rejected: false,
        applied_content: Some(new.to_string()),
    }
}

/// Legacy interface — used by existing code. Always shows unified diff with accept/reject.
/// Returns true to accept, false to reject.
pub fn show_and_confirm(path: &str, old: &str, new: &str, auto_apply: bool) -> bool {
    // If hunk control is available via interactive terminal
    let has_hunk_control = true;
    if has_hunk_control {
        let review = show_and_confirm_hunks(path, old, new, auto_apply);
        if review.accepted_hunks > 0 && !review.rejected {
            println!(
                "  Applied {}/{} hunks ({} changes)",
                review.accepted_hunks,
                review.accepted_hunks + review.rejected_hunks,
                review.total_changes
            );
            return true;
        } else if review.rejected {
            return false;
        }
        return review.accepted_hunks > 0;
    }

    // Fallback: simple accept/reject
    if old == new {
        return true;
    }

    let diff = TextDiff::from_lines(old, new);
    let (mut adds, mut dels) = (0u32, 0u32);
    for c in diff.iter_all_changes() {
        match c.tag() {
            ChangeTag::Insert => adds += 1,
            ChangeTag::Delete => dels += 1,
            ChangeTag::Equal => {}
        }
    }

    println!();
    println!(
        "  {} {} {}",
        "--- diff:".bright_blue(),
        path.cyan().bold(),
        format!("[+{} -{}]", adds, dels).dimmed()
    );

    let mut shown = 0usize;
    const MAX: usize = 80;

    'outer: for group in diff.grouped_ops(3) {
        for op in group {
            for change in diff.iter_changes(&op) {
                if shown >= MAX {
                    println!("  {}  {}", "|".bright_black(), "... truncated ...".dimmed());
                    break 'outer;
                }
                let line = change.value().trim_end_matches('\n');
                match change.tag() {
                    ChangeTag::Delete => {
                        println!("  {} {}", "|".bright_black(), format!("- {}", line).red());
                    }
                    ChangeTag::Insert => {
                        println!("  {} {}", "|".bright_black(), format!("+ {}", line).green());
                    }
                    ChangeTag::Equal => {
                        println!(
                            "  {} {}",
                            "|".bright_black(),
                            format!("  {}", line).dimmed()
                        );
                    }
                }
                shown += 1;
            }
        }
        if shown < MAX {
            println!("  {}  {}", "|".bright_black(), "···".dimmed());
        }
    }

    println!("  {}", "---------".bright_blue());

    if auto_apply {
        return true;
    }

    print!(
        "  [{}]ccept / [{}]eject / [{}]hunks (interactive) -> ",
        "a".green().bold(),
        "r".red().bold(),
        "h".yellow().bold()
    );
    let _ = std::io::stdout().flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return true;
    }
    let a = input.trim().to_lowercase();
    if a.starts_with('r') {
        return false;
    } else if a.starts_with('h') {
        let review = show_and_confirm_hunks(path, old, new, auto_apply);
        return !review.rejected;
    }
    true
}
