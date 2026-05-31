//! Infra Awareness — Dockerfile, Kubernetes YAML, and Terraform HCL static
//! analysis. Pure text scanning in v1 — no cloud API, no live pricing.

use colored::Colorize;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InfraKind {
    Dockerfile,
    K8sManifest,
    Terraform,
}

#[derive(Clone, Debug)]
pub struct InfraFile {
    pub path: PathBuf,
    pub kind: InfraKind,
}

#[derive(Clone, Debug)]
pub struct Finding {
    #[allow(dead_code)]
    pub file: PathBuf,
    pub severity: Severity,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warn,
    High,
}

fn sev_label(s: Severity) -> colored::ColoredString {
    match s {
        Severity::Info => "[info]".dimmed(),
        Severity::Warn => "[warn]".yellow(),
        Severity::High => "[high]".red().bold(),
    }
}

pub fn discover() -> Vec<InfraFile> {
    let mut out = Vec::new();
    for entry in WalkDir::new(".").max_depth(8).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        let s = p.to_string_lossy();
        if s.contains("/.git/") || s.contains("/target/") || s.contains("/node_modules/") {
            continue;
        }
        if !p.is_file() { continue; }

        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");

        let kind = if name == "Dockerfile" || name.starts_with("Dockerfile.") {
            Some(InfraKind::Dockerfile)
        } else if ext == "tf" || ext == "tfvars" {
            Some(InfraKind::Terraform)
        } else if (ext == "yaml" || ext == "yml") && looks_like_k8s(p) {
            Some(InfraKind::K8sManifest)
        } else {
            None
        };

        if let Some(k) = kind {
            out.push(InfraFile { path: p.to_path_buf(), kind: k });
        }
    }
    out
}

fn looks_like_k8s(p: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(p) else { return false };
    content.contains("apiVersion:") && content.contains("kind:")
}

pub fn analyze_file(f: &InfraFile) -> Vec<Finding> {
    let Ok(content) = std::fs::read_to_string(&f.path) else { return Vec::new() };
    match f.kind {
        InfraKind::Dockerfile => analyze_dockerfile(&f.path, &content),
        InfraKind::K8sManifest => analyze_k8s(&f.path, &content),
        InfraKind::Terraform => analyze_terraform(&f.path, &content),
    }
}

fn analyze_dockerfile(path: &Path, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lower = content.to_lowercase();

    // Latest tag
    for (i, line) in content.lines().enumerate() {
        let l = line.trim_start();
        if l.to_lowercase().starts_with("from ") && (l.contains(":latest") || !l.contains(':')) {
            findings.push(Finding {
                file: path.to_path_buf(),
                severity: Severity::Warn,
                message: format!("line {}: FROM uses `:latest` or no tag — pin a version", i + 1),
            });
        }
        if l.to_lowercase().starts_with("user root") {
            findings.push(Finding {
                file: path.to_path_buf(),
                severity: Severity::High,
                message: format!("line {}: container runs as root — add a non-root USER", i + 1),
            });
        }
    }

    // Secrets in ENV / ARG
    for (i, line) in content.lines().enumerate() {
        let l = line.trim_start().to_uppercase();
        if (l.starts_with("ENV ") || l.starts_with("ARG ")) &&
           ["PASSWORD", "SECRET", "TOKEN", "API_KEY"].iter().any(|k| l.contains(k)) {
            findings.push(Finding {
                file: path.to_path_buf(),
                severity: Severity::High,
                message: format!("line {}: secret-like value in ENV/ARG — use build secrets or runtime env", i + 1),
            });
        }
    }

    // Multi-stage hint
    if lower.matches("from ").count() == 1 && content.len() > 2_000 {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::Info,
            message: "single-stage Dockerfile — consider multi-stage build for smaller images".into(),
        });
    }

    findings
}

