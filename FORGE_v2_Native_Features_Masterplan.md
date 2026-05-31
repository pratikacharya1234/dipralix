# DIPRALIX v2: NATIVE FEATURES MASTERPLAN
## "Stealing the Best Community Ideas, Building Them Better in Rust"

---

## RESEARCH FINDINGS: What the Community Built (That We Should Own)

I analyzed every major community extension, plugin, and hack built around Claude Code, Cursor, Aider, OpenCode, and Cline in 2026. Here is what developers are duct-taping together because the core tools don't provide it natively:

### Community Innovations Discovered

| Tool | Community Built | Problem It Solves | Why It's External |
|------|----------------|-------------------|-----------------|
| **Claude Code** | MemClaw | Persistent project memory across sessions | Claude forgets everything between sessions |
| **Claude Code** | Plannotator | Visual plan review with annotations | Claude plans are text-only, hard to track |
| **Claude Code** | Ralph Wiggum Loop | Long-running multi-task with auto-commit | Claude context pollution after 3+ tasks |
| **Claude Code** | Agent-Peer-Review | Cross-model code review (Claude ↔ Codex) | Single model misses bugs |
| **Claude Code** | Shipyard | IaC validation + security auditor | Claude doesn't understand Terraform/K8s |
| **Claude Code** | Dev-Browser | Headless browsing with low context cost | Playwright MCP is heavy and slow |
| **Claude Code** | Firecrawl | Web scraping with JS rendering | Claude's native fetch is primitive |
| **Aider** | Comment Protocol (`// AI:`) | Drive agent from source code comments | Aider only works in terminal |
| **Aider** | `--watch-files` | Real-time file sync with IDE | No native IDE bridge |
| **Pi** | Lazy-Loading Skills | Tiny system prompt, load skills on demand | Most agents waste 50% tokens on irrelevant context |
| **Cline** | Approval-First Workflow | Per-action approval before execution | Auto-apply is dangerous for destructive ops |
| **Caliber** | Project Fingerprinting | Auto-generate AI configs from codebase | Manual CLAUDE.md creation is tedious |
| **Kiro** | Spec-Driven Development | Write spec → auto-generate tasks | No native spec-to-code pipeline |
| **OpenCode** | Planning Agents | Separate planning vs execution agents | Single agent can't plan and execute well |
| **Continue** | Custom Model Routing | Route different tasks to different models | Most tools are single-model |

**The Pattern:** Every successful AI coding tool has a **community plugin economy** because the core product is incomplete. Claude Code has 9,000+ plugins. Cursor has a marketplace. Aider has dozens of IDE extensions.

**DIPRALIX v2 Strategy:** Don't build a plugin economy. **Build every critical feature natively into the Rust core.** No external dependencies. No MCP servers. No skills. Just a single binary that does everything.

---

## THE 10 NATIVE FEATURES OF DIPRALIX v2

---

### 1. MEMORY CORE
**Inspired by:** MemClaw + Caliber  
**Community Pain:** Claude Code forgets everything between sessions. MemClaw is an external API service that costs money and requires setup. Caliber fingerprints projects but doesn't learn over time.

**DIPRALIX v2 Native Implementation:**

```rust
// src/memory_core.rs
pub struct MemoryCore {
    project_memory: ProjectMemory,      // .dipralix/memory/
    cross_project_patterns: PatternDB,  // ~/.dipralix/patterns/
    living_readme: LivingReadme,        // Auto-updated README
    session_continuity: SessionBridge,  // Resume any session in 8 seconds
}
```

**What it does:**
- **Auto-evolving project memory:** Every session, DIPRALIX appends learnings to `.dipralix/memory/session_001.md`, `.dipralix/memory/decisions.md`, `.dipralix/memory/errors.md`. Not static — it grows.
- **Cross-project pattern learning:** "You use `Result<T, E>` with `thiserror` in all your Rust projects. Auto-applying pattern here." Stored in `~/.dipralix/patterns/rust_error_handling.md`.
- **Living README:** `README.md` auto-updates when you add new modules, change architecture, or update dependencies. `dipralix --sync-readme` or auto-on-commit.
- **8-second session restore:** `dipralix --resume` loads the last 5 sessions' context, open files, pending tasks, and model state. Not from scratch.
- **No external service.** All files are local markdown. Git-trackable. Human-readable.

