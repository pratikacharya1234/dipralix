//! evolve.rs — Phase 5 self-evolution. Dipralix periodically researches how its
//! world is changing and folds what it learns back into persistent memory, so
//! running it over time (or 24/7) compounds instead of standing still. Each pass
//! is token-budgeted: a few tight queries, concrete findings only.

use crate::config::Config;

/// Topics Dipralix watches by default, biased toward the detected stack.
pub fn default_topics(dna: &crate::learning::ProjectDna) -> Vec<String> {
    let mut topics = vec![
        "AI coding agents and developer tooling".to_string(),
        "notable CVEs and security advisories this week".to_string(),
    ];
    match dna.language.as_str() {
        "rust" => topics.push("Rust language, crates, and ecosystem updates".into()),
        "typescript" | "javascript" => {
            topics.push("TypeScript / Node / frontend ecosystem updates".into())
        }
        "python" => topics.push("Python language and key library updates".into()),
        "go" => topics.push("Go language and ecosystem updates".into()),
        _ => {}
    }
    topics
}

/// Run one evolution pass: research each topic with a strict token budget and
/// store timestamped findings in global memory. Returns a short report.
pub async fn run(config: &Config, topics: &[String]) -> String {
    let mem = crate::memory::MemoryCore::new();
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut learned = 0usize;

    for topic in topics {
        let prompt = format!(
            "In 4 short bullet points, what genuinely changed recently about: {topic}? \
             Only concrete, current facts (versions, releases, advisories, benchmarks). \
             No filler, no preamble."
        );
        if let Ok(text) = crate::agent::run_jarvis_query(config, &prompt).await {
            let body = format!("_{}_\n\n{}", date, text.trim());
            if mem
                .record_pattern(&format!("evolve_{}", slug(topic)), &body)
                .is_ok()
            {
                learned += 1;
            }
        }
    }

    format!(
        "Evolution pass complete — refreshed {learned}/{} topic(s) into memory.",
        topics.len()
    )
}

/// Turn a free-text topic into a safe, compact memory key.
fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_clean() {
        assert_eq!(slug("Rust language & crates!"), "rust_language_crates");
        assert_eq!(slug("  AI / ML  "), "ai_ml");
    }

    #[test]
    fn default_topics_include_stack() {
        let dna = crate::learning::ProjectDna {
            language: "rust".to_string(),
            ..Default::default()
        };
        let topics = default_topics(&dna);
        assert!(topics.iter().any(|t| t.contains("Rust")));
        assert!(topics.len() >= 3);
    }
}
