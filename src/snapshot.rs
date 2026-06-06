use std::sync::{Mutex, OnceLock};

struct Entry {
    path: String,
    old_content: Option<String>, // None = file did not exist (was created)
    description: String,
}

static STORE: OnceLock<Mutex<Vec<Entry>>> = OnceLock::new();

fn store() -> &'static Mutex<Vec<Entry>> {
    STORE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Save the current state of `path` before modifying it.
/// Call this before every write / edit / append.
pub fn capture(path: &str, description: &str) {
    let old = std::fs::read_to_string(path).ok(); // None if file doesn't exist yet
    store().lock().unwrap().push(Entry {
        path: path.to_string(),
        old_content: old,
        description: description.to_string(),
    });
}

/// Undo the last captured change. Returns a human-readable summary, or None if empty.
pub fn undo() -> Option<String> {
    let mut guard = store().lock().unwrap();
    let e = guard.pop()?;
    match &e.old_content {
        None => {
            // File was created by the agent — remove it
            let _ = std::fs::remove_file(&e.path);
            Some(format!("removed newly created '{}'", e.path))
        }
        Some(old) => {
            // Restore previous content
            let _ = std::fs::write(&e.path, old);
            Some(format!("restored '{}' ({})", e.path, e.description))
        }
    }
}

/// List all snapshots, most recent first (path, description).
pub fn list() -> Vec<(String, String)> {
    store()
        .lock()
        .unwrap()
        .iter()
        .rev()
        .map(|e| (e.path.clone(), e.description.clone()))
        .collect()
}

#[allow(dead_code)]
pub fn count() -> usize {
    store().lock().unwrap().len()
}
