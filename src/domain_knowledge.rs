// Domain Knowledge Base — auto-injected expertise for every project type.
// FORGE detects the project domain and injects relevant best practices,
// folder structures, tech stack conventions, and domain-specific guidance.
// Updated: April 2026

/// Detect project domain from file structure and inject knowledge.
pub fn domain_guidance() -> String {
    let mut guidance = String::new();

    // ── Web: Next.js ─────────────────────────────────────────────────
    if has_file("next.config.js") || has_file("next.config.ts") || has_file("next.config.mjs") {
        guidance.push_str(
            r#"
## Next.js Project Conventions

**Folder Structure:**
```
src/
├── app/          # App Router (v13+)
│   ├── (auth)/   # Route groups
│   ├── api/      # API routes
│   └── layout.tsx
├── components/   # Reusable UI
│   ├── ui/       # Primitives (shadcn/ui)
│   └── features/ # Feature-specific
├── lib/          # Utilities, API clients
├── hooks/        # Custom hooks
├── types/        # TypeScript types
└── server/       # Server actions, DB queries
```

**Conventions:**
- Use Server Components by default. Add 'use client' only when needed.
- Data fetching: React Server Components > Server Actions > tRPC > REST
- Styling: Tailwind CSS 4 + shadcn/ui is the standard in 2026
- Auth: NextAuth v5 (Auth.js), Clerk, or Lucia
- DB: Prisma + PlanetScale/Neon (serverless Postgres)
- Deployment: Vercel (default), Cloudflare Pages, or Docker
- Use `revalidatePath` / `revalidateTag` for cache invalidation
- Metadata API for SEO, not next/head
- Route handlers for external webhooks
- Middleware for auth guards, geo redirects, A/B testing
- Bundle analysis: @next/bundle-analyzer
- Image optimization: next/image with remotePatterns
- Streaming: React Suspense boundaries + loading.tsx
"#,
        );
    }

    // ── React / Vite ─────────────────────────────────────────────────
    if has_file("vite.config.ts") || has_file("vite.config.js") {
        guidance.push_str(r#"
## React + Vite Conventions

**Stack (2026):**
- React 19 + Vite 6 + TypeScript
- State: Zustand (global), TanStack Query (server state), Jotai (atomic)
- Router: TanStack Router (type-safe) or React Router v7
- Forms: React Hook Form + Zod validation
- Styling: Tailwind CSS 4 + shadcn/ui or Radix UI primitives
- Testing: Vitest + React Testing Library + Playwright (E2E)
- Build: Vite with code splitting, lazy routes

**Pattern:** Feature-based folder structure. Co-locate tests. Use Suspense for code splitting. Prefer composition over inheritance.
"#);
    }

    // ── Svelte / SvelteKit ────────────────────────────────────────────
    if has_file("svelte.config.js") || has_file("vite.config.ts") && has_dir("src/routes") {
        guidance.push_str(r#"
## Svelte/SvelteKit Conventions

**Stack (2026):**
- Svelte 5 (runes: $state, $derived, $effect) + SvelteKit
- Styling: Tailwind CSS 4 or built-in scoped styles
- State: Svelte stores or runes ($state)
- Forms: Superforms + Zod
- Auth: Lucia Auth or Auth.js SvelteKit adapter
- DB: Drizzle ORM + Turso/SQLite
- Deployment: Vercel, Cloudflare Pages, or Node adapter

**Pattern:** Use $state runes over stores. +page.server.ts for load functions. Form actions for mutations. Prefer Svelte 5 snippets over slots.
"#);
    }

    // ── Python / Backend ──────────────────────────────────────────────
    if has_file("pyproject.toml") || has_file("requirements.txt") || has_file("setup.py") {
        guidance.push_str(r#"
## Python Project Conventions

**Stack (2026):**
- Package manager: uv (fastest) or Poetry
- Web: FastAPI (async) or Django 5
- Linting: ruff (replaces flake8, isort, pylint)
- Formatting: ruff format (replaces black)
- Type checking: mypy (strict mode)
- Testing: pytest + pytest-asyncio + pytest-cov
- Task runner: taskipy or just
- ASGI server: uvicorn (dev), granian (prod)
- ORM: SQLAlchemy 2.0 or Django ORM
- Data: Polars (fast) or Pandas

**Pattern:** Use pydantic v2 for validation. Async where possible. Type hints everywhere. .env files with pydantic-settings. Use context managers for resources. Structural pattern matching (match/case) for control flow.
"#);
    }

    // ── Rust ─────────────────────────────────────────────────────────
    if has_file("Cargo.toml") {
        guidance.push_str(r#"
## Rust Conventions

**Stack (2026):**
- Edition: 2024 (use impl Trait, async fn in traits)
- Async: tokio (multi-threaded)
- Web: axum 0.8 or actix-web 4
- Serialization: serde + serde_json
- CLI: clap 4 (derive API)
- Error handling: anyhow (app), thiserror (libraries)
- Logging: tracing (structured) + tracing-subscriber
- Testing: built-in #[test] + proptest for property testing
- DB: sqlx (async, compile-time checked)
- Linting: clippy (strict) + rustfmt

**Pattern:** Use Result<T, E> everywhere. Avoid unwrap() in library code. Prefer &str over String for parameters. Use derive macros. Module structure: lib.rs re-exports, modules by feature.
"#);
    }

    // ── Go ───────────────────────────────────────────────────────────
    if has_file("go.mod") {
        guidance.push_str(r#"
## Go Conventions

**Stack (2026):**
- Go 1.24+ with toolchain directive
- Web: net/http (stdlib) or chi router or fiber
- DB: sqlc (type-safe SQL) or GORM
- Config: envconfig or viper
- Testing: stdlib testing + testify
- Linting: golangci-lint
- Build: Makefile + Docker multi-stage

**Pattern:** Keep it simple. Stdlib first. Interfaces small (1-3 methods). Use context.Context everywhere. Error handling: if err != nil. Prefer composition over inheritance. Use generics sparingly. Package layout: domain-driven.
"#);
    }

    // ── Mobile: React Native / Expo ──────────────────────────────────
    if has_file("app.json") || has_file("expo") || has_dir("app") && has_file("package.json") {
        guidance.push_str(r#"
## React Native / Expo Conventions

**Stack (2026):**
- Expo SDK 53+ with Expo Router (file-based routing)
- UI: NativeWind (Tailwind for RN) or tamagui
- State: Zustand + TanStack Query
- Navigation: Expo Router (built on React Navigation)
- Forms: React Hook Form + Zod
- Storage: expo-secure-store + MMKV
- Notifications: expo-notifications
- Auth: Clerk Expo or Supabase Auth
- Backend: tRPC or GraphQL (use typed clients)
- EAS Build + Submit for CI/CD

**Pattern:** Use Expo Router file-based routing. Platform-specific files: `index.ios.tsx`. SafeAreaView/useSafeAreaInsets. useWindowDimensions for responsive. EAS Updates for OTA.
"#);
    }

    // ── AI / Deep Learning ───────────────────────────────────────────
    if has_file("*.ipynb") || has_dir("models") && has_file("requirements.txt") {
        guidance.push_str(r#"
## AI / Deep Learning Conventions

**Stack (2026):**
- Framework: PyTorch 2.5+ (torch.compile) or JAX
- LLMs: Hugging Face transformers + PEFT/LoRA
- Training: PyTorch Lightning or Hugging Face Trainer
- Serving: vLLM (LLMs), Triton Inference Server
- Vector DB: LanceDB (local) or Pinecone/Qdrant (cloud)
- Embeddings: sentence-transformers or OpenAI embeddings
- Evaluation: lm-evaluation-harness, DeepEval
- Data: HuggingFace datasets, Ray Data (scale)
- Experiment tracking: W&B or MLflow
- MLOps: Docker + FastAPI serving

**Pattern:** Use torch.compile() for 2x speed. LoRA/QLoRA for fine-tuning (never full fine-tune unless massive budget). Use mixed precision (bfloat16). DataLoader with prefetch_factor. Checkpoint with torch.save state_dict.
"#);
    }

    // ── Cybersecurity / Pentesting ────────────────────────────────────
    if has_file("*.pcap") || has_dir("exploits") || has_file("Dockerfile") && has_content("kali") {
        guidance.push_str(r#"
## Cybersecurity / Pentesting Conventions

**Stack (2026):**
- OS: Kali Linux or Parrot OS
- Recon: nmap, masscan, Amass, subfinder, Shodan CLI
- Web: Burp Suite, OWASP ZAP, ffuf, sqlmap, nuclei
- Exploitation: Metasploit, Cobalt Strike (red team), Sliver (C2)
- Network: Wireshark, tcpdump, Scapy, impacket
- Wireless: aircrack-ng, bettercap, Kismet
- Password: hashcat, john, hydra
- Forensics: Volatility 3, Autopsy, FTK Imager
- Cloud: ScoutSuite, Prowler, CloudSploit
- Reporting: Dradis, Ghostwriter, custom templates

**Pattern:** Always document chain of custody. Use OSCP/OSEP methodology. Enumerate before exploiting. Screenshots + timestamps. Clean up artifacts. Follow responsible disclosure. Use .env for credentials NEVER hardcode. Separate red team infrastructure from personal.
"#);
    }

    // ── DevOps / Infrastructure ──────────────────────────────────────
    if has_file("Dockerfile") || has_file("docker-compose.yml") || has_file("docker-compose.yaml") {
        guidance.push_str(r#"
## DevOps / Infrastructure Conventions

**Stack (2026):**
- Containers: Docker + Docker Compose (dev), Kubernetes (prod)
- CI/CD: GitHub Actions (default), GitLab CI, ArgoCD (GitOps)
- IaC: OpenTofu (Terraform fork) or Pulumi
- Config: Helm charts + Kustomize
- Secrets: HashiCorp Vault, SOPS, or cloud-native (AWS Secrets Manager)
- Monitoring: Prometheus + Grafana + Loki
- Logs: OpenTelemetry → Grafana/DataDog
- Service mesh: Linkerd or Istio
- CDN/Edge: Cloudflare Workers, Vercel Edge

**Pattern:** Multi-stage Docker builds (builder → runner). Non-root user. HEALTHCHECK. Use .dockerignore. Secrets via build args, never COPY .env. Use distroless or alpine images. Tag with git SHA.
"#);
    }

    // ── General Software Engineering ──────────────────────────────────
    guidance.push_str(
        r#"

## Universal Software Engineering Standards

**Always Follow:**
1. **Read before write** — never edit a file you haven't read
2. **Minimal changes** — fix only what's broken, don't refactor passing code
3. **Test after change** — run tests immediately after any modification
4. **Use existing patterns** — match the codebase style, don't impose your own
5. **Error handling** — every error path must be handled or explicitly documented
6. **No silent failures** — if something can fail, surface it
7. **Git hygiene** — atomic commits, descriptive messages, conventional commits
8. **Security first** — no secrets in code, no eval of user input, validate all inputs

**Code Review Checklist:**
- [ ] Does it compile/run?
- [ ] Are there tests?
- [ ] Is error handling complete?
- [ ] Are there security implications?
- [ ] Does it match existing patterns?
- [ ] Is it documented where non-obvious?
- [ ] Are there performance concerns?

**Modern Tech Principles (2026):**
- Type safety over runtime checks (TypeScript strict, Python mypy, Rust)
- Server-side first (server components, server actions, SSR)
- Edge computing where latency matters
- Local-first with sync (CRDTs, SQLite)
- AI-native architecture (embeddings, RAG, tool calling)
- Platform engineering over manual ops
- Observability built-in, not bolted on
"#,
    );

    guidance
}

fn has_file(name: &str) -> bool {
    std::path::Path::new(name).exists()
}

fn has_dir(name: &str) -> bool {
    std::path::Path::new(name).is_dir()
}

fn has_content(_pattern: &str) -> bool {
    false // Placeholder for future content-based detection
}
