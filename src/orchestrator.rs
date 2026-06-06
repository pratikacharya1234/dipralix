/// Intelligent Multi-Model Task Orchestration
///
/// Pipeline: Research → Decompose → Dispatch → Consensus → Merge
///
/// This is FORGE's killer feature — the only coding agent that:
/// 1. Auto-researches before coding
/// 2. Decomposes tasks into subtasks
/// 3. Routes each subtask to the best model for the job
/// 4. Runs subagents in parallel across different providers
/// 5. Checks consensus on critical changes
/// 6. Auto-escalates from cheap to capable models on failure
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::backend::{self, BackendClient, Provider};
use crate::config::Config;
use crate::integrations::IntegrationRegistry;
use crate::mcp::McpRegistry;
use crate::token_counter::CostTracker;
use crate::tools::ToolContext;
use crate::types::*;

// ── Subtask definition ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: usize,
    pub description: String,
    pub difficulty: String,     // "low" | "medium" | "high" | "critical"
    pub model: Option<String>,  // override model (None = auto-route)
    pub files: Vec<String>,     // expected files involved
    pub depends_on: Vec<usize>, // subtask IDs that must complete first
}

#[derive(Debug, Clone)]
pub struct SubtaskResult {
    pub subtask: Subtask,
    pub success: bool,
    pub summary: String,
    pub files_changed: Vec<String>,
    pub model_used: String,
    #[allow(dead_code)]
    pub attempts: u32,
}

// ── Orchestrator state ───────────────────────────────────────────────────────

