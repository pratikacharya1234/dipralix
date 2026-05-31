//! Dipralix stats backend.
//!
//! Single endpoint `GET /api/stats` that returns live numbers:
//!
//! ```json
//! {
//!   "stars":     12,
//!   "forks":      2,
//!   "downloads": 45,
//!   "visits":   281,
//!   "as_of":    "2026-05-31T18:42:00Z"
//! }
//! ```
//!
//! - GitHub numbers come from the public REST API and are cached for 60s
//!   per metric to stay well inside GitHub's 60-req/hour unauthenticated limit.
//! - The `visits` counter is an in-memory `AtomicU64` that increments on every
//!   successful response. It resets when the container restarts — that's a
//!   feature for v0.1.0 (no DB to operate). Swap to SQLite when the number
//!   stops fitting in a `u64` (i.e. never).

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, Method, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use parking_lot::Mutex;
use serde::Serialize;
use tower_http::cors::CorsLayer;

const REPO_OWNER: &str = "pratikacharya1234";
const REPO_NAME: &str = "dipralix";
const CACHE_TTL: Duration = Duration::from_secs(60);
const USER_AGENT: &str = "dipralix-stats/0.1.0 (+https://github.com/pratikacharya1234/dipralix)";

#[derive(Clone, Default, Serialize)]
struct GitHubSnapshot {
    stars: u64,
    forks: u64,
    downloads: u64,
}

struct CachedSnapshot {
    value: GitHubSnapshot,
    fetched_at: Instant,
}

#[derive(Clone)]
struct AppState {
    http: reqwest::Client,
    cache: Arc<Mutex<Option<CachedSnapshot>>>,
    visits: Arc<AtomicU64>,
}

#[derive(Serialize)]
struct StatsResponse {
    stars: u64,
    forks: u64,
    downloads: u64,
    visits: u64,
    as_of: String,
    repo: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,axum=info,tower_http=info".into()),
        )
        .init();

    let port: u16 = env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
    let bind = SocketAddr::from(([0, 0, 0, 0], port));

    let state = AppState {
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent(USER_AGENT)
            .build()?,
        cache: Arc::new(Mutex::new(None)),
        visits: Arc::new(AtomicU64::new(0)),
    };

    // CORS — permissive for GET. Locking down to a specific origin later is one line.
    let cors = CorsLayer::new()
        .allow_origin(env::var("CORS_ORIGIN").ok()
            .and_then(|s| s.parse::<HeaderValue>().ok())
            .unwrap_or_else(|| HeaderValue::from_static("*")))
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/api/stats", get(stats))
        .route("/healthz", get(|| async { (StatusCode::OK, "ok") }))
        .route("/", get(|| async { (StatusCode::OK, "dipralix-stats — see /api/stats") }))
        .with_state(state)
        .layer(cors);

    tracing::info!(%bind, "dipralix-stats listening");
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    // Try cache first.
    let cached = {
        let guard = state.cache.lock();
        guard.as_ref().and_then(|c| {
            if c.fetched_at.elapsed() < CACHE_TTL {
                Some(c.value.clone())
            } else {
                None
            }
        })
    };

    let snapshot = match cached {
        Some(v) => v,
        None => match fetch_github(&state.http).await {
            Ok(snap) => {
                let mut guard = state.cache.lock();
                *guard = Some(CachedSnapshot { value: snap.clone(), fetched_at: Instant::now() });
                snap
            }
            Err(e) => {
                tracing::warn!(error = ?e, "github fetch failed; serving stale or zero");
                let guard = state.cache.lock();
                guard.as_ref().map(|c| c.value.clone()).unwrap_or_default()
            }
        },
    };

    let visits = state.visits.fetch_add(1, Ordering::Relaxed) + 1;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let as_of = format_rfc3339(now);

    Json(StatsResponse {
        stars: snapshot.stars,
        forks: snapshot.forks,
        downloads: snapshot.downloads,
        visits,
        as_of,
        repo: "pratikacharya1234/dipralix",
    })
}

async fn fetch_github(http: &reqwest::Client) -> anyhow::Result<GitHubSnapshot> {
    #[derive(serde::Deserialize)]
    struct Repo {
        stargazers_count: u64,
        forks_count: u64,
    }
    #[derive(serde::Deserialize)]
    struct Release {
        assets: Vec<Asset>,
    }
    #[derive(serde::Deserialize)]
    struct Asset {
        download_count: u64,
    }

    let repo_url = format!("https://api.github.com/repos/{}/{}", REPO_OWNER, REPO_NAME);
    let releases_url = format!("https://api.github.com/repos/{}/{}/releases", REPO_OWNER, REPO_NAME);

    let (repo_res, releases_res) = tokio::join!(
        http.get(&repo_url).send(),
        http.get(&releases_url).send(),
    );

    let repo: Repo = repo_res?.error_for_status()?.json().await?;
    let releases: Vec<Release> = releases_res?.error_for_status()?.json().await?;

    let downloads = releases.iter()
        .flat_map(|r| r.assets.iter())
        .map(|a| a.download_count)
        .sum();

    Ok(GitHubSnapshot {
        stars: repo.stargazers_count,
        forks: repo.forks_count,
        downloads,
    })
}

/// Minimal RFC 3339 formatter — avoids pulling in `chrono` for a single string.
fn format_rfc3339(unix_secs: u64) -> String {
    // SAFETY-ish: we accept whatever the system clock says; epoch is reasonable.
    let (year, month, day, hour, minute, second) = epoch_to_civil(unix_secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hour, minute, second)
}

fn epoch_to_civil(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    // Howard Hinnant's date algorithm — public domain, faithful integer port.
    let days = (secs / 86_400) as i64;
    let secs_of_day = secs % 86_400;
    let hour = (secs_of_day / 3600) as u32;
    let minute = ((secs_of_day % 3600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;

    let z = days + 719_468;
    let era = if z >= 0 { z / 146_097 } else { (z - 146_096) / 146_097 };
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (year, m as u32, d as u32, hour, minute, second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_is_1970() {
        assert_eq!(epoch_to_civil(0), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn known_epoch_is_correct() {
        // 1735689600 = 2025-01-01 00:00:00 UTC
        assert_eq!(epoch_to_civil(1_735_689_600), (2025, 1, 1, 0, 0, 0));
    }

    #[test]
    fn format_known_date() {
        assert_eq!(format_rfc3339(1_735_689_600), "2025-01-01T00:00:00Z");
    }
}