fn analyze_k8s(path: &Path, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let has_resources = content.contains("resources:") && (content.contains("limits:") || content.contains("requests:"));
    if !has_resources {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::Warn,
            message: "no resource limits/requests — pods may starve the node".into(),
        });
    }
    if !content.contains("livenessProbe:") {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::Info,
            message: "no livenessProbe defined".into(),
        });
    }
    if !content.contains("readinessProbe:") {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::Info,
            message: "no readinessProbe defined".into(),
        });
    }
    if !content.contains("runAsNonRoot") && content.contains("kind: Deployment") {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::Warn,
            message: "missing securityContext.runAsNonRoot for Deployment".into(),
        });
    }
    if content.contains("privileged: true") {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::High,
            message: "privileged: true — container has host-level access".into(),
        });
    }
    findings
}

fn analyze_terraform(path: &Path, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let l = content.to_lowercase();
    if l.contains("0.0.0.0/0") && l.contains("ingress") {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::High,
            message: "ingress open to 0.0.0.0/0 — public exposure".into(),
        });
    }
    if l.contains("acl") && l.contains("\"public-read") {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::High,
            message: "S3 ACL set to public-read — publicly readable bucket".into(),
        });
    }
    if l.contains("encrypted") && l.contains("= false") {
        findings.push(Finding {
            file: path.to_path_buf(),
            severity: Severity::Warn,
            message: "resource declared with encryption disabled".into(),
        });
    }
    findings
}

pub fn scan_workspace() {
    let files = discover();
    if files.is_empty() {
        println!("\n  {} No infrastructure files found.", "[INFRA]".dimmed());
        return;
    }
    println!("\n  {} Found {} infrastructure files:", "[INFRA]".cyan(), files.len());
    for f in &files {
        let kind = match f.kind {
            InfraKind::Dockerfile => "Dockerfile",
            InfraKind::K8sManifest => "Kubernetes",
            InfraKind::Terraform => "Terraform",
        };
        println!("    {} {}", kind.cyan(), f.path.display().to_string().dimmed());
    }
    println!();
}

pub fn security_scan() {
    let files = discover();
    if files.is_empty() {
        println!("\n  {} No infrastructure files to scan.", "[INFRA]".dimmed());
        return;
    }
    let mut total = 0usize;
    let mut high = 0usize;
    println!("\n  {} Security scan", "[INFRA]".cyan().bold());
    for f in &files {
        let findings = analyze_file(f);
        if findings.is_empty() { continue; }
        println!("\n  {}", f.path.display().to_string().bright_white());
        for finding in &findings {
            total += 1;
            if finding.severity == Severity::High { high += 1; }
            println!("    {} {}", sev_label(finding.severity), finding.message);
        }
    }
    if total == 0 {
        println!("    {} No issues found.", "[OK]".green());
    } else {
        println!("\n  {} {} issues total ({} high severity)",
            "Summary:".cyan(), total, high.to_string().red());
    }
    println!();
}

pub fn optimize_report() {
    // Optimization hints are the Info/Warn findings from a regular scan.
    security_scan();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn dockerfile_latest_tag_flagged() {
        let f = analyze_dockerfile(&PathBuf::from("Dockerfile"), "FROM alpine:latest\nCMD [\"sh\"]\n");
        assert!(f.iter().any(|x| x.severity == Severity::Warn && x.message.contains("latest")));
    }

    #[test]
    fn dockerfile_user_root_flagged() {
        let f = analyze_dockerfile(&PathBuf::from("Dockerfile"), "FROM alpine:3\nUSER root\n");
        assert!(f.iter().any(|x| x.severity == Severity::High));
    }

    #[test]
    fn k8s_missing_resources_flagged() {
        let yaml = "apiVersion: apps/v1\nkind: Deployment\nspec:\n  replicas: 1\n";
        let f = analyze_k8s(&PathBuf::from("dep.yaml"), yaml);
        assert!(f.iter().any(|x| x.message.contains("resource limits")));
    }

    #[test]
    fn terraform_open_ingress_flagged() {
        let tf = "resource \"aws_security_group\" \"x\" { ingress { cidr_blocks = [\"0.0.0.0/0\"] } }";
        let f = analyze_terraform(&PathBuf::from("main.tf"), tf);
        assert!(f.iter().any(|x| x.severity == Severity::High));
    }
}
