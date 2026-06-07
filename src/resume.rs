//! resume.rs — Phase 3 continuity. On every start, Dipralix assembles a short
//! "welcome back" briefing from what it already knows about this repo — the
//! verified-outcome ledger, persistent memory, and prior sessions — so it
//! resumes one ongoing relationship instead of starting from zero, without
//! spending tokens to re-derive what it has already proved.

use colored::Colorize;

/// Build the welcome-back briefing, or `None` on a truly first run in this repo.
pub fn briefing(call_name: &str) -> Option<String> {
    let ledger = crate::ledger::Ledger::open();
    let all = ledger.all().unwrap_or_default();
    let last = all.last().map(|e| {
        (
            e.ts.get(..10).unwrap_or(e.ts.as_str()).to_string(),
            e.task.trim().to_string(),
        )
    });
    let sessions = crate::session::list_sessions().len();
    format_briefing(call_name, last, sessions, all.len())
}

/// Pure formatter (kept separate so it is testable without touching the disk).
fn format_briefing(
    call_name: &str,
    last: Option<(String, String)>,
    sessions: usize,
    outcomes: usize,
) -> Option<String> {
    if last.is_none() && sessions == 0 && outcomes == 0 {
        return None;
    }
    let mut out = format!("\n  welcome back — {call_name} here.\n");
    if let Some((date, task)) = last {
        out.push_str(&format!("  last in this repo [{date}]: {task}\n"));
    }
    let mut bits = Vec::new();
    if sessions > 0 {
        bits.push(format!("{sessions} saved session(s)"));
    }
    if outcomes > 0 {
        bits.push(format!("{outcomes} recorded outcome(s)"));
    }
    if !bits.is_empty() {
        out.push_str(&format!("  memory: {}\n", bits.join(", ")));
    }
    Some(out)
}

/// Print the briefing (dimmed) if there is one.
pub fn print_briefing(call_name: &str) {
    if let Some(b) = briefing(call_name) {
        print!("{}", b.dimmed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_history_yields_no_briefing() {
        assert!(format_briefing("Nova", None, 0, 0).is_none());
    }

    #[test]
    fn briefing_includes_last_task_and_counts() {
        let b = format_briefing(
            "Nova",
            Some(("2026-06-06".to_string(), "add rate limiting".to_string())),
            2,
            5,
        )
        .expect("should produce a briefing");
        assert!(b.contains("Nova"));
        assert!(b.contains("add rate limiting"));
        assert!(b.contains("2 saved session(s)"));
        assert!(b.contains("5 recorded outcome(s)"));
    }
}