**Why it's better than MemClaw:**
- MemClaw requires API keys and external storage. DIPRALIX Memory Core is just files in `.dipralix/`.
- MemClaw costs money. DIPRALIX Memory Core is free.
- MemClaw is black-box. DIPRALIX Memory Core is transparent markdown you can edit.

**Commands:**
```bash
/memorize "Always use tokio::sync::RwLock for concurrent access"
/forget "old_pattern_about_std_mutex"
/memory view                    # Show all active memories
/memory export                 # Export to .dipralix/memory/
/memory sync                   # Cross-project pattern sync
```

---

### 2. PLAN VISUALIZER
**Inspired by:** Plannotator + OpenCode Planning Agents  
**Community Pain:** Claude Code's plans are flat text lists. OpenCode has planning agents but no visualization. Developers lose track of complex multi-step tasks.

**DIPRALIX v2 Native Implementation:**

```rust
// src/plan_visualizer.rs
pub struct PlanVisualizer {
    dependency_graph: Graph<TaskNode>,
    progress_tracker: ProgressTracker,
    risk_analyzer: RiskAnalyzer,
    renderer: TerminalRenderer,
}
```

**What it does:**
- **Terminal-native dependency graphs:** When you run `/task "Add OAuth2 to API"`, DIPRALIX renders a live ASCII dependency graph in the terminal:
  ```
  ┌─────────────────┐
  │ 1. Add JWT lib  │ ◀───┐
  └────────┬────────┘     │
           │              │
  ┌────────▼────────┐     │
  │ 2. Create /auth │     │
  │    endpoints    │ ◀───┤ (parallel)
  └────────┬────────┘     │
           │              │
  ┌────────▼────────┐     │
  │ 3. Add middleware│     │
  │    to routes    │ ◀───┘
  └────────┬────────┘
           │
  ┌────────▼────────┐
  │ 4. Write tests  │
  └─────────────────┘
  ```
- **Progress bars with risk indicators:** Each task shows `[████░░░░░░] 40%` and a risk score (`🔒 Safe`, `⚠️ Review`, `🔴 Danger`).
- **Real-time plan mutation:** If a subtask fails, the graph updates — failed node turns red, dependent tasks pause, DIPRALIX suggests replanning.
- **Plan diff:** `/plan diff` shows what changed in the plan since last session.
- **No web UI.** Pure terminal ASCII art with Unicode box-drawing. Works in any terminal, including SSH.

**Why it's better than Plannotator:**
- Plannotator is a separate web UI you have to open. DIPRALIX Plan Visualizer is in your terminal where you already are.
- Plannotator requires a plugin install. DIPRALIX Plan Visualizer is built-in.
- Plannotator is static. DIPRALIX Plan Visualizer updates live as tasks execute.

**Commands:**
```bash
/task "refactor auth module"   # Auto-generates plan graph
/plan view                     # Show current plan
/plan risk                     # Analyze plan risks
/plan replan                   # Regenerate from current state
/plan export                   # Save plan as .dipralix/plans/oauth2.md
```

---

### 3. PEER REVIEW ENGINE
**Inspired by:** Agent-Peer-Review (Claude ↔ Codex debate)  
**Community Pain:** Single-model agents miss bugs. The community built a plugin that makes Claude and Codex argue with each other. It works but costs 2× API calls and requires two separate tools.

**DIPRALIX v2 Native Implementation:**

```rust
// src/peer_review.rs
pub struct PeerReviewEngine {
    red_team: ModelDispatcher,    // Argues FOR the solution
    blue_team: ModelDispatcher,   // Argues AGAINST the solution
    arbitrator: ConsensusEngine,  // DIPRALIX decides winner
    evidence_collector: EvidenceCollector,
}
```

**What it does:**
- **Red Team vs Blue Team:** When DIPRALIX generates a critical change (e.g., auth logic, payment flow), it automatically spawns a debate:
  - **Red Team (fast model):** "This implementation is correct because..."
  - **Blue Team (reasoning model):** "But what about edge case X? And performance issue Y?"
  - **Arbitrator (DIPRALIX core):** Evaluates evidence, checks test results, reviews git diff, declares winner.
