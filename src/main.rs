use anyhow::Result;
use clap::Parser;

use dipralix::sync;

mod agent;
mod approval;
mod ast;
mod audit;
mod backend;
mod browser;
mod ci_runner;
mod comment_protocol;
mod config;
mod context;
mod debate;
mod diff_view;
mod domain_bootstrap;
mod domain_knowledge;
#[cfg(feature = "ember")]
mod ember;
mod fingerprint;
mod infra;
mod integrations;
mod learning;
mod living_docs;
mod mcp;
mod memory;
mod models;
mod orchestrator;
mod packer;
mod plan_visualizer;
mod project;
mod safety;
mod security;
mod session;
mod snapshot;
mod token_counter;
mod tools;
mod types;
mod ui;
#[cfg(feature = "ember")]
mod voice;

#[cfg(test)]
mod test_harness;

#[derive(Parser, Debug)]
#[clap(
    name    = "dipralix",
    about   = "DIPRALIX — Multi-model terminal AI coding agent",
    version = env!("CARGO_PKG_VERSION"),
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

    /// Realtime sync: join a room and stream `.dipralix/` changes.
    /// See `dipralix-realtime.md` §10 Phase 1.
    #[clap(long)]
    sync: bool,

    /// (realtime) WebSocket URL of the sync server, e.g. `ws://host:7878`.
    #[clap(long, requires = "sync")]
    server: Option<String>,

    /// (realtime) JWT bearer token for the room.
    #[clap(long, requires = "sync")]
    token: Option<String>,

    /// (realtime) Room (project) name to join.
    #[clap(long, requires = "sync")]
    room: Option<String>,

    /// (realtime) Identity advertised to other clients.
    #[clap(long, requires = "sync")]
    user: Option<String>,

    /// (realtime) Project root; the watcher scopes to `<root>/.dipralix/`.
    #[clap(long, requires = "sync")]
    project_root: Option<String>,

    /// (realtime) Serverless P2P mesh: discover peers over mDNS on the LAN and
    /// sync over Noise-encrypted TCP. No `--server`/`--token` needed.
    #[clap(long, requires = "sync")]
    mesh: bool,

    /// (realtime, mesh) TCP port to bind for peer links (0 picks an ephemeral port).
    #[clap(long, requires = "mesh", default_value_t = 0)]
    mesh_port: u16,

    /// (realtime, mesh) Shared room secret; stretched into the Noise key. Every
    /// peer that knows this secret (and is on the LAN) can join the room.
    #[clap(long, requires = "mesh")]
    secret: Option<String>,

    /// (realtime, mesh) Manually add a peer `host:port` to dial, in addition to
    /// mDNS discovery. Repeatable; useful when multicast is firewalled.
    #[clap(long = "peer", requires = "mesh")]
    peers: Vec<String>,
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
    if args.sync {
        return run_realtime(&args).await;
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
        || std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .is_some_and(|k| !k.is_empty())
        || std::env::var("OPENAI_API_KEY")
            .ok()
            .is_some_and(|k| !k.is_empty());

    let api_key = args
        .api_key
        .or(file_cfg.api_key)
        .or_else(|| std::env::var("DIPRALIX_API_KEY").ok())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .unwrap_or_default();

    if api_key.is_empty() && !has_alt_key {
        anyhow::bail!(
            "No API key found.\n\
             Set DIPRALIX_API_KEY, ANTHROPIC_API_KEY, or OPENAI_API_KEY.\n\
             Free Gemini key: https://aistudio.google.com/apikey"
        );
    }

    // Model resolution.
    //   1. Explicit --model flag → use it as-is
    //   2. Otherwise → "auto": per-message routing in agent.rs picks the right model
    //      for each task (complex → reasoning model, simple → fast model). This
    //      replaces the old "fetch best model from first provider" heuristic, which
    //      hardcoded specific model names that drift out of date.
    let model = args
        .model
        .clone()
        .or(file_cfg.model.clone())
        .unwrap_or_else(|| "auto".to_string());

    if model == "auto" {
        crate::ui::nullvoid::print_model_detect("auto", "task-routed", 0);
    }

    let thinking = args.think || file_cfg.thinking;
    let budget = if args.think {
        args.think_budget
    } else {
        file_cfg.thinking_budget
    };
    let auto_apply = args.auto_apply || file_cfg.auto_apply;
    let max_iterations = if args.max_iter != 50 {
        args.max_iter
    } else {
        file_cfg.max_iterations
    };

    let config = config::Config {
        api_key,
        model,
        grounding: args.grounding || file_cfg.grounding,
        thinking,
        thinking_budget: budget,
        auto_apply,
        max_iterations: if max_iterations == 0 {
            0
        } else {
            max_iterations.max(1)
        },
        context_warn: file_cfg.context_warn,
        context_compact: file_cfg.context_compact,
        mcp_servers: file_cfg.mcp_servers,
        integrations: file_cfg.integrations,
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
        let prompt = args
            .prompt
            .as_deref()
            .unwrap_or("Fix any issues in this project");
        let result = ci_runner::run_ci(&config, prompt).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        if !result.success {
            std::process::exit(1);
        }
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

/// Dispatch the realtime sync subcommand. Two modes:
/// - **mesh** (`--mesh`): serverless P2P over mDNS + encrypted TCP.
/// - **server** (default): WebSocket client against a `dipralix-server`,
///   requiring `--server` and `--token`.
async fn run_realtime(args: &Args) -> Result<()> {
    init_sync_tracing();
    let room = args.room.clone().unwrap_or_else(|| "default".to_string());
    let user = args
        .user
        .clone()
        .unwrap_or_else(|| std::env::var("USER").unwrap_or_else(|_| "anon".to_string()));
    let project_root =
        std::path::PathBuf::from(args.project_root.clone().unwrap_or_else(|| ".".to_string()));

    if args.mesh {
        let secret = args.secret.clone().ok_or_else(|| {
            anyhow::anyhow!("--mesh requires --secret <room-secret> (shared by the team)")
        })?;
        let seed_peers = resolve_peers(&args.peers)?;
        let node = sync::MeshNode::new(room, user, &secret, project_root, args.mesh_port)
            .with_seed_peers(seed_peers);
        return node.run().await.map_err(Into::into);
    }

    let server = args
        .server
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--sync requires --server <ws-url> (or use --mesh)"))?;
    let token = args
        .token
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--sync requires --token <jwt>"))?;

    let cfg = sync::ClientConfig {
        server,
        token,
        room,
        user,
        project_root,
    };
    let client = sync::SyncClient::new(cfg);
    client.run().await.map_err(Into::into)
}

/// Resolve `host:port` peer strings to socket addresses (first resolved addr
/// each). DNS/`/etc/hosts` resolution happens here, at startup.
fn resolve_peers(specs: &[String]) -> Result<Vec<std::net::SocketAddr>> {
    use std::net::ToSocketAddrs;
    let mut out = Vec::with_capacity(specs.len());
    for spec in specs {
        let addr = spec
            .to_socket_addrs()
            .map_err(|e| anyhow::anyhow!("bad --peer '{spec}': {e}"))?
            .next()
            .ok_or_else(|| anyhow::anyhow!("--peer '{spec}' resolved to no address"))?;
        out.push(addr);
    }
    Ok(out)
}

/// Initialize structured tracing for the sync client side, mirroring
/// the server binary's behavior.
fn init_sync_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,dipralix::sync=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
