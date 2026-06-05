// ══════════════════════════════════════════════════════════════════════════════
// FORGE ◈ Domain Bootstrap — Project Type Selector + Real-Time Research
//
// Before coding begins, FORGE asks WHAT you're building. It loads the
// embedded domain blueprint AND searches the web for the latest tech
// stack, architecture, and security recommendations — so the agent
// starts with the freshest context, not stale guesses.
// ══════════════════════════════════════════════════════════════════════════════

use crate::ui::nullvoid::{
    thin_rule_stdout, AMBER, BRIGHT, FIRE, I_ACTIVE, I_ADD, I_ERROR, I_MARK, I_OUT, I_PROMPT,
    I_STREAM, I_TARGET, I_WARN, MINT, MUTED, PLASMA, RESET, TEXT, VIOLET,
};
use std::io::Write;
use std::time::{Duration, Instant};

// ══════════════════════════════════════════════════════════════════════════════
// Domain Blueprint
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
pub struct DomainBlueprint {
    pub name: &'static str,
    pub category: &'static str,
    pub tech_stack: &'static str,
    pub architecture: &'static str,
    pub security: &'static str,
    pub best_practices: &'static str,
    pub testing: &'static str,
    pub deployment: &'static str,
    pub key_terms: &'static str,
    pub search_query: &'static str,
}

// ══════════════════════════════════════════════════════════════════════════════
// Embedded Domain Blueprints (15 domains)
// ══════════════════════════════════════════════════════════════════════════════