- **Evidence-based arbitration:** Not just "model A said this, model B said that." DIPRALIX runs tests, checks type safety, verifies against `.dipralix/safety.toml`, and makes a data-driven decision.
- **Auto-escalation:** If debate is inconclusive, auto-escalate to human with a structured report: "Red Team says safe. Blue Team found 2 issues. Tests pass. Recommend: Approve with note."
- **Cost control:** Debates only run on high-risk changes (safety level ≥ `Confirm`). Low-risk changes skip debate.

**Why it's better than Agent-Peer-Review:**
- Agent-Peer-Review requires two separate API subscriptions (Claude + OpenAI). DIPRALIX uses its existing multi-model keys.
- Agent-Peer-Review is a plugin with setup friction. DIPRALIX Peer Review is automatic.
- Agent-Peer-Review has no arbitration logic — it just shows you both opinions. DIPRALIX makes a decision.

**Commands:**
```bash
/review on                     # Enable peer review for session
/review off                    # Disable for speed
/review force                  # Manually trigger review of last change
/review config                 # Set debate thresholds
```

---

### 4. CODE FINGERPRINTING
**Inspired by:** Caliber (auto-generates AI configs)  
**Community Pain:** Developers spend 30 minutes manually writing `CLAUDE.md` or `.cursor/rules` for every project. Caliber automates this but is a separate CLI tool.

**DIPRALIX v2 Native Implementation:**

```rust
// src/fingerprinting.rs
pub struct CodeFingerprinter {
    stack_detector: StackDetector,
    convention_analyzer: ConventionAnalyzer,
    config_generator: ConfigGenerator,
    quality_scorer: QualityScorer,
}
```

**What it does:**
- **`dipralix init` — One command, full project DNA:**
  ```bash
  $ dipralix init
  🔍 Scanning 847 files...
  📦 Detected: Rust + Axum + SQLx + PostgreSQL
  📝 Generated: .dipralix/project.md
  🛡️  Generated: .dipralix/safety.toml
  🧠 Generated: .dipralix/conventions.md
  📊 Quality Score: 72/100 (suggestions: add error handling, missing tests)
  ```
- **Stack detection:** Tree-sitter parses `Cargo.toml`, `package.json`, `go.mod`, `requirements.txt`, `Dockerfile`, `docker-compose.yml`, `.github/workflows/*.yml` — detects full tech stack with versions.
- **Convention analysis:** Scans existing code to infer patterns:
  - "All error types use `thiserror` with `#[error(...)]`"
  - "HTTP handlers return `Json<T>` with `StatusCode`"
  - "Tests use `tokio::test` with `#[serial]` for DB tests"
  - "Imports are grouped: std, external, internal"
- **Auto-generated `.dipralix/project.md`:** Not a template — a living document based on YOUR actual code.
- **Quality scoring:** Caliber-style scoring. "Your project has 72/100. Issues: 3 unwraps, no CI, missing CONTRIBUTING.md."
- **One-time setup, continuous updates:** `dipralix init` runs once. `dipralix --sync` updates configs when dependencies change.

**Why it's better than Caliber:**
- Caliber is a separate tool (`npm install -g caliber`). DIPRALIX fingerprinting is `dipralix init` — already installed.
- Caliber generates static configs. DIPRALIX continuously updates them as code evolves.
- Caliber doesn't analyze conventions from existing code. DIPRALIX does.

**Commands:**
```bash
dipralix init                     # Fingerprint current directory
dipralix init --template rust-api # Use template + fingerprint
dipralix --sync                   # Update configs from current code
/fingerprint                   # Show detected patterns
/fingerprint quality           # Show quality score
```

---

### 5. COMMENT PROTOCOL
**Inspired by:** Aider Comment-Driven Workflow (`// AI: ...`)  
**Community Pain:** Aider's `// AI: refactor this` is brilliant but only works in Aider's terminal. No IDE integration. DIPRALIX should own this natively across all surfaces.

**DIPRALIX v2 Native Implementation:**

```rust
// src/comment_protocol.rs
pub struct CommentProtocol {
    scanner: CommentScanner,
    task_queue: TaskQueue,
    file_watcher: FileWatcher,
    ide_bridge: IdeBridge,
}
```

**What it does:**
- **In-source directives:** Write `// DIPRALIX: refactor this to use async/await` in any source file. DIPRALIX scans on startup and queues the task.
- **Multi-language support:**
  - Rust: `// DIPRALIX: ...`
  - Python: `# DIPRALIX: ...`
  - JavaScript: `// DIPRALIX: ...` or `/* DIPRALIX: ... */`
  - Go: `// DIPRALIX: ...`
  - YAML: `# DIPRALIX: ...`
