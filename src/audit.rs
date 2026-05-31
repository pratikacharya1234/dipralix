use std::io::Write as _;

pub struct AuditEntry {
    pub timestamp: String,
    pub action:    String,
    pub detail:    String,
    pub success:   bool,
}

fn audit_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(".dipralix");
    let _ = std::fs::create_dir_all(dir);
    if dir.is_dir() {
        return Some(dir.join("audit.log"));
    }
    dirs::home_dir().map(|h| {
        let d = h.join(".dipralix");
        let _ = std::fs::create_dir_all(&d);
        d.join("audit.log")
    })
}

pub fn log(action: &str, detail: &str, success: bool) {
    let Some(path) = audit_path() else { return };
    let ts = chrono::Utc::now().to_rfc3339();
    let line = format!(
        "{{\"ts\":\"{}\",\"action\":\"{}\",\"detail\":{},\"ok\":{}}}",
        ts,
        action.replace('"', "'"),
        serde_json::Value::String(detail.chars().take(200).collect::<String>()),
        success
    );
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", line);
    }
}

pub fn tail(n: usize) -> Vec<AuditEntry> {
    let Some(path) = audit_path() else { return vec![] };
    let Ok(content) = std::fs::read_to_string(path) else { return vec![] };
    content
        .lines()
        .filter_map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).ok()?;
            Some(AuditEntry {
                timestamp: v["ts"].as_str().unwrap_or("?").to_string(),
                action:    v["action"].as_str().unwrap_or("?").to_string(),
                detail:    v["detail"].as_str().unwrap_or("").to_string(),
                success:   v["ok"].as_bool().unwrap_or(true),
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