const DOMAINS: &[DomainBlueprint] = &[
    // ── [1] Mobile ──────────────────────────────────────────────────────────
    DomainBlueprint {
        name: "Mobile App",
        category: "mobile",
        tech_stack: "React Native 0.82+, Flutter 4.0+, Swift 6/SwiftUI, Kotlin 2.1/Jetpack Compose, Expo SDK 54. Zustand/Riverpod 3 for state.",
        architecture: "MVVM + Clean Architecture. Unidirectional data flow. Repository pattern. Feature-first modules. Offline-first with sync.",
        security: "OWASP Mobile Top 10. Certificate pinning. Biometric auth. Secure enclave. Encrypted local storage. Runtime integrity checks.",
        best_practices: "Lazy loading. Image caching. Debounced search. Deep linking. Accessibility. CI/CD with fastlane/EAS Build.",
        testing: "Jest + RNTL for components. Detox/Maestro for E2E. XCTest/JUnit. Snapshot testing.",
        deployment: "Fastlane for App Store/Play Store. EAS Build. TestFlight. CodePush for OTA. Staged rollouts.",
        key_terms: "mobile iOS Android React Native Flutter Swift Kotlin cross-platform",
        search_query: "best tech stack mobile app development 2026 latest architecture security",
    },
    // ── [2] Web ─────────────────────────────────────────────────────────────
    DomainBlueprint {
        name: "Website / Web App",
        category: "web",
        tech_stack: "Next.js 15+ (App Router) or SvelteKit 5 or Astro 5. React 19/RSC, Svelte 5 runes. TypeScript 5.7+. Tailwind CSS 4. Drizzle/Prisma.",
        architecture: "RSC + Client Components boundary. Streaming SSR. Island Architecture. Edge middleware. Optimistic UI. CQRS for complex domains.",
        security: "OWASP Top 10 2024. CSP headers. CORS whitelist. CSRF tokens. SQL injection prevention. Rate limiting at edge. Helmet.js.",
        best_practices: "Core Web Vitals (LCP<2.5s). Image optimization (WebP/AVIF). Bundle splitting. ISR. Edge caching. WCAG 2.2 AA.",
        testing: "Vitest + Testing Library. Playwright for E2E. Storybook. Lighthouse CI. MSW for API mocking.",
        deployment: "Vercel/Netlify for frontend. Railway/Render for backend. Docker multi-stage. Feature flags. Blue-green deploy.",
        key_terms: "web React Next.js Svelte Vue TypeScript fullstack frontend backend SSR edge",
        search_query: "best web development tech stack 2026 latest framework architecture patterns",
    },
    // ── [3] AI / ML Model ───────────────────────────────────────────────────
    DomainBlueprint {
        name: "AI / ML Model",
        category: "ai",
        tech_stack: "PyTorch 2.5+ with torch.compile. HuggingFace Transformers/Diffusers/TRL. LangChain/LlamaIndex. vLLM/TGI. ONNX Runtime. Qdrant/Pinecone.",
        architecture: "RAG: chunk→embed→retrieve→rerank→generate. LoRA/QLoRA fine-tuning. GPTQ/AWQ quantization. FSDP/DeepSpeed ZeRO-3 distributed training.",
        security: "Model poisoning prevention. Prompt injection guards. PII redaction. Differential privacy. Output safety classifiers. Signed model artifacts.",
        best_practices: "Start with pre-trained. bfloat16 training. Flash Attention 2. Data versioning (DVC). A/B test models. Monitor drift (evidently).",
        testing: "lm-eval-harness, DeepEval, Ragas. CheckList behavioral testing. Red-teaming for safety.",
        deployment: "Docker + NVIDIA CTK. Triton/vLLM serving. Model quantization. Auto-scaling. GPU MIG/MPS sharing.",
        key_terms: "AI ML PyTorch transformers LLM RAG LoRA fine-tuning inference embedding vector DB",
        search_query: "best AI ML development stack 2026 latest LLM RAG deployment architecture",
    },
    // ── [4] Deep Learning ────────────────────────────────────────────────────
    DomainBlueprint {
        name: "Deep Learning",
        category: "deeplearning",
        tech_stack: "PyTorch 2.5+ / JAX. CUDA 12.4+/cuDNN 9. OpenCV. NVIDIA DALI. HuggingFace ecosystem. W&B for tracking.",
        architecture: "CNNs (ConvNeXt, EfficientNet). ViTs (DINOv2). GANs (StyleGAN3). Diffusion (Stable Diffusion 3, FLUX). YOLOv10/RT-DETR. NeRF, Gaussian Splatting. Whisper, EnCodec.",
        security: "Adversarial robustness (PGD, FGSM). Model watermarking. Membership inference protection. Secure aggregation for federated learning.",
        best_practices: "Mixed precision (AMP). Gradient accumulation. Cosine LR scheduling. RandAugment/MixUp/CutMix. Early stopping. Profile before optimizing.",
        testing: "Cross-validation. Per-class metrics. Confusion matrix. IoU/mAP. PSNR/SSIM. FID for generation.",
        deployment: "TorchScript/torch.compile. TensorRT. ONNX Runtime. CoreML. TFLite. Triton Inference Server.",
        key_terms: "deep learning CNN transformer GAN diffusion neural network PyTorch CUDA training vision",
        search_query: "best deep learning framework 2026 latest computer vision NLP architecture training",
    },
    // ── [5] Desktop Software ─────────────────────────────────────────────────
    DomainBlueprint {
        name: "Desktop Software",
        category: "desktop",
        tech_stack: "Tauri 2 (Rust+web) or Electron 33. Rust egui/iced. SwiftUI for macOS. .NET 9 MAUI/WPF. Flutter Desktop. GTK4/Qt6.",
        architecture: "Backend-for-frontend pattern. IPC via Tauri commands. State: Svelte stores/Zustand. File system APIs. System tray. Auto-updater.",
        security: "Code signing (Apple notarization, Windows Authenticode). Sandbox entitlements. CSP for webview. IPC validation. Keychain/Credential Manager.",
        best_practices: "Native feel over web feel. OS conventions. DPI scaling. Accessibility. Small bundle (<5MB Tauri). Delta patch updates.",
        testing: "Playwright for webview E2E. Rust unit tests. Platform-specific CI. Manual testing on target OS.",
        deployment: "GitHub Actions for all platforms. .dmg/.msi/.AppImage/.deb. Apple notarization. Microsoft Store. Flathub.",
        key_terms: "desktop Tauri Electron Rust SwiftUI native GUI cross-platform app signed",
        search_query: "best desktop app framework 2026 Tauri Electron comparison cross-platform",
    },
    // ── [6] Hardware / IoT / Embedded ────────────────────────────────────────
    DomainBlueprint {
        name: "Hardware / IoT",
        category: "hardware",
        tech_stack: "Embedded Rust (embassy, rtic) or C (FreeRTOS, Zephyr). ESP-IDF for ESP32. RP2040/RP2350 SDK. STM32Cube. MQTT/CoAP.",
        architecture: "Sensor→MCU→Edge→Cloud pipeline. RTOS scheduling. Interrupt-driven I/O + DMA. Power management. OTA with A/B partitioning. Thread/Zigbee/BLE mesh.",
        security: "Secure boot. Firmware signing. Encrypted flash. JTAG/SWD lock. TLS/DTLS. PSK/cert auth. Side-channel mitigation. Tamper detection. Secure element (ATECC608).",
        best_practices: "Battery profiling — every µA counts. Watchdog always on. Brownout detection. ESD protection. FCC/CE certification. RoHS. HIL testing.",
        testing: "Host unit tests (no_std). HIL with real hardware. Logic analyzer. Oscilloscope. Power profiler. Device farm CI.",
        deployment: "Factory flashing. Fleet management (Balena, AWS IoT). OTA with canary. Device decommissioning.",
        key_terms: "IoT embedded hardware ESP32 STM32 ARM RTOS firmware sensor BLE MQTT OTA",
        search_query: "best IoT embedded development 2026 latest hardware framework security practices",
    },
    // ── [7] Game Development ─────────────────────────────────────────────────
    DomainBlueprint {
        name: "Game Development",
        category: "gamedev",
        tech_stack: "Bevy 0.15 (Rust ECS) or Godot 4.4 or Unity 6 or Unreal 5.5. Raylib/Macroquad for 2D. WGPU for custom. FMOD/Wwise for audio.",
        architecture: "ECS for performance. Game loop: input→update→physics→render. Object pooling. LOD system. Occlusion culling. Async asset loading.",
        security: "Server-authoritative game state. Anti-cheat memory integrity. Save encryption. IAP receipt validation. Anti-tamper. GDPR/COPPA for kids.",
        best_practices: "Profile early (Tracy/puffin). Frame budget: 16ms@60fps. Texture atlasing. GPU instancing. Delta time. Rollback netcode.",
        testing: "Automated gameplay testing. Performance regression at target FPS. Platform cert (TRC/TCR). Multiplayer load testing.",
        deployment: "Steamworks SDK. Console dev kits. App Store/Play Store. Web export via WASM/WebGPU.",
        key_terms: "game dev Bevy Godot Unity Unreal ECS rendering shader physics multiplayer Steam",
        search_query: "best game development engine 2026 indie AAA latest graphics techniques",
    },
    // ── [8] DevOps / Infrastructure ──────────────────────────────────────────
    DomainBlueprint {
        name: "DevOps / Infrastructure",
        category: "devops",
        tech_stack: "Terraform 1.10+/OpenTofu. Kubernetes 1.32+ (k3s/GKE/EKS/AKS). Docker/Podman. Helm 3. GitHub Actions. ArgoCD/Flux. Prometheus+Grafana+Loki. OpenTelemetry.",
        architecture: "IaC — everything in git. Immutable infrastructure. GitOps: git as source of truth. Service mesh (Linkerd/Istio). Zero-trust networking.",
        security: "IAM least privilege. Short-lived credentials (Vault/OIDC). Network policies deny-all. Pod security. Image scanning (Trivy). SBOM. CIS benchmarks.",
        best_practices: "12-Factor App. Graceful shutdown. Circuit breakers. Chaos engineering. Cost optimization. Runbooks as code. Blameless postmortems.",
        testing: "Terratest for infra. k6/Artillery for load. Container structure tests. OPA/Rego policy tests. Chaos experiments in staging.",
        deployment: "Blue-green/canary. Feature flags. Automated rollback. Multi-region failover. DR drills.",
        key_terms: "DevOps Kubernetes Terraform Docker CI/CD GitOps observability infra cloud IaC SRE",
        search_query: "best DevOps infrastructure 2026 latest platform engineering GitOps observability",
    },
    // ── [9] Data Engineering ─────────────────────────────────────────────────
    DomainBlueprint {
        name: "Data Engineering",
        category: "data",
        tech_stack: "Apache Spark 4/PySpark. dbt 1.9+. Airflow 3/Dagster/Prefect. Kafka 3.8/Redpanda. Flink. Delta Lake/Iceberg/Hudi. DuckDB. Polars. Great Expectations/Soda.",
        architecture: "Medallion: Bronze→Silver→Gold. Kappa for streaming-first. CDC with Debezium. Data mesh: domain ownership. Schema registry. SCD Type 2.",
        security: "Column-level encryption. PII masking. Row-level security. Audit logging. Data lineage. GDPR/CCPA compliance. Encryption at rest and in transit.",
        best_practices: "Partition pruning. Z-ordering. Compaction for small files. Idempotent pipelines. Data contracts. Freshness/volume monitoring.",
        testing: "dbt tests. Great Expectations suites. Schema validation on ingest. Reconciliation checks. Pipeline integration tests.",
        deployment: "CI/CD for dbt. Blue-green for streaming. Resource isolation. Per-pipeline cost attribution.",
        key_terms: "data engineering Spark dbt Kafka Airflow Delta Lake pipeline ETL lakehouse streaming analytics",
        search_query: "best data engineering stack 2026 latest lakehouse streaming pipeline architecture",
    },
    // ── [10] Blockchain / Web3 ───────────────────────────────────────────────
    DomainBlueprint {
        name: "Blockchain / Web3",
        category: "blockchain",
        tech_stack: "Solidity 0.8.27+ for EVM. Rust for Solana (Anchor) or Polkadot (ink!). Foundry/Hardhat. ethers.js v6/viem. Wagmi v3. IPFS/Arweave for storage. The Graph for indexing.",
        architecture: "Smart contract patterns: proxy/delegate, diamond (EIP-2535), factory. Layer 2: Optimistic (OP Stack) and ZK rollups. Account abstraction (ERC-4337). Cross-chain bridges. Oracles (Chainlink).",
        security: "Reentrancy guards (CEI pattern). Integer overflow (Solidity 0.8+ built-in). Flash loan attack prevention. Access control (OpenZeppelin). Formal verification (Certora). Audits (Slither, Mythril). Timelock for admin. Multi-sig (Gnosis Safe). Bug bounties.",
        best_practices: "Gas optimization from day 1. Storage packing. Immutable variables. Events for off-chain indexing. Upgradeable contracts (UUPS). Test on fork (mainnet state). Fuzz testing (Foundry).",
        testing: "Foundry forge tests. Hardhat + Mocha. Echidna for fuzzing. Tenderly for simulation. Mainnet fork testing.",
        deployment: "Hardhat/Ignition deploy scripts. Multisig deployment. Etherscan/Polygonscan verification. Testnet→mainnet pipeline. Monitoring (Tenderly/OpenZeppelin Defender).",
        key_terms: "blockchain Web3 Solidity EVM smart contract DeFi NFT Solana Rust Foundry token ERC",
        search_query: "best blockchain Web3 development 2026 latest smart contract security DeFi architecture",
    },
    // ── [11] Cybersecurity ───────────────────────────────────────────────────
    DomainBlueprint {
        name: "Cybersecurity",
        category: "security",
        tech_stack: "Rust/Go for tooling. Python for automation (Scapy, impacket). Wireshark/tshark. Nmap/Masscan. Burp Suite API. Metasploit framework. Ghidra/IDA for RE. YARA for malware. ELK/Splunk for SIEM.",
        architecture: "Defense in depth. Zero trust architecture (ZTA). NIST CSF framework. MITRE ATT&CK mapping. SOC: detection→triage→investigation→response pipeline. Threat intelligence feeds (MISP). Honeypot networks.",
        security: "This IS the security domain. Assume everything is compromised. Principle of least privilege. Air-gapped analysis environments. Encrypted comms always. Operational security (opsec). Evidence handling chain of custody.",
        best_practices: "Threat modeling (STRIDE, MITRE). Red team / blue team exercises. Purple teaming. Continuous pen testing. Assume breach mentality. CTI-driven defense. Tabletop exercises.",
        testing: "Penetration testing methodology. Vulnerability scanning (Nessus, OpenVAS). DAST/SAST pipelines. Fuzzing (AFL++, libFuzzer). Red team ops. Bug bounty programs.",
        deployment: "CI/CD security gates. Container scanning (Trivy, Grype). Secrets detection (TruffleHog, Gitleaks). SAST/DAST in pipeline. Dependency scanning (dependabot, Snyk).",
        key_terms: "cybersecurity pentesting red team blue team threat hunting SOC SIEM malware reverse engineering CTI zero trust",
        search_query: "best cybersecurity tools 2026 latest threat detection response zero trust architecture",
    },
    // ── [12] CLI / Developer Tools ───────────────────────────────────────────
    DomainBlueprint {
        name: "CLI / Developer Tools",
        category: "cli",
        tech_stack: "Rust (clap 4 + anyhow + indicatif + console + dialoguer) or Go (cobra + bubbletea + lipgloss). Python (click/typer + rich). Cross-compilation: cross/cargo-zigbuild. Shell completions: clap_complete.",
        architecture: "Builder pattern for CLI construction. Subcommand routing. Config layering (defaults→config file→env→flags). Progress bars for long ops (indicatif). Terminal UI (ratatui/bubbletea).",
        security: "Input validation at CLI boundary. Shell injection prevention (avoid system(), use exec()). Config file permissions (0600 for secrets). Token/sensitive value masking in logs.",
        best_practices: "POSIX conventions (--flag, -f). --help always works. Exit codes (0=ok, 1=error, 2=usage). stdin/stdout pipeline compatible. --json for scripting. Colored output with --no-color flag. Man pages or --help docs.",
        testing: "assert_cmd + predicates for CLI testing. trycmd for snapshot testing. Integration tests for full workflows.",
        deployment: "cargo-dist / GoReleaser for cross-platform binaries. Homebrew/npm/pip/scoop packages. Shell completion generation. CI matrix for linux/mac/win.",
        key_terms: "CLI developer tools Rust Go Python command-line terminal TUI cross-platform distribution",
        search_query: "best CLI developer tools framework 2026 Rust Go Python terminal UI best practices",
    },
    // ── [13] API / Backend Service ────────────────────────────────────────────
    DomainBlueprint {
        name: "API / Backend Service",
        category: "api",
        tech_stack: "Rust (Axum/Actix-web + sqlx + tower), Go (chi/fiber + sqlc), TypeScript (Hono/Fastify + Drizzle). GraphQL via async-graphql/gqlgen. gRPC via tonic/connect. Redis for cache. RabbitMQ/Kafka for queues.",
        architecture: "Hexagonal/ports-and-adapters. CQRS + Event Sourcing for complex domains. API Gateway pattern. Backend-for-frontend (BFF). Saga pattern for distributed transactions. Idempotency keys.",
        security: "JWT with short expiry + refresh tokens. OAuth2/OIDC. Rate limiting (token bucket). API key rotation. Request validation (JSON Schema). SQL injection via parameterized queries. CORS strict origins. mTLS for service-to-service.",
        best_practices: "OpenAPI/Swagger spec-first. Structured logging (tracing/logrus/zap). Health check endpoints. Graceful shutdown. Connection pooling. Circuit breakers. Retry with exponential backoff. Bulkhead pattern.",
        testing: "Unit: handlers with mock services. Integration: testcontainers for DB/Redis. Contract: pact.io. Load: k6. Property-based: proptest/quickcheck.",
        deployment: "Docker multi-stage. Health/liveness probes. Horizontal pod autoscaling. Database migrations in CI/CD. API versioning strategy.",
        key_terms: "API backend REST GraphQL gRPC microservices Rust Go TypeScript Axum Fastify database",
        search_query: "best backend API framework 2026 Rust Go TypeScript microservices architecture patterns",
    },
    // ── [14] Scientific Computing / HPC ────────────────────────────────────────
    DomainBlueprint {
        name: "Scientific / HPC",
        category: "scientific",
        tech_stack: "Rust (nalgebra, ndarray, faer, burn), C++ (Eigen, Kokkos, HPX), Python (NumPy, SciPy, Numba, CuPy). MPI (rsmpi/mpi4py). CUDA/HIP for GPU. OpenMP for CPU parallelism. HDF5/NetCDF for data. LAPACK/BLAS via ndarray-linalg.",
        architecture: "SIMD vectorization (std::simd, packed_simd). Data parallelism via Rayon. Task-based: Rayon work-stealing. Distributed: MPI collectives. GPU offload: CUDA streams, overlapping compute+transfer. Mixed precision (f16/f32/f64).",
        security: "Reproducibility — lock dependency versions, pin RNG seeds. Input validation for scientific data formats. Memory safety for large allocations. Resource limits for HPC jobs.",
        best_practices: "Profile with perf/VTune/NSight before optimizing. Vectorize hot loops. Minimize allocations in inner loops. Use stack over heap where possible. Zero-copy deserialization (rkyv/flatbuffers). Checkpointing for long runs.",
        testing: "Property-based testing for numerical stability. Tolerance-aware assertions. Regression test against known results. Performance regression benchmarks (criterion).",
        deployment: "Singularity/Apptainer containers for HPC. Slurm job scripts. Module files (Lmod). MPI launcher integration. Paraview/VisIt for visualization.",
        key_terms: "HPC scientific computing MPI CUDA SIMD numerical linear algebra FFT CFD simulation parallelism",
        search_query: "best scientific computing HPC 2026 latest numerical libraries GPU acceleration Rust",
    },
    // ── [15] General ──────────────────────────────────────────────────────────
    DomainBlueprint {
        name: "General / Other",
        category: "general",
        tech_stack: "Auto-detect from project files and context.",
        architecture: "Analyze existing codebase and follow established patterns.",
        security: "OWASP Top 10 awareness. Follow language-specific security guides.",
        best_practices: "Follow existing project conventions. Read .forge/project.md.",
        testing: "Detect test framework from project config.",
        deployment: "Detect deployment config from project (Dockerfile, CI, etc.).",
        key_terms: "general multi-purpose auto-detect",
        search_query: "",
    },
];