- **Real-time file watcher:** As you save files with new `// DIPRALIX:` comments, the terminal shows:
  ```
  📥 New task detected in src/auth.rs: "refactor to use async/await"
  ⏳ Queued: 3 tasks pending
  💡 Run /tasks to view queue
  ```
- **IDE integration:** VS Code extension shows `// DIPRALIX:` comments as clickable buttons — "Execute", "Edit", "Dismiss".
- **Task lifecycle:**
  1. Comment added → Detected by watcher
  2. Task queued → Shown in `/tasks`
  3. User runs `/task execute 3` → DIPRALIX implements
  4. Comment auto-replaced with `// DIPRALIX-DONE: ...` or removed
  5. Git commit with conventional message

**Why it's better than Aider:**
- Aider's comment protocol is terminal-only. DIPRALIX works in terminal + IDE + background.
- Aider requires `--watch-files` flag. DIPRALIX watches by default.
- Aider doesn't queue tasks — it executes immediately. DIPRALIX queues for review.

**Commands:**
```bash
/tasks                         # View queued tasks from comments
/tasks execute 3              # Execute task #3
/tasks dismiss 2              # Remove task #2 from queue
/tasks auto                   # Auto-execute all safe tasks
```

---

### 6. LAZY CONTEXT
**Inspired by:** Pi (lazy-loading skill system)  
**Community Pain:** Most AI agents load a massive system prompt + all context for every request. 50% of tokens are wasted on irrelevant patterns. Pi solved this with lazy-loading but is a separate tool.

**DIPRALIX v2 Native Implementation:**

```rust
// src/lazy_context.rs
pub struct LazyContext {
    skill_registry: SkillRegistry,      // ~/.dipralix/skills/
    semantic_router: SemanticRouter,   // Vector DB + routing
    context_assembler: ContextAssembler,
    token_optimizer: TokenOptimizer,
}
```

**What it does:**
- **Hierarchical skill system:** Skills are modular context blocks stored in `~/.dipralix/skills/`:
  ```
  ~/.dipralix/skills/
  ├── rust/
  │   ├── error_handling.md      (2K tokens)
  │   ├── async_patterns.md      (1.5K tokens)
  │   └── axum_best_practices.md (3K tokens)
  ├── frontend/
  │   ├── react_hooks.md
  │   └── tailwind_patterns.md
  └── devops/
      ├── docker_multi_stage.md
      └── k8s_deployments.md
  ```
- **Semantic routing:** Before each request, DIPRALIX's vector DB (embedded) determines which skills are relevant:
  - User asks about "error handling in Rust" → Load `rust/error_handling.md` + `rust/async_patterns.md`
  - User asks about "Docker optimization" → Load `devops/docker_multi_stage.md`
  - User asks about "API design" → Load `rust/axum_best_practices.md` + `frontend/react_hooks.md` (if fullstack)
- **Dynamic context assembly:** Only loads skills + relevant codebase context. Typical reduction: 60% fewer tokens per request.
- **Project-specific skills:** `.dipralix/skills/` in project root overrides global skills. Team-shared patterns.
- **Auto-skill generation:** When DIPRALIX encounters a new pattern 3+ times, it auto-generates a skill file: "Detected you use `sqlx::query_as!` macros. Created skill `rust/sqlx_patterns.md`."

**Why it's better than Pi:**
- Pi is JavaScript/Node.js. DIPRALIX Lazy Context is Rust-native with zero-copy assembly.
- Pi's skills are static. DIPRALIX's skills auto-generate from your codebase.
- Pi has no vector routing. DIPRALIX uses embedded vector DB for sub-millisecond routing.

**Commands:**
```bash
/skills list                   # Show all available skills
/skills load rust_async       # Manually load a skill
/skills generate              # Auto-generate from recent patterns
/skills stats                 # Show token savings from lazy loading
```

---

### 7. INFRA AWARENESS
**Inspired by:** Shipyard (IaC validation + security auditor)  
**Community Pain:** Shipyard is a Claude Code plugin for Terraform/K8s validation. It requires plugin install, separate API calls, and doesn't understand your specific infrastructure.

**DIPRALIX v2 Native Implementation:**