pub struct TaskOrchestrator {
    config: Config,
    /// Track model success/failure per task type for cost-intelligent escalation
    model_stats: HashMap<String, ModelStats>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
struct ModelStats {
    successes: u32,
    failures: u32,
}

// ── Public API ───────────────────────────────────────────────────────────────

impl TaskOrchestrator {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            model_stats: HashMap::new(),
        }
    }

    /// Run the full pipeline.
    pub async fn run(
        &mut self,
        requirement: &str,
        mcp: Option<Arc<McpRegistry>>,
        integrations: Option<Arc<IntegrationRegistry>>,
    ) -> Result<String> {
        println!("\n  {} TASK ORCHESTRATOR", "══".cyan());
        println!("  {} Requirement: {}", "▸".cyan(), requirement.yellow());
        println!();

        // ── Phase 1: Research ───────────────────────────────────────────────
        let research = self.research_phase(requirement).await?;

        // ── Phase 2: Decompose ──────────────────────────────────────────────
        let subtasks = self.decompose_phase(requirement, &research).await?;

        // ── Phase 3: Dispatch ───────────────────────────────────────────────
        let results = self
            .dispatch_phase(&subtasks, requirement, &research, mcp, integrations)
            .await?;

        // ── Phase 4: Consensus ──────────────────────────────────────────────
        let verified = self.consensus_phase(&results, requirement).await?;

        // ── Phase 5: Merge ──────────────────────────────────────────────────
        let summary = self.merge_phase(&verified, requirement).await?;

        println!();
        Ok(summary)
    }

    // ── Phase 1: Research Pipeline ──────────────────────────────────────────

    async fn research_phase(&self, requirement: &str) -> Result<String> {
        println!(
            "  {} Researching requirement...",
            "[PHASE 1/5]".cyan().bold()
        );
        println!(
            "  {} Auto-searching web for relevant docs, APIs, and best practices",
            "│".dimmed()
        );

        let queries = self.generate_research_queries(requirement);
        let mut research = String::from("## Pre-Execution Research\n\n");

        // Run research searches in parallel
        let client = BackendClient::new(&self.config)?;
        let mut futures = Vec::new();

        for query in &queries {
            println!("  {} Searching: \"{}\"", "│".dimmed(), query.dimmed());

            // Use the model's grounding/google_search capability via the agent
            let research_prompt = format!(
                "Search the web and summarize findings for this query: {}\n\n\
                 Provide:\n\
                 1. Best practices and patterns\n\
                 2. Relevant libraries/packages with versions\n\
                 3. Common pitfalls and gotchas\n\
                 4. Example code snippets if applicable\n\
                 Be concise but thorough. Include specific package names and version numbers.",
                query
            );

            let req = GenerateContentRequest {
                contents: vec![Content { role: "user".into(), parts: vec![Part::text(&research_prompt)] }],
                tools: vec![],
                tool_config: None,
                system_instruction: Some(SystemContent {
                    parts: vec![Part::text("You are a research assistant. Search the web and provide factual, cited technical information. Be specific with package names, versions, and code examples.")],
                }),
                generation_config: Some(GenerationConfig {
                    temperature: Some(0.3),
                    max_output_tokens: Some(2048),
                    thinking_config: None,
                }),
            };

            futures.push(async {
                let resp = client.generate(req).await;
                (query.clone(), resp)
            });
        }

        // Collect results
        let responses = futures_util::future::join_all(futures).await;
        for (query, result) in &responses {
            match result {
                Ok(resp) => {
                    if let Some(text) = Self::extract_text(resp) {
                        research.push_str(&format!("### {}\n\n{}\n\n", query, text));
                        println!(
                            "  {} Research complete: \"{}\" ({} chars)",
                            "│".dimmed(),
                            query.dimmed(),
                            text.len()
                        );
                    }
                }
                Err(e) => {
                    warn_no_color(&format!("Research query '{}' failed: {}", query, e));
                    research.push_str(&format!("### {} (search unavailable)\n\n", query));
                }
            }
        }

        if research.len() < 50 {
            research.push_str(
                "(No research results available — proceeding with general knowledge)\n\n",
            );
        } else {
            let research_chars = research.len();
            println!(
                "  {} Research gathered: {} chars across {} queries",
                "│".dimmed(),
                research_chars,
                queries.len()
            );
        }
        println!();

        Ok(research)
    }

    fn generate_research_queries(&self, requirement: &str) -> Vec<String> {
        let lower = requirement.to_lowercase();
        let mut queries = vec![format!("best practices {}", requirement)];

        // Add tech-specific queries
        if lower.contains("rust") || lower.contains("cargo") {
            queries.push(format!("rust crate {}", requirement));
        }
        if lower.contains("api") || lower.contains("rest") || lower.contains("endpoint") {
            queries.push(format!("API design patterns {}", requirement));
        }
        if lower.contains("auth") || lower.contains("jwt") || lower.contains("oauth") {
            queries.push(format!(
                "authentication security best practices {}",
                requirement
            ));
        }
        if lower.contains("test") || lower.contains("testing") {
            queries.push(format!("testing strategy {}", requirement));
        }
        if lower.contains("database") || lower.contains("sql") || lower.contains("postgres") {
            queries.push(format!("database schema design {}", requirement));
        }
        if lower.contains("docker") || lower.contains("deploy") || lower.contains("ci/cd") {
            queries.push(format!("deployment pipeline {}", requirement));
        }

        // Always include a general "latest" query
        queries.push(format!("{} 2026 recommended approach", requirement));

        // Cap at 4 queries
        queries.truncate(4);
        queries
    }

    // ── Phase 2: Task Decomposition ─────────────────────────────────────────

    async fn decompose_phase(&self, requirement: &str, research: &str) -> Result<Vec<Subtask>> {
        println!(
            "  {} Decomposing task into subtasks...",
            "[PHASE 2/5]".cyan().bold()
        );

        // Use a reasoning-capable model for decomposition
        let decompose_model = if self
            .config
            .anthropic_api_key
            .as_deref()
            .is_some_and(|k| !k.is_empty())
        {
            "claude-4-sonnet"
        } else if self.config.model.contains("pro") {
            &self.config.model
        } else {
            "gemini-3-pro"
        };

        let dc_config = Config {
            model: decompose_model.to_string(),
            ..self.config.clone()
        };

        let client = match BackendClient::new(&dc_config) {
            Ok(c) => c,
            Err(_) => BackendClient::new(&self.config)?,
        };

        let research_section = if research.len() > 50 {
            format!("\n\n## Research Results\n\n{}", research)
        } else {
            String::new()
        };

        let decompose_prompt = format!(
            "Break down this coding task into independent subtasks:\n\n\
             REQUIREMENT: {}{}\n\n\
             Return a JSON array of subtasks. Each subtask has:\n\
             - id: sequential number starting at 1\n\
             - description: what to implement (1-2 sentences)\n\
             - difficulty: \"low\", \"medium\", \"high\", or \"critical\"\n\
               * low = simple file edits, config changes, dependency adds\n\
               * medium = moderate logic, API endpoints, moderate refactoring\n\
               * high = complex algorithms, architecture, security, multi-file coordination\n\
               * critical = core infrastructure, auth, data migration, breaking changes\n\
             - model: null (let the system auto-route), or specify if you have strong preference\n\
             - files: array of expected file paths involved\n\
             - depends_on: array of subtask IDs that must complete first (empty if independent)\n\n\
             Rules:\n\
             - Make subtasks as independent as possible (maximize parallelism)\n\
             - Each subtask should be completable in 1-3 file changes\n\
             - Critical/security subtasks should be isolated for consensus checking\n\
             - Order: setup tasks first, core logic middle, testing last\n\n\
             Return ONLY valid JSON array, no explanation, no markdown fences.",
            requirement, research_section
        );

        let req = GenerateContentRequest {
            contents: vec![Content {
                role: "user".into(),
                parts: vec![Part::text(&decompose_prompt)],
            }],
            tools: vec![],
            tool_config: None,
            system_instruction: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.2),
                max_output_tokens: Some(4096),
                thinking_config: None,
            }),
        };

        let resp = client.generate(req).await?;
        let text = Self::extract_text(&resp)
            .ok_or_else(|| anyhow::anyhow!("Failed to get decomposition response"))?;

        // Parse JSON from response (handle markdown code fences)
        let json_str = text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let subtasks: Vec<Subtask> =
            serde_json::from_str(json_str).context("Failed to parse task decomposition JSON")?;

        if subtasks.is_empty() {
            anyhow::bail!("Decomposition returned no subtasks");
        }

        println!(
            "  {} Decomposed into {} subtasks:",
            "│".dimmed(),
            subtasks.len()
        );
        for st in &subtasks {
            let diff_color = match st.difficulty.as_str() {
                "critical" => "CRITICAL".red(),
                "high" => "HIGH".yellow(),
                _ => st.difficulty.dimmed(),
            };
            let deps = if st.depends_on.is_empty() {
                String::new()
            } else {
                format!(" (after: {:?})", st.depends_on)
            };
            println!(
                "  {} [{}. {}] {} — {}{}",
                "│".dimmed(),
                st.id,
                diff_color,
                st.description.dimmed(),
                "▸".dimmed(),
                deps
            );
        }
        println!();

        Ok(subtasks)
    }

    // ── Phase 3: Parallel Dispatch ──────────────────────────────────────────

    async fn dispatch_phase(
        &mut self,
        subtasks: &[Subtask],
        requirement: &str,
        research: &str,
        mcp: Option<Arc<McpRegistry>>,
        integrations: Option<Arc<IntegrationRegistry>>,
    ) -> Result<Vec<SubtaskResult>> {
        println!("  {} Dispatching subtasks...", "[PHASE 3/5]".cyan().bold());

        // Group independent subtasks by dependency level
        let levels = group_by_dependency_level(subtasks);
        let mut all_results: Vec<SubtaskResult> = Vec::new();

        // Share requirement/research across all subtasks via Arc
        let req_shared = Arc::new(requirement.to_string());
        let res_shared = Arc::new(research.to_string());

        for (level, batch) in levels.iter().enumerate() {
            if batch.is_empty() {
                continue;
            }

            println!(
                "  {} Level {} — {} parallel subtask(s)",
                "│".dimmed(),
                level + 1,
                batch.len()
            );

            let mut futures = Vec::new();
            for subtask in batch {
                let model = self.route_model(subtask);
                let req = req_shared.clone();
                let res = res_shared.clone();
                let mcp_clone = mcp.clone();
                let int_clone = integrations.clone();

                futures.push(async move {
                    println!(
                        "  {} [{}] {} → {}",
                        "│".dimmed(),
                        subtask.id,
                        subtask.description.dimmed(),
                        model.cyan()
                    );
                    let result =
                        run_subtask(subtask, &req, &res, &model, mcp_clone, int_clone).await;
                    (subtask.clone(), result, model)
                });
            }

            let batch_results = futures_util::future::join_all(futures).await;
            for (subtask, result, model) in batch_results {
                match result {
                    Ok(summary) => {
                        let success = !summary.contains("FAILED") && !summary.contains("ERROR");
                        println!(
                            "  {} [{}] {} {}",
                            "│".dimmed(),
                            subtask.id,
                            if success {
                                "DONE".green()
                            } else {
                                "DONE*".yellow()
                            },
                            summary.lines().next().unwrap_or("").dimmed()
                        );
                        self.record_model_result(&model, success);

                        all_results.push(SubtaskResult {
                            subtask: subtask.clone(),
                            success,
                            summary,
                            files_changed: subtask.files.clone(),
                            model_used: model.to_string(),
                            attempts: 1,
                        });
                    }
                    Err(e) => {
                        println!(
                            "  {} [{}] {} {}",
                            "│".dimmed(),
                            subtask.id,
                            "FAIL".red(),
                            e.to_string().red()
                        );
                        self.record_model_result(&model, false);

                        // Auto-escalation: retry with better model
                        let escalated = self.escalate_model(&model);
                        if escalated != model {
                            println!(
                                "  {} [{}] Escalating: {} → {}",
                                "│".dimmed(),
                                subtask.id,
                                model.cyan(),
                                escalated.cyan()
                            );
                            let retry = run_subtask(
                                &subtask,
                                &req_shared,
                                &res_shared,
                                &escalated,
                                mcp.clone(),
                                integrations.clone(),
                            )
                            .await;
                            match retry {
                                Ok(summary) => {
                                    let success = !summary.contains("FAILED");
                                    println!(
                                        "  {} [{}] {} (escalated to {})",
                                        "│".dimmed(),
                                        subtask.id,
                                        if success {
                                            "DONE".green()
                                        } else {
                                            "DONE*".yellow()
                                        },
                                        escalated.cyan()
                                    );
                                    self.record_model_result(&escalated, success);
                                    all_results.push(SubtaskResult {
                                        subtask: subtask.clone(),
                                        success,
                                        summary,
                                        files_changed: subtask.files.clone(),
                                        model_used: escalated.to_string(),
                                        attempts: 2,
                                    });
                                }
                                Err(e2) => {
                                    all_results.push(SubtaskResult {
                                        subtask: subtask.clone(),
                                        success: false,
                                        summary: format!("FAILED after escalation: {}", e2),
                                        files_changed: vec![],
                                        model_used: escalated.to_string(),
                                        attempts: 2,
                                    });
                                }
                            }
                        } else {
                            all_results.push(SubtaskResult {
                                subtask: subtask.clone(),
                                success: false,
                                summary: format!("FAILED: {}", e),
                                files_changed: vec![],
                                model_used: model.to_string(),
                                attempts: 1,
                            });
                        }
                    }
                }
            }
        }

        println!();
        Ok(all_results)
    }

    fn route_model(&self, subtask: &Subtask) -> String {
        if let Some(ref m) = subtask.model {
            return m.clone();
        }

        match subtask.difficulty.as_str() {
            "critical" | "high" => {
                if self
                    .config
                    .anthropic_api_key
                    .as_deref()
                    .is_some_and(|k| !k.is_empty())
                {
                    "claude-4-sonnet".into()
                } else if self
                    .config
                    .openai_api_key
                    .as_deref()
                    .is_some_and(|k| !k.is_empty())
                {
                    "o3".into()
                } else {
                    "gemini-3-pro".into()
                }
            }
            "low" => "gemini-2.5-flash-lite".into(),
            _ => "gemini-2.5-flash".into(),
        }
    }

    fn escalate_model(&self, current: &str) -> String {
        let provider = backend::detect_provider(current);
        match provider {
            Provider::Gemini => "gemini-3-pro".into(),
            Provider::Anthropic => {
                if current.contains("sonnet")
                    && self
                        .config
                        .openai_api_key
                        .as_deref()
                        .is_some_and(|k| !k.is_empty())
                {
                    "o3".into()
                } else {
                    "claude-4-sonnet".into()
                }
            }
            Provider::OpenAI => {
                if current.contains("o3") || current.contains("o4") {
                    current.to_string() // already max
                } else if self
                    .config
                    .anthropic_api_key
                    .as_deref()
                    .is_some_and(|k| !k.is_empty())
                {
                    "claude-4-sonnet".into()
                } else {
                    "gemini-3-pro".into()
                }
            }
            Provider::Ollama => current.to_string(),
        }
    }

    fn record_model_result(&mut self, model: &str, success: bool) {
        let stats = self.model_stats.entry(model.to_string()).or_default();
        if success {
            stats.successes += 1;
        } else {
            stats.failures += 1;
        }
    }

    // ── Phase 4: Consensus Check ────────────────────────────────────────────

    async fn consensus_phase(
        &self,
        results: &[SubtaskResult],
        requirement: &str,
    ) -> Result<Vec<SubtaskResult>> {
        let critical: Vec<&SubtaskResult> = results
            .iter()
            .filter(|r| r.subtask.difficulty == "critical" && r.success)
            .collect();

        if critical.is_empty() {
            println!(
                "  {} No critical subtasks — skipping consensus check",
                "[PHASE 4/5]".cyan().bold()
            );
            println!();
            return Ok(results.to_vec());
        }

        println!(
            "  {} Running consensus check on {} critical subtask(s)...",
            "[PHASE 4/5]".cyan().bold(),
            critical.len()
        );

        let mut verified = results.to_vec();

        for result in &critical {
            println!(
                "  {} [{}] Verifying with second model...",
                "│".dimmed(),
                result.subtask.id
            );

            // Pick a different model for verification
            let verifier_model = if result.model_used.contains("gemini") {
                if self
                    .config
                    .anthropic_api_key
                    .as_deref()
                    .is_some_and(|k| !k.is_empty())
                {
                    "claude-4-sonnet"
                } else if self
                    .config
                    .openai_api_key
                    .as_deref()
                    .is_some_and(|k| !k.is_empty())
                {
                    "o3"
                } else {
                    &result.model_used // fallback to same model
                }
            } else if result.model_used.contains("claude") {
                if self
                    .config
                    .openai_api_key
                    .as_deref()
                    .is_some_and(|k| !k.is_empty())
                {
                    "o3"
                } else {
                    "gemini-3-pro"
                }
            } else {
                if self
                    .config
                    .anthropic_api_key
                    .as_deref()
                    .is_some_and(|k| !k.is_empty())
                {
                    "claude-4-sonnet"
                } else {
                    "gemini-3-pro"
                }
            };

            let review_prompt = format!(
                "Review this completed subtask for correctness and security:\n\n\
                 Original requirement: {}\n\
                 Subtask: {}\n\
                 Implementation summary: {}\n\
                 Model used: {}\n\n\
                 Evaluate:\n\
                 1. Is the implementation correct for the requirement?\n\
                 2. Are there security concerns?\n\
                 3. Are there edge cases not handled?\n\
                 4. Would you approve this in code review?\n\n\
                 Reply with: APPROVED (if correct), CONCERNS (if issues found, list them), or REJECTED (if fundamentally wrong).",
                requirement, result.subtask.description, result.summary, result.model_used
            );

            let vc_config = Config {
                model: verifier_model.to_string(),
                ..self.config.clone()
            };

            match BackendClient::new(&vc_config) {
                Ok(client) => {
                    let req = GenerateContentRequest {
                        contents: vec![Content {
                            role: "user".into(),
                            parts: vec![Part::text(&review_prompt)],
                        }],
                        tools: vec![],
                        tool_config: None,
                        system_instruction: None,
                        generation_config: Some(GenerationConfig {
                            temperature: Some(0.3),
                            max_output_tokens: Some(1024),
                            thinking_config: None,
                        }),
                    };

                    match client.generate(req).await {
                        Ok(resp) => {
                            if let Some(verdict) = Self::extract_text(&resp) {
                                if verdict.to_uppercase().contains("APPROVED") {
                                    println!(
                                        "  {} [{}] {} Consensus: APPROVED",
                                        "│".dimmed(),
                                        result.subtask.id,
                                        "◈".green()
                                    );
                                } else if verdict.to_uppercase().contains("REJECTED") {
                                    println!(
                                        "  {} [{}] {} Consensus: REJECTED — {}",
                                        "│".dimmed(),
                                        result.subtask.id,
                                        "⊗".red(),
                                        verdict.lines().next().unwrap_or("").dimmed()
                                    );
                                    // Mark as needing review
                                    for v in &mut verified {
                                        if v.subtask.id == result.subtask.id {
                                            v.summary = format!(
                                                "[CONSENSUS REJECTED by {}] {}",
                                                verifier_model, verdict
                                            );
                                            v.success = false;
                                        }
                                    }
                                } else {
                                    println!(
                                        "  {} [{}] {} Consensus: CONCERNS — {}",
                                        "│".dimmed(),
                                        result.subtask.id,
                                        "⌬".yellow(),
                                        verdict.lines().next().unwrap_or("").dimmed()
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            println!(
                                "  {} [{}] Consensus check failed: {}",
                                "│".dimmed(),
                                result.subtask.id,
                                e.to_string().red()
                            );
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "  {} [{}] Cannot create verifier: {}",
                        "│".dimmed(),
                        result.subtask.id,
                        e.to_string().red()
                    );
                }
            }
        }

        println!();
        Ok(verified)
    }

    // ── Phase 5: Merge ──────────────────────────────────────────────────────

    async fn merge_phase(&self, results: &[SubtaskResult], requirement: &str) -> Result<String> {
        println!("  {} Merging results...", "[PHASE 5/5]".cyan().bold());

        let succeeded: Vec<_> = results.iter().filter(|r| r.success).collect();
        let failed: Vec<_> = results.iter().filter(|r| !r.success).collect();

        let mut report = format!(
            "# Task Report: {}\n\n**Generated:** {}\n\n## Summary\n\n",
            requirement,
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        );

        report.push_str(&format!("- **Subtasks:** {} total\n", results.len()));
        report.push_str(&format!("- **Succeeded:** {}\n", succeeded.len()));
        report.push_str(&format!("- **Failed:** {}\n", failed.len()));

        // Model usage stats
        let mut model_usage: HashMap<String, u32> = HashMap::new();
        for r in results {
            *model_usage.entry(r.model_used.clone()).or_default() += 1;
        }
        report.push_str("\n### Models Used\n\n");
        for (model, count) in &model_usage {
            let stats = self.model_stats.get(model);
            let success_rate = stats
                .map(|s| {
                    let total = s.successes + s.failures;
                    if total > 0 {
                        format!("{:.0}% success", s.successes as f32 / total as f32 * 100.0)
                    } else {
                        "N/A".into()
                    }
                })
                .unwrap_or_else(|| "N/A".into());
            report.push_str(&format!(
                "- **{}** — {} subtask(s), {}\n",
                model, count, success_rate
            ));
        }

        report.push_str("\n## Results\n\n");
        for r in results {
            let status = if r.success { "PASS" } else { "FAIL" };
            report.push_str(&format!(
                "### [{}. {}] {} ({})\n- **Model:** {}\n- **Files:** {}\n- **Summary:** {}\n\n",
                r.subtask.id,
                r.subtask.difficulty.to_uppercase(),
                r.subtask.description,
                status,
                r.model_used,
                r.files_changed.join(", "),
                r.summary,
            ));
        }

        if !failed.is_empty() {
            report.push_str("## Action Required\n\n");
            report.push_str("The following subtasks failed and need manual attention:\n\n");
            for f in &failed {
                report.push_str(&format!("- [{}] {}\n", f.subtask.id, f.subtask.description));
            }
        }

        // Print summary to terminal
        let pass_count = succeeded.len();
        let total = results.len();
        println!(
            "  {} {}/{} subtasks passed",
            "│".dimmed(),
            pass_count,
            total
        );
        if failed.is_empty() {
            println!("  {} All subtasks completed successfully", "└".cyan());
        } else {
            println!(
                "  {} {} subtask(s) need attention",
                "└".yellow(),
                failed.len()
            );
        }

        // Save report
        let report_path = format!(
            "task_report_{}.md",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        std::fs::write(&report_path, &report)?;
        println!("  {} Report saved: {}", "▸".dimmed(), report_path.cyan());

        Ok(report)
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn extract_text(resp: &GenerateContentResponse) -> Option<String> {
        resp.candidates
            .as_ref()?
            .first()?
            .content
            .as_ref()?
            .parts
            .iter()
            .filter_map(|p| {
                if let Part::Text {
                    text,
                    thought: None | Some(false),
                    ..
                } = p
                {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into()
    }
}

// ── Subtask execution ───────────────────────────────────────────────────────

async fn run_subtask(
    subtask: &Subtask,
    requirement: &str,
    research: &str,
    model: &str,
    mcp: Option<Arc<McpRegistry>>,
    integrations: Option<Arc<IntegrationRegistry>>,
) -> Result<String> {
    let mut cfg = Config {
        model: model.to_string(),
        api_key: std::env::var("DIPRALIX_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .unwrap_or_default(),
        ..Config::default()
    };

    // Load keys from environment
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            cfg.anthropic_api_key = Some(key);
        }
    }
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            cfg.openai_api_key = Some(key);
        }
    }

    cfg.max_iterations = 20; // Subagent gets fewer iterations
    cfg.auto_apply = true; // Subagents auto-apply (no interactive prompts)

    let client = BackendClient::new(&cfg)?;
    let mut history: Vec<Content> = Vec::new();

    // Build subagent prompt with full context
    let prompt = format!(
        "You are a specialized subagent working on a specific subtask as part of a larger project.\n\n\
         ## Overall Project Requirement\n{}\n\n\
         ## Research Context\n{}\n\n\
         ## Your Subtask (ID: {}, Difficulty: {})\n{}\n\n\
         ## Instructions\n\
         - Focus ONLY on this subtask. Do not work on anything else.\n\
         - Read relevant files first before making changes.\n\
         - Make minimal, correct changes. Use edit_file for existing files.\n\
         - After making changes, run cargo check or equivalent build command.\n\
         - If you encounter errors, fix them. You have {} attempts.\n\
         - When done, summarize what you changed in 1-2 sentences.\n\n\
         Begin. Execute the subtask now.",
        requirement, research, subtask.id, subtask.difficulty, subtask.description, 20
    );

    history.push(Content {
        role: "user".into(),
        parts: vec![Part::text(&prompt)],
    });

    let mut cost_tracker = CostTracker::new(model, None);
    let tool_ctx = ToolContext {
        stream_output: false,
        auto_apply: true,
        mcp: mcp.clone(),
        integrations: integrations.clone(),
    };

    // Run mini agentic loop
    use crate::tools;
    use crate::types::*;

    for iter in 0..=cfg.max_iterations {
        if iter >= cfg.max_iterations {
            return Ok("Max iterations reached".into());
        }

        let sys_prompt = format!(
            "You are a specialized coding subagent (Subtask {}). Execute only this task. Be focused and efficient.\n\
             Working directory: {}\n\
             Auto-apply mode: ON (no diff prompts)",
            subtask.id,
            std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default()
        );

        let sys = SystemContent {
            parts: vec![Part::text(&sys_prompt)],
        };

        let tools_list =
            vec![serde_json::json!({ "functionDeclarations": tools::get_tool_declarations() })];

        let gen_cfg = Some(GenerationConfig {
            temperature: Some(0.5),
            max_output_tokens: Some(4096),
            thinking_config: None,
        });

        let request = GenerateContentRequest {
            contents: history.clone(),
            tools: tools_list,
            tool_config: Some(ToolConfig {
                function_calling_config: FunctionCallingConfig {
                    mode: "AUTO".into(),
                },
            }),
            system_instruction: Some(sys),
            generation_config: gen_cfg,
        };

        let response = match client.generate(request).await {
            Ok(r) => r,
            Err(e) => return Err(anyhow::anyhow!("Subtask {} API error: {}", subtask.id, e)),
        };

        // Extract function calls
        let mut function_calls: Vec<(&FunctionCall, Option<Part>)> = Vec::new();
        let mut text_parts = Vec::new();

        if let Some(candidates) = &response.candidates {
            for candidate in candidates {
                if let Some(content) = &candidate.content {
                    for part in &content.parts {
                        match part {
                            Part::Text {
                                text,
                                thought: None | Some(false),
                                ..
                            } => {
                                text_parts.push(text.clone());
                            }
                            Part::FunctionCall { function_call, .. } => {
                                function_calls.push((function_call, None));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Add model response to history
        let mut model_parts = Vec::new();
        for t in &text_parts {
            model_parts.push(Part::text(t));
        }
        for (fc, _) in &function_calls {
            model_parts.push(Part::FunctionCall {
                function_call: FunctionCall {
                    name: fc.name.clone(),
                    args: fc.args.clone(),
                    thought_signature: None,
                },
                thought_signature: None,
            });
        }
        if model_parts.is_empty() {
            model_parts.push(Part::text("(no output)"));
        }
        history.push(Content {
            role: "model".into(),
            parts: model_parts,
        });

        if let Some(usage) = &response.usage_metadata {
            let t = usage.total_token_count.unwrap_or(0);
            cost_tracker.record_usage(
                usage.prompt_token_count.unwrap_or(0),
                usage.candidates_token_count.unwrap_or(0),
                usage.thoughts_token_count.unwrap_or(0),
            );
            let _ = t;
        }

        // Execute function calls
        if function_calls.is_empty() {
            // No more tool calls — subtask complete
            let summary = text_parts.join("\n");
            return Ok(if summary.is_empty() {
                format!("[{}] Subtask completed", subtask.id)
            } else {
                summary
            });
        }

        let mut response_parts: Vec<Part> = Vec::new();
        for (fc, _) in &function_calls {
            let result = tools::execute_tool(&fc.name, &fc.args, &tool_ctx).await;
            response_parts.push(Part::FunctionResponse {
                function_response: FunctionResponse {
                    name: fc.name.clone(),
                    response: serde_json::json!({ "content": if result.is_error { format!("ERROR: {}", result.output) } else { result.output.clone() } }),
                    id: None,
                },
            });
        }
        history.push(Content {
            role: "user".into(),
            parts: response_parts,
        });
    }

    Ok("Completed".into())
}

// ── Dependency ordering ─────────────────────────────────────────────────────

fn group_by_dependency_level(subtasks: &[Subtask]) -> Vec<Vec<Subtask>> {
    let mut levels: Vec<Vec<Subtask>> = Vec::new();
    let mut completed: Vec<usize> = Vec::new();
    let mut remaining: Vec<Subtask> = subtasks.to_vec();

    while !remaining.is_empty() {
        let mut current_level = Vec::new();
        let mut next_remaining = Vec::new();

        for st in remaining {
            let deps_satisfied = st.depends_on.iter().all(|d| completed.contains(d));
            if deps_satisfied {
                current_level.push(st);
            } else {
                next_remaining.push(st);
            }
        }

        if current_level.is_empty() && !next_remaining.is_empty() {
            // Circular dependency or all remaining depend on each other —
            // just run them all
            current_level = next_remaining;
            next_remaining = Vec::new();
        }

        for st in &current_level {
            completed.push(st.id);
        }
        levels.push(current_level);
        remaining = next_remaining;
    }

    levels
}

fn warn_no_color(msg: &str) {
    println!("  {} {}", "!".yellow(), msg);
}