// ══════════════════════════════════════════════════════════════════════════════
// Web Search — DuckDuckGo Lite (no API key needed)
// ══════════════════════════════════════════════════════════════════════════════

pub(crate) struct SearchResult {
    pub(crate) title: String,
    pub(crate) snippet: String,
    #[allow(dead_code)]
    url: String,
}

/// Search DuckDuckGo Lite for the latest domain info.
/// Returns results or empty vec if search fails/times out.
async fn search_domain(query: &str) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding(query));

    let client = match reqwest::Client::builder()
        .user_agent("FORGE/0.0.2 (domain-research)")
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let body = match resp.text().await {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };

    parse_ddg_lite(&body)
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "+")
        .replace('/', "%2F")
        .replace(':', "%3A")
        .replace('&', "%26")
        .replace('?', "%3F")
        .replace('=', "%3D")
}

fn parse_ddg_lite(html: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let mut lines = html.lines();

    while let Some(line) = lines.next() {
        // DDG Lite: result links look like:
        // <a rel="nofollow" class="result-link" href="URL">Title</a>
        // followed by <span class="result-snippet">Snippet</span>

        if line.contains("result-link") && line.contains("href=") {
            let url = extract_attr(line, "href=");
            let title = strip_html(line);

            // Next result-snippet
            let mut snippet = String::new();
            for next in lines.by_ref() {
                if next.contains("result-snippet") {
                    snippet = strip_html(next);
                    break;
                }
                if next.contains("result-link") {
                    break;
                }
            }

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResult {
                    title: truncate(&title, 80),
                    snippet: truncate(&snippet, 120),
                    url,
                });
            }

            if results.len() >= 6 {
                break;
            }
        }
    }

    results
}