```rust
// src/infra_awareness.rs
pub struct InfraAwareness {
    hcl_parser: HclParser,           // Terraform
    yaml_parser: K8sYamlParser,      // Kubernetes
    docker_parser: DockerfileParser, // Docker
    cost_estimator: CostEstimator,
    drift_detector: DriftDetector,
    security_scanner: InfraSecurityScanner,
}
```

**What it does:**
- **Native Terraform parsing:** `terraform plan` output parsed by Rust HCL parser. DIPRALIX understands:
  - Resource changes before they happen
  - Cost impact of each change (AWS/GCP pricing API)
  - Security implications (public S3 bucket? open security group?)
- **Kubernetes manifest validation:** Native YAML parser checks:
  - Resource limits set?
  - Liveness/readiness probes present?
  - Security contexts (runAsNonRoot, readOnlyRootFilesystem)?
  - Network policies defined?
- **Dockerfile optimization:**
  - Multi-stage build recommendations
  - Layer caching analysis
  - Image size reduction suggestions
  - Security scanning (no `latest` tags, no secrets in ENV)
- **Drift detection:** `dipralix infra drift` compares `terraform state` vs actual cloud state. Detects manual console changes.
- **Cost estimation:** `dipralix infra cost` shows monthly cost impact of pending changes before `terraform apply`.
- **Safety integration:** Destructive infra changes (`terraform destroy`, `kubectl delete -f`) trigger `Confirm` safety level.

**Why it's better than Shipyard:**
- Shipyard is a plugin requiring external setup. DIPRALIX Infra Awareness is native.
- Shipyard uses generic validation. DIPRALIX understands your specific `.tf` and `.yaml` files.
- Shipyard has no cost estimation. DIPRALIX does.
- Shipyard is Claude-only. DIPRALIX works with any model.

**Commands:**
```bash
/infra plan                    # Parse terraform plan + show impact
/infra cost                    # Estimate monthly cost of changes
/infra drift                   # Detect configuration drift
/infra security                # Scan for infra security issues
/infra optimize                # Suggest Dockerfile/K8s optimizations
```

---

### 8. APPROVAL MATRIX
**Inspired by:** Cline Approval-First Workflow  
**Community Pain:** Cline's approval-first model is loved by teams but requires a VS Code extension. Claude Code's auto-apply is dangerous. DIPRALIX needs granular, native approval.

**DIPRALIX v2 Native Implementation:**

```rust
// src/approval_matrix.rs
pub struct ApprovalMatrix {
    action_classifier: ActionClassifier,
    per_action_policy: HashMap<ActionType, ApprovalLevel>,
    project_policy: ProjectPolicy,
    team_policy: TeamPolicy,
}

pub enum ApprovalLevel {
    Auto,       // Execute without asking
    Notify,     // Execute but show in log
    Confirm,    // Pause, show diff, wait for Y/n
    Deny,       // Block entirely
}
```

**What it does:**
- **Granular per-action approval:** Not all-or-nothing. Each action type has its own level:
  ```toml
  # .dipralix/approval.toml
  [actions]
  read_file = "Auto"           # Safe, no risk
  write_file = "Notify"        # Show what was written
  edit_file = "Confirm"        # Show diff, ask for approval
  bash = "Confirm"             # Shell commands need review
  bash_rm = "Deny"             # Never allow rm without override
  bash_git_push = "Deny"       # Never auto-push
  bash_docker_run = "Confirm"  # Containers need review
  bash_curl = "Confirm"        # Network calls need review
  ```
- **Risk-aware escalation:** If a single session contains 3+ `edit_file` actions on critical files (e.g., `auth.rs`, `payment.rs`), escalate to `Confirm` even if default is `Auto`.
- **Team policy sync:** `.dipralix/approval.toml` is git-tracked. Junior devs inherit senior dev policies. Enterprise admins can enforce `Deny` on `bash_sudo`.
- **Speed mode:** `/speed fast` temporarily sets everything to `Auto` (with undo protection). `/speed safe` sets everything to `Confirm`.
- **Undo integration:** Every `Confirm` action creates a snapshot before execution. `/undo` reverts instantly.

**Why it's better than Cline:**
- Cline is IDE-only. DIPRALIX Approval Matrix works in terminal + IDE + background.
- Cline has binary approve/reject. DIPRALIX has 4 levels (Auto, Notify, Confirm, Deny).
- Cline doesn't have per-action granularity. DIPRALIX does.
- Cline has no team policy sync. DIPRALIX does via `.dipralix/approval.toml`.

