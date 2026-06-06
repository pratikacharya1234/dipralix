use anyhow::Result;
use walkdir::WalkDir;

/// Result of loading a project tree into context.
pub struct LoadedProject {
    /// Number of source files included.
    pub file_count: usize,
    /// Rough token estimate (chars / 4).
    pub token_estimate: usize,
    /// Full concatenated content to inject into conversation.
    pub context_block: String,
}

// Extensions always skipped (binary / generated)
const SKIP_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "svg", "webp", "pdf", "zip", "tar", "gz", "bz2", "xz",
    "7z", "exe", "bin", "so", "dll", "dylib", "wasm", "lock", "sum", "min.js", "min.css",
];

// Directory names always skipped
const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    "__pycache__",
    ".next",
    "dist",
    "build",
    ".cache",
    "vendor",
    "coverage",
];

/// Load all readable source files under `root` into a single context block.
/// Limits total size to `max_chars` (default 800_000 ≈ 200K tokens).
pub fn load_project(root: &str, max_chars: Option<usize>) -> Result<LoadedProject> {
    let limit = max_chars.unwrap_or(800_000);

    // Warn before heavy loads
    if limit > 800_000 {
        eprintln!(
            "  [Warning] loading >{}K chars may exceed Gemini's context window",
            limit / 1000
        );
    }
    let mut files: Vec<(String, String)> = Vec::new(); // (path, content)
    let mut total_chars = 0usize;
    let mut skipped = 0usize;

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let path_str = path.to_string_lossy();

        // Skip noisy directories
        if SKIP_DIRS
            .iter()
            .any(|d| path_str.contains(&format!("/{}/", d)))
        {
            continue;
        }

        // Skip binary extensions
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if SKIP_EXTS.contains(&ext.as_str()) {
            continue;
        }

        // Skip large files (>100 KB)
        if let Ok(meta) = path.metadata() {
            if meta.len() > 102_400 {
                skipped += 1;
                continue;
            }
        }

        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        if content.is_empty() {
            continue;
        }

        let relative = pathdiff(root, &path_str);
        total_chars += content.len();
        files.push((relative, content));

        if total_chars >= limit {
            skipped += 1;
            break;
        }
    }

    let file_count = files.len();

    let mut context_block = String::with_capacity(total_chars + file_count * 80);
    context_block.push_str(&format!(
        "=== PROJECT CONTEXT ({} files loaded) ===\n\n",
        file_count
    ));

    for (path, content) in &files {
        context_block.push_str(&format!("--- {} ---\n{}\n\n", path, content));
    }

    if skipped > 0 {
        context_block.push_str(&format!(
            "[Note: {} file(s) skipped — too large or binary]\n",
            skipped
        ));
    }

    Ok(LoadedProject {
        file_count,
        token_estimate: context_block.len() / 4,
        context_block,
    })
}

/// Clone a remote git repository into a temp directory and load it.
pub async fn clone_and_load(url: &str) -> Result<LoadedProject> {
    let tmp = std::env::temp_dir().join(format!(
        "forge-learn-{}",
        url.rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git")
    ));

    // Remove stale clone
    if tmp.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
    }

    let out = tokio::process::Command::new("git")
        .args(["clone", "--depth=1", url, &tmp.to_string_lossy()])
        .output()
        .await?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("git clone failed: {}", err.trim());
    }

    load_project(&tmp.to_string_lossy(), None)
}

fn pathdiff(root: &str, path: &str) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .trim_start_matches('/')
        .to_string()
}