fn extract_attr(html: &str, attr: &str) -> String {
    if let Some(start) = html.find(attr) {
        let after = &html[start + attr.len()..];
        let delim = after.chars().next().unwrap_or('"');
        let inner = if delim == '"' || delim == '\'' {
            &after[1..]
        } else {
            after
        };
        let end = if delim == '"' || delim == '\'' {
            inner.find(delim).unwrap_or(inner.len())
        } else {
            inner
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(inner.len())
        };
        inner[..end].to_string()
    } else {
        String::new()
    }
}

fn strip_html(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..s.floor_char_boundary(max)])
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Interactive Domain Selector (with real-time search + spinner)
// ══════════════════════════════════════════════════════════════════════════════

/// Returns the selected DomainBlueprint and optional search results.
pub async fn select_domain() -> (DomainBlueprint, Vec<SearchResult>) {
    println!();
    println!(
        " {viol}{mark} {bright}What are you building?{reset}  \
         {muted}(select domain — web search + embedded blueprint){reset}",
        viol = VIOLET,
        mark = I_MARK,
        bright = BRIGHT,
        reset = RESET,
        muted = MUTED
    );
    println!();
    println!(
        " {mint}[1]{reset} Mobile App         {mint}[2]{reset} Web App            {mint}[3]{reset} AI / ML Model",
        mint = MINT, reset = RESET
    );
    println!(
        " {mint}[4]{reset} Deep Learning      {mint}[5]{reset} Desktop Software    {mint}[6]{reset} Hardware / IoT",
        mint = MINT, reset = RESET
    );
    println!(
        " {mint}[7]{reset} Game Dev           {mint}[8]{reset} DevOps / Infra      {mint}[9]{reset} Data Engineering",
        mint = MINT, reset = RESET
    );
    println!(
        " {mint}[10]{reset} Blockchain/Web3    {mint}[11]{reset} Cybersecurity      {mint}[12]{reset} CLI / Dev Tools",
        mint = MINT, reset = RESET
    );
    println!(
        " {mint}[13]{reset} API / Backend      {mint}[14]{reset} Scientific / HPC   {mint}[15]{reset} General",
        mint = MINT, reset = RESET
    );
    println!(
        " {mint}[C]{reset} Custom domain      {muted}[Enter] = General{reset}",
        mint = MINT,
        muted = MUTED,
        reset = RESET
    );
    print!(
        " {plas}{prompt}{reset} ",
        plas = PLASMA,
        prompt = I_PROMPT,
        reset = RESET
    );
    let _ = std::io::stdout().flush();

    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
    let trimmed = buf.trim().to_lowercase();

    let domain = if trimmed == "c" {
        // ── Custom domain ──────────────────────────────────────────────────
        println!();
        println!(
            " {amber}{warn} {bright}Describe what you're building (1 line):{reset}",
            amber = AMBER,
            warn = I_WARN,
            bright = BRIGHT,
            reset = RESET
        );
        print!(
            " {plas}{prompt}{reset} ",
            plas = PLASMA,
            prompt = I_PROMPT,
            reset = RESET
        );
        let _ = std::io::stdout().flush();
        let mut desc = String::new();
        let _ = std::io::stdin().read_line(&mut desc);
        let desc = desc.trim().to_string();

        if desc.is_empty() {
            DOMAINS.last().unwrap().clone()
        } else {
            // Build a custom domain blueprint from user description
            let query = format!("best tech stack architecture {}", desc);
            DomainBlueprint {
                name: Box::leak(desc.clone().into_boxed_str()),
                category: "custom",
                tech_stack: Box::leak(format!("Researching: {}", desc).into_boxed_str()),
                architecture:
                    "Analyze requirements and propose architecture based on industry patterns.",
                security: "Apply OWASP Top 10 and domain-specific security best practices.",
                best_practices: "Research current best practices for this domain.",
                testing: "Use appropriate testing framework for the chosen tech stack.",
                deployment: "Follow modern CI/CD and deployment practices.",
                key_terms: Box::leak(desc.clone().into_boxed_str()),
                search_query: Box::leak(query.into_boxed_str()),
            }
        }
    } else {
        let idx: usize = match trimmed.as_str() {
            "1" => 0,
            "2" => 1,
            "3" => 2,
            "4" => 3,
            "5" => 4,
            "6" => 5,
            "7" => 6,
            "8" => 7,
            "9" => 8,
            "10" => 9,
            "11" => 10,
            "12" => 11,
            "13" => 12,
            "14" => 13,
            _ => 14, // default General
        };
        DOMAINS[idx].clone()
    };

    // ── Real-time web search with spinner ──────────────────────────────────
    print!(
        "\n {viol}{out} {bright}Researching {amber}{name}{bright} — searching web…{reset}  ",
        viol = VIOLET,
        out = I_OUT,
        bright = BRIGHT,
        amber = AMBER,
        name = domain.name,
        reset = RESET
    );
    let _ = std::io::stdout().flush();

    let search_start = Instant::now();
    let search_results = search_domain(domain.search_query).await;
    let elapsed = search_start.elapsed();

    // ── Display results ───────────────────────────────────────────────────
    println!();
    println!(
        " {muted}{stream}{stream}{stream} {text}{count} results in {elapsed:.1?}s{reset}",
        muted = MUTED,
        stream = I_STREAM,
        text = TEXT,
        count = search_results.len(),
        elapsed = elapsed,
        reset = RESET
    );

    if !search_results.is_empty() {
        println!(
            " {plas}┌── {bright}Latest from the web{reset}",
            plas = PLASMA,
            bright = BRIGHT,
            reset = RESET
        );
        for (i, r) in search_results.iter().enumerate() {
            println!(
                " {muted}│ {mint}{num}.{reset} {bright}{title}{reset}",
                muted = MUTED,
                mint = MINT,
                num = i + 1,
                bright = BRIGHT,
                title = r.title,
                reset = RESET
            );
            if !r.snippet.is_empty() {
                println!(
                    " {muted}│    {text}{snippet}{reset}",
                    muted = MUTED,
                    text = TEXT,
                    snippet = r.snippet,
                    reset = RESET
                );
            }
        }
        println!(
            " {muted}└{rule}{reset}",
            muted = MUTED,
            rule = "─".repeat(58),
            reset = RESET
        );
    } else if domain.category != "general" {
        println!(
            " {muted}{warn}  {text}Web search returned no results. Using embedded blueprint.{reset}",
            muted = MUTED, warn = I_WARN, text = TEXT, reset = RESET
        );
    }

    // ── Embedded blueprint ────────────────────────────────────────────────
    if domain.category != "general" {
        println!();
        println!(
            " {viol}{out} {bright}Embedded Blueprint — {amber}{name}{reset}",
            viol = VIOLET,
            out = I_OUT,
            bright = BRIGHT,
            amber = AMBER,
            name = domain.name,
            reset = RESET
        );
        thin_rule_stdout();
        println!(
            " {mint}{mark} Tech Stack:{reset}  {text}{stack}{reset}",
            mint = MINT,
            mark = I_MARK,
            reset = RESET,
            text = TEXT,
            stack = domain.tech_stack
        );
        println!(
            " {plas}{target} Architecture:{reset}  {text}{arch}{reset}",
            plas = PLASMA,
            target = I_TARGET,
            reset = RESET,
            text = TEXT,
            arch = domain.architecture
        );
        println!(
            " {fire}{err} Security:{reset}  {text}{sec}{reset}",
            fire = FIRE,
            err = I_ERROR,
            reset = RESET,
            text = TEXT,
            sec = domain.security
        );
        println!(
            " {amber}{add} Best Practices:{reset}  {text}{bp}{reset}",
            amber = AMBER,
            add = I_ADD,
            reset = RESET,
            text = TEXT,
            bp = domain.best_practices
        );
        thin_rule_stdout();
        println!(
            " {muted}{active} {text}Domain context injected. Agent will use \
             {amber}{name}{text} conventions.{reset}",
            muted = MUTED,
            active = I_ACTIVE,
            reset = RESET,
            text = TEXT,
            amber = AMBER,
            name = domain.name
        );
        println!();
    }

    (domain, search_results)
}