**Commands:**
```bash
/approval                      # Show current approval matrix
/approval set bash rm Deny     # Set specific action level
/approval speed fast           # Fast mode (Auto)
/approval speed safe           # Safe mode (Confirm)
/approval team sync            # Sync team policy from git
```

---

### 9. LIVING DOCUMENTATION
**Inspired by:** MemClaw Living README + Kiro Spec-Driven Development  
**Community Pain:** Documentation is always out of date. MemClaw updates READMEs but requires an external service. Kiro generates specs but doesn't sync them with code changes.

**DIPRALIX v2 Native Implementation:**

```rust
// src/living_docs.rs
pub struct LivingDocs {
    architecture_doc: ArchitectureDoc,
    api_doc: ApiDoc,
    changelog: Changelog,
    spec_generator: SpecGenerator,
    sync_engine: SyncEngine,
}
```

**What it does:**
- **Auto-maintained `ARCHITECTURE.md`:**
  - When you add a new module, DIPRALIX updates the architecture diagram (Mermaid ASCII in markdown).
  - When you change a dependency, it updates the tech stack section.
  - When you refactor, it updates the data flow description.
  - Trigger: `dipralix docs sync` or auto-on-commit.
- **Auto-maintained `API.md`:**
  - Parses HTTP handlers from source code (Axum, Actix, Express, FastAPI).
  - Extracts route paths, methods, request/response types.
  - Updates `API.md` with endpoint documentation.
  - Detects breaking changes: "Route `/users/:id` changed from GET to POST — breaking change flagged."
- **Auto-maintained `CHANGELOG.md`:**
  - Not just git log. DIPRALIX analyzes commits to categorize:
    - `feat:` → Added section
    - `fix:` → Fixed section
    - `refactor:` → Changed section
    - `BREAKING:` → Breaking Changes section
  - Links to relevant PRs and issues.
- **Spec-driven development bridge:**
  - Write `docs/specs/oauth2.md`: "We need OAuth2 with JWT refresh tokens."
  - `dipralix spec implement oauth2` → DIPRALIX generates tasks from spec, implements them, updates spec with completion status.
  - Spec becomes living document: sections marked ✅ as completed.

**Why it's better than MemClaw + Kiro:**
- MemClaw requires external API. DIPRALIX Living Docs is local file operations.
- Kiro is spec-only. DIPRALIX bridges spec → code → updated docs.
- Neither detects breaking API changes. DIPRALIX does via AST analysis.

**Commands:**
```bash
/docs sync                     # Sync all docs with current code
/docs architecture             # Regenerate ARCHITECTURE.md
/docs api                      # Regenerate API.md
/docs changelog                # Regenerate CHANGELOG.md
/spec list                     # List all specs in docs/specs/
/spec implement oauth2         # Implement spec from docs/specs/oauth2.md
```

---

### 10. BROWSER ENGINE
**Inspired by:** Dev-Browser + Firecrawl  
**Community Pain:** Firecrawl is an external API ($$$). Dev-Browser is a Claude Code plugin. Both require setup. DIPRALIX needs native web interaction for research, testing, and documentation.

**DIPRALIX v2 Native Implementation:**

```rust
// src/browser_engine.rs
pub struct BrowserEngine {
    headless_chrome: HeadlessChrome,   // Rust headless_chrome crate
    markdown_extractor: Extractor,
    screenshot_comparator: Comparator,
    form_interactor: FormInteractor,
    session_cache: SessionCache,
}
```

**What it does:**
- **Embedded headless Chromium:** Not an external API. Chromium embedded via Rust `headless_chrome` or `chromiumoxide` crate. Zero external dependencies.
- **JS rendering + interaction:**
  - `dipralix web https://docs.stripe.com` → Renders page, extracts clean markdown.
  - Can fill forms, click buttons, navigate flows.
  - Useful for: "Test the signup flow on our staging site."
- **Screenshot comparison:**
  - `dipralix web screenshot --compare` → Takes screenshot, compares with baseline.
  - Detects UI regressions: "Button color changed from blue to red."
  - Useful for frontend testing without Playwright setup.
- **Clean markdown extraction:**
  - Strips nav, ads, footers. Extracts article content only.
  - Better than raw HTML for LLM context.
  - Caches results locally: `~/.dipralix/cache/web/`.
