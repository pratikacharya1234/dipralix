use anyhow::Result;
use clap::Parser;

mod agent;
mod approval;
mod ast;
mod audit;
mod backend;
mod browser;
mod comment_protocol;
mod config;
mod context;
mod debate;
mod diff_view;
mod fingerprint;
mod infra;
mod learning;
mod living_docs;
mod memory;
mod plan_visualizer;
mod types;
mod integrations;
mod mcp;
mod models;
mod orchestrator;
mod project;
mod safety;
mod security;
mod session;
mod snapshot;
mod token_counter;
mod tools;
mod ui;
mod packer;
mod ci_runner;
mod domain_knowledge;
mod domain_bootstrap;
#[cfg(feature = "ember")]
mod voice;
#[cfg(feature = "ember")]
mod ember;

#[cfg(test)]
mod test_harness;

#[derive(Parser, Debug)]
#[clap(
    name    = "dipralix",
    about   = "DIPRALIX — Multi-model terminal AI coding agent",
    version = "0.1.0",
    long_about = None
)]
struct Args {
    /// API key for Gemini backend (or set DIPRALIX_API_KEY / GEMINI_API_KEY env var).
    #[clap(short = 'k', long, env = "DIPRALIX_API_KEY")]
    api_key: Option<String>,

    /// Model to use (auto-detected if not specified).
    #[clap(short, long)]
    model: Option<String>,

    /// Enable Google Search grounding.
    #[clap(short, long)]
    grounding: bool,

    /// Enable ThinkMode (gemini-2.5+ only).
    #[clap(short, long)]
    think: bool,

    /// ThinkMode token budget (default 8000, max 24576, 0 = unlimited).
    #[clap(long, default_value = "8000")]
    think_budget: i32,

    /// Auto-apply all file changes without diff preview.
    #[clap(long)]
    auto_apply: bool,

    /// Pack project context into a portable file for sharing with any AI.
    #[clap(long)]
    pack: Option<String>,

    /// Custom API base URL for proxying (e.g., LiteLLM, OpenRouter).
    #[clap(long)]
    api_base: Option<String>,
    #[clap(long)]
    ci: bool,

    /// Voice input — record mic, transcribe via Gemini, run as prompt.
    /// (Under development — coming in v0.0.3)
    #[clap(long, hide = true)]
    voice: bool,

    /// EMBER — real-time voice AI with Google TTS responses.
    /// (Under development — coming in v0.0.3)
    #[clap(long, hide = true)]
    ember: bool,

    #[clap(long)]
    pipeline: Option<String>,

    /// Max tool-call iterations per turn before pausing (0 = unlimited).
    #[clap(long, default_value = "50")]
    max_iter: u32,

    /// Run a single prompt non-interactively and exit.
    #[clap(short, long)]
    prompt: Option<String>,

    /// Attach a screenshot to the initial prompt (ScreenFix).
    #[clap(long)]
    screenshot: Option<String>,

    /// Domain preset: mobile, web, ai, deeplearning, desktop, hardware,
    /// gamedev, devops, data, general. Skips interactive selector.
    #[clap(long)]
    domain: Option<String>,

    /// Anthropic (Claude) API key.
    #[clap(long, env = "ANTHROPIC_API_KEY")]
    anthropic_api_key: Option<String>,

    /// OpenAI API key.
    #[clap(long, env = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,

    /// Explain planned actions before executing tools.
    #[clap(long)]
    explain: bool,

    /// Initialize .dipralix/ scaffolding (project.md, conventions.md, safety.toml, approval.toml).
    #[clap(long)]
    init: bool,

    /// Show project fingerprint and quality score, then exit.
    #[clap(long)]
    fingerprint: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle non-interactive modes
    if args.init {
        fingerprint::run_init()?;
        return Ok(());
    }
    if args.fingerprint {
        fingerprint::run_fingerprint();
        return Ok(());
    }
    if let Some(ref output) = args.pack {
        let msg = packer::pack_project(Some(output))?;
        println!("  ⊞ {}", msg);
        return Ok(());
    }

    let file_cfg = config::Config::file_defaults();

    // Allow launch without Gemini key if Claude/OpenAI keys are available
    let has_alt_key = args.anthropic_api_key.is_some()
        || args.openai_api_key.is_some()
        || file_cfg.anthropic_api_key.is_some()
        || file_cfg.openai_api_key.is_some()
        || std::env::var("ANTHROPIC_API_KEY").ok().is_some_and(|k| !k.is_empty())
        || std::env::var("OPENAI_API_KEY").ok().is_some_and(|k| !k.is_empty());

    let api_key = args.api_key
        .or(file_cfg.api_key)
        .or_else(|| std::env::var("DIPRALIX_API_KEY").ok())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .unwrap_or_else(|| {
            if has_alt_key {
                String::new() // OK — user will switch model before using Gemini
            } else {
                String::new() // Will fail on first Gemini API call
            }
        });

