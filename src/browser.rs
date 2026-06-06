//! Browser Engine (lite) — reqwest-based fetch + simple HTML→Markdown
//! extraction with on-disk cache at `~/.dipralix/cache/web/`.
//!
//! v1 is intentionally lightweight (no headless Chromium). It handles the
//! common case: read a docs page, extract readable text. Pages that require
//! JS rendering are out of scope here.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn cache_path(url: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let cache_dir = home.join(".dipralix").join("cache").join("web");
    let _ = fs::create_dir_all(&cache_dir);
    let key = url_to_key(url);
    cache_dir.join(format!("{}.md", key))
}

fn url_to_key(url: &str) -> String {
    // Cheap stable key — strip scheme, replace unsafe chars.
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let mut s: String = stripped
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    if s.len() > 120 {
        s.truncate(120);
    }
    s
}

pub async fn fetch_markdown(url: &str) -> Result<String> {
    let cache = cache_path(url);
    if let Ok(cached) = fs::read_to_string(&cache) {
        return Ok(cached);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("dipralix/0.1.0 (+https://github.com/pratikacharya1234/dipralix)")
        .build()
        .context("failed to build http client")?;

    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {}", url))?;
    let status = resp.status();
    let body = resp.text().await.context("read response body")?;

    if !status.is_success() {
        anyhow::bail!("HTTP {} from {}", status, url);
    }

    let md = html_to_markdown(&body);
    let _ = fs::write(&cache, &md);
    Ok(md)
}

#[allow(dead_code)]
pub fn clear_cache() -> Result<usize> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let cache_dir = home.join(".dipralix").join("cache").join("web");
    if !cache_dir.exists() {
        return Ok(0);
    }
    let mut n = 0;
    for entry in fs::read_dir(&cache_dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            fs::remove_file(entry.path())?;
            n += 1;
        }
    }
    Ok(n)
}

/// Minimal HTML→Markdown extractor. Strips `<script>`/`<style>` blocks,
/// rewrites common block tags, collapses whitespace. Adequate for docs pages.
pub fn html_to_markdown(html: &str) -> String {
    let stripped = strip_blocks(html, &["script", "style", "noscript", "svg"]);
    let lowered = stripped.to_lowercase();

    // Try to focus on <main> / <article> if present.
    let body = extract_focus(&lowered, &stripped).unwrap_or(stripped.clone());

    let mut out = String::with_capacity(body.len());
    let mut in_tag = false;
    let mut tag_buf = String::new();
    for ch in body.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                emit_tag_marker(&tag_buf, &mut out);
            }
            _ => {
                if in_tag {
                    tag_buf.push(ch);
                } else {
                    out.push(ch);
                }
            }
        }
    }

    decode_entities(&collapse_ws(&out))
}

fn strip_blocks(html: &str, tags: &[&str]) -> String {
    let mut out = html.to_string();
    for tag in tags {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        loop {
            let lower = out.to_lowercase();
            let Some(start) = lower.find(&open) else {
                break;
            };
            let Some(end_rel) = lower[start..].find(&close) else {
                break;
            };
            let end = start + end_rel + close.len();
            out.replace_range(start..end, "");
        }
    }
    out
}

fn extract_focus(lower: &str, original: &str) -> Option<String> {
    for tag in &["<main", "<article"] {
        if let Some(start) = lower.find(tag) {
            // Find the end of the opening tag
            let start_close = lower[start..].find('>')? + start + 1;
            let close_tag = if tag == &"<main" {
                "</main>"
            } else {
                "</article>"
            };
            let end_rel = lower[start_close..].find(close_tag)?;
            return Some(original[start_close..start_close + end_rel].to_string());
        }
    }
    None
}

fn emit_tag_marker(tag: &str, out: &mut String) {
    let name = tag.split_whitespace().next().unwrap_or("").to_lowercase();
    let name = name.trim_start_matches('/');
    match name {
        "br" => out.push('\n'),
        "p" | "div" | "section" | "header" | "footer" | "ul" | "ol" => out.push_str("\n\n"),
        "li" => out.push_str("\n- "),
        "h1" => out.push_str("\n\n# "),
        "h2" => out.push_str("\n\n## "),
        "h3" => out.push_str("\n\n### "),
        "h4" => out.push_str("\n\n#### "),
        "h5" => out.push_str("\n\n##### "),
        "h6" => out.push_str("\n\n###### "),
        "code" | "pre" => out.push('`'),
        _ => {}
    }
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_nl = 0u8;
    let mut last_space = false;
    for ch in s.chars() {
        if ch == '\n' {
            if last_nl < 2 {
                out.push('\n');
                last_nl += 1;
            }
            last_space = false;
        } else if ch.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
            last_nl = 0;
        } else {
            out.push(ch);
            last_space = false;
            last_nl = 0;
        }
    }
    out.trim().to_string()
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_blocks() {
        let h = "<p>before</p><script>evil()</script><p>after</p>";
        let md = html_to_markdown(h);
        assert!(!md.contains("evil"));
        assert!(md.contains("before"));
        assert!(md.contains("after"));
    }

    #[test]
    fn headings_render() {
        let h = "<h1>Title</h1><p>body</p>";
        let md = html_to_markdown(h);
        assert!(md.contains("# Title"));
    }

    #[test]
    fn entity_decode() {
        assert_eq!(decode_entities("a &amp; b"), "a & b");
    }

    #[test]
    fn url_key_safe() {
        let k = url_to_key("https://docs.rs/axum/latest/axum/");
        assert!(k.chars().all(|c| c.is_alphanumeric() || c == '_'));
    }
}