- **Research mode:**
  - `dipralix web research "best Rust auth libraries 2026"` → Searches web, visits top 5 results, extracts content, synthesizes comparison table.
  - No external search API needed. Uses DuckDuckGo or SearX scraping.
- **Session persistence:** Logged-in sessions cached. "Check my GitHub notifications" works without re-auth.

**Why it's better than Firecrawl + Dev-Browser:**
- Firecrawl costs API credits. DIPRALIX Browser Engine is free (local Chromium).
- Firecrawl is external — your data leaves your machine. DIPRALIX is local.
- Dev-Browser is a Claude plugin. DIPRALIX is native.
- Neither has screenshot comparison. DIPRALIX does.

**Commands:**
```bash
/web https://docs.rs/axum      # Fetch and extract docs
/web research "Rust auth"      # Multi-site research synthesis
/web screenshot --compare      # UI regression detection
/web test https://staging...   # Interactive flow testing
/web cache clear               # Clear web cache
```

---

## IMPLEMENTATION PRIORITY

### Phase 2A: Core Intelligence (Q4 2026)
1. **Memory Core** — Highest impact, lowest effort. Just file operations.
2. **Lazy Context** — Reduces API costs by 60%. Immediate ROI.
3. **Peer Review Engine** — Differentiator. No competitor has native debate.

### Phase 2B: Developer Experience (Q1 2027)
4. **Code Fingerprinting** — Onboarding friction killer.
5. **Comment Protocol** — Bridges IDE and terminal workflows.
6. **Plan Visualizer** — Makes complex tasks manageable.

### Phase 2C: Power User Features (Q1 2027)
7. **Approval Matrix** — Safety + speed balance.
8. **Living Documentation** — Reduces documentation debt.
9. **Infra Awareness** — DevOps-native coding.
10. **Browser Engine** — Research + testing without external APIs.

---

## WHY THIS WINS

| Competitor | Their Plugin Ecosystem | DIPRALIX v2 Approach |
|------------|----------------------|-------------------|
| **Claude Code** | 9,000+ plugins. MemClaw, Plannotator, Ralph Wiggum, Shipyard, Dev-Browser, Firecrawl, Agent-Peer-Review — all external, all cost extra, all require setup. | All 10 features native. One binary. Zero setup. |
| **Cursor** | Extension marketplace. No terminal power. No multi-model. | Terminal + IDE + native features. Multi-model. |
| **OpenCode** | MCP servers. Node.js. Slower. | No MCP needed. Rust-native. Faster. |
| **Aider** | IDE plugins (3rd party). No orchestration. | Native IDE bridge + orchestration. |
| **Cline** | VS Code only. Approval-first but limited. | All surfaces + 4-level approval matrix. |
| **Pi** | Lazy-loading but no ecosystem. | Lazy-loading + full ecosystem. |
| **Caliber** | Separate CLI tool. | `dipralix init` — built-in. |
| **Kiro** | Spec-driven but no code sync. | Spec → Code → Docs — full loop. |

**The Pitch:**
> "Claude Code needs 9 plugins to do what DIPRALIX does out of the box. And those plugins cost money, break on updates, and leak your data to external APIs. DIPRALIX v2 is one 15MB binary. No plugins. No MCPs. No skills. No subscriptions. Just everything you need, built in Rust, running at wire speed."

---

## METRICS FOR SUCCESS

| Feature | Success Metric | Target Date |
|---------|---------------|-------------|
| Memory Core | Session restore time | <8 seconds |
| Lazy Context | Token reduction per request | 60% |
| Peer Review | Bug catch rate | +25% vs single model |
| Code Fingerprinting | `dipralix init` accuracy | 95% stack detection |
| Comment Protocol | Tasks detected from comments | 100% of `// DIPRALIX:` |
| Plan Visualizer | Plan comprehension speed | 3× faster than text |
| Approval Matrix | Accidental destructive ops | 0 |
| Living Docs | Doc freshness score | 90%+ auto-synced |
| Infra Awareness | Pre-apply cost accuracy | 95% |
| Browser Engine | Research task completion | 100% local, 0 API cost |

---

*Document Version: 2026.05.30 v2*
*Classification: Open Source — MIT License*
*Philosophy: "The best plugin is no plugin."*
"*