    if api_key.is_empty() && !has_alt_key {
        anyhow::bail!(
            "No API key found.\n\
             Set DIPRALIX_API_KEY, ANTHROPIC_API_KEY, or OPENAI_API_KEY.\n\
             Free Gemini key: https://aistudio.google.com/apikey"
        );
    }

    // Resolve model: CLI arg → config file → auto-detect from best available provider
    let model = if let Some(m) = args.model.clone().or(file_cfg.model.clone()) {
        m
    } else {
        // Try Anthropic first, then OpenAI, then Gemini
        let anthropic_key = args.anthropic_api_key.clone()
            .or(file_cfg.anthropic_api_key.clone())
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .filter(|k| !k.is_empty());

        let openai_key = args.openai_api_key.clone()
            .or(file_cfg.openai_api_key.clone())
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .filter(|k| !k.is_empty());

        if let Some(key) = anthropic_key {
            match models::fetch_anthropic_models(&key).await {
                Ok(list) => {
                    let best = models::resolve_best_anthropic(&list);
                    crate::ui::nullvoid::print_model_detect(&best, "Anthropic", list.len());
                    best
                }
                Err(_) => "claude-sonnet-4-20250514".to_string()
            }
        } else if let Some(key) = openai_key {
            match models::fetch_openai_models(&key).await {
                Ok(list) => {
                    let best = models::resolve_best_openai(&list);
                    crate::ui::nullvoid::print_model_detect(&best, "OpenAI", list.len());
                    best
                }
                Err(_) => "gpt-4o".to_string()
            }
        } else if !api_key.is_empty() {
            match models::fetch_available_models(&api_key).await {
                Ok(all) => {
                    let best = models::resolve_best_model(&all);
                    crate::ui::nullvoid::print_model_detect(&best, "Gemini", all.len());
                    best
                }
                Err(_) => "gemini-2.5-flash".to_string()
            }
        } else {
            anyhow::bail!(
                "No API key found for any provider.\n\
                 Set DIPRALIX_API_KEY, ANTHROPIC_API_KEY, or OPENAI_API_KEY.\n\
                 Free keys: https://aistudio.google.com/apikey"
            );
        }
    };

    let thinking = args.think || file_cfg.thinking;
    let budget   = if args.think { args.think_budget } else { file_cfg.thinking_budget };
    let auto_apply    = args.auto_apply  || file_cfg.auto_apply;
    let max_iterations = if args.max_iter != 50 { args.max_iter } else { file_cfg.max_iterations };

    let config = config::Config {
        api_key,
        model,
        grounding:       args.grounding || file_cfg.grounding,
        thinking,
        thinking_budget: budget,
        auto_apply,
        max_iterations:  if max_iterations == 0 { 0 } else { max_iterations.max(1) },
        context_warn:    file_cfg.context_warn,
        context_compact: file_cfg.context_compact,
        mcp_servers:     file_cfg.mcp_servers,
        integrations:    file_cfg.integrations,
        daily_budget_usd: file_cfg.daily_budget_usd,
        anthropic_api_key: args.anthropic_api_key.or(file_cfg.anthropic_api_key),
        openai_api_key: args.openai_api_key.or(file_cfg.openai_api_key),
        explain_before_execute: args.explain || file_cfg.explain_before_execute,
        api_base: args.api_base,
            domain: args.domain,
    };

    // ▸ EMBER / Voice — under development for v0.0.3
    // ▸ See src/ember.rs and src/voice.rs
    /*
    // EMBER mode — real-time voice conversation loop (explicit flag)
    if args.ember {
        ember::ember_loop(&config).await?;
        return Ok(());
    }

    // Voice mode — record mic, transcribe, run as prompt
    if args.voice {
        let text = voice::voice_prompt(&config.api_key, 10).await?;
        agent::run_once(&config, &text, None).await?;
        return Ok(());
    }
    */

    // CI headless mode — run prompt, output JSON, exit
    if args.ci {
        let prompt = args.prompt.as_deref().unwrap_or("Fix any issues in this project");
        let result = ci_runner::run_ci(&config, prompt).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        if !result.success { std::process::exit(1); }
        return Ok(());
    }

    // Pipeline mode — run a named pipeline and exit
    if let Some(ref name) = args.pipeline {
        ci_runner::run_pipeline(&config, name).await?;
        return Ok(());
    }

    if let Some(prompt) = args.prompt {
        agent::run_once(&config, &prompt, args.screenshot.as_deref()).await?;
    } else {
        agent::run_interactive(&config).await?;
    }

    /* ▸ EMBER path — disabled for v0.0.2, ship in v0.0.3
    } else if ember::mic_available() {
        if crate::ui::nullvoid::print_mode_selector() {
            agent::run_interactive(&config).await?;
        } else {
            ember::ember_loop(&config).await?;
        }
    */

    Ok(())
}