/// Resolve a domain by name/category (for --domain flag). Returns General if not found.
pub fn domain_by_name(name: &str) -> DomainBlueprint {
    let lower = name.to_lowercase();
    for d in DOMAINS {
        if d.category == lower || d.name.to_lowercase().contains(&lower) {
            return d.clone();
        }
    }
    DOMAINS.last().unwrap().clone()
}

// ══════════════════════════════════════════════════════════════════════════════
// Prompt builder
// ══════════════════════════════════════════════════════════════════════════════

pub fn domain_context(domain: &DomainBlueprint, search_results: &[SearchResult]) -> String {
    if domain.category == "general" {
        return String::new();
    }

    let mut ctx = format!(
        r#"## Project Domain: {name}

You are building a **{name}** project. Pre-load this domain knowledge:

### Recommended Tech Stack
{tech_stack}

### Architecture Patterns
{architecture}

### Security Requirements
{security}

### Best Practices
{best_practices}

### Testing Strategy
{testing}

### Deployment
{deployment}

### Key Concepts to Reference
{key_terms}
"#,
        name = domain.name,
        tech_stack = domain.tech_stack,
        architecture = domain.architecture,
        security = domain.security,
        best_practices = domain.best_practices,
        testing = domain.testing,
        deployment = domain.deployment,
        key_terms = domain.key_terms,
    );

    if !search_results.is_empty() {
        ctx.push_str("\n### Latest Web Research\n\n");
        ctx.push_str("Recent findings from a live web search for this domain:\n\n");
        for (i, r) in search_results.iter().enumerate() {
            ctx.push_str(&format!("{}. **{}** — {}\n", i + 1, r.title, r.snippet));
        }
        ctx.push_str(
            "\nIncorporate these latest findings where they improve upon the embedded blueprint.\n",
        );
    }

    ctx.push_str("\nUse this blueprint as your foundation. Prefer the recommended stack unless the user specifies otherwise. \
                   Apply the security requirements from day one — never defer security. \
                   Follow the architecture patterns for code organization and data flow.\n");

    ctx
}
