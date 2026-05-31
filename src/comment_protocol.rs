use anyhow::Result;
use colored::Colorize;
use walkdir::WalkDir;

pub struct CommentTask {
    pub file: String,
    pub line_num: usize,
    pub description: String,
}

pub struct CommentProtocol;

impl CommentProtocol {
    pub fn scan_workspace() -> Result<Vec<CommentTask>> {
        let mut tasks = Vec::new();

        println!("  {} Scanning workspace for 'DIPRALIX:' tasks...", "→".dimmed());

        for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let s = path.to_string_lossy();

            if s.contains("/.") || s.contains("target/") || s.contains("node_modules/") {
                continue;
            }

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if matches!(ext_str.as_ref(), "rs" | "js" | "ts" | "py" | "go" | "yaml" | "yml" | "md" | "toml") {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            for (i, line) in content.lines().enumerate() {
                                // Skip DIPRALIX-DONE markers
                                if line.contains("DIPRALIX-DONE:") {
                                    continue;
                                }
                                if let Some(idx) = line.find("DIPRALIX:") {
                                    let description = line[idx + "DIPRALIX:".len()..].trim().to_string();
                                    tasks.push(CommentTask {
                                        file: s.to_string(),
                                        line_num: i + 1,
                                        description,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(tasks)
    }

    pub fn print_tasks(tasks: &[CommentTask]) {
        if tasks.is_empty() {
            println!("  {} No pending DIPRALIX tasks found.", "[OK]".green());
            return;
        }

        println!("\n  {} Pending Tasks:", "[TASKS]".cyan());
        for (i, task) in tasks.iter().enumerate() {
            println!("  {}. {} {}: {}",
                (i + 1).to_string().yellow(),
                task.file.dimmed(),
                format!("line {}", task.line_num).dimmed(),
                task.description.bright_white()
            );
        }
        println!("  Run {} to process a task.", "/tasks execute <num>".cyan());
    }

    /// Rewrite the source file so the DIPRALIX: marker becomes DIPRALIX-DONE: (dismissed)
    pub fn dismiss(task: &CommentTask) -> Result<()> {
        let content = std::fs::read_to_string(&task.file)?;
        let mut out = String::with_capacity(content.len());
        for (i, line) in content.lines().enumerate() {
            if i + 1 == task.line_num {
                out.push_str(&line.replacen("DIPRALIX:", "DIPRALIX-DONE:", 1));
            } else {
                out.push_str(line);
            }
            out.push('\n');
        }
        std::fs::write(&task.file, out)?;
        Ok(())
    }
}
