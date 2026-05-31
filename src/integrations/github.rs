use serde_json::{json, Value};

use crate::types::FunctionDeclaration;
use crate::integrations::GithubConfig;
use crate::integrations::IntegrationService;
use crate::tools::ToolResult;

pub struct GithubIntegration {
    token: String,
    client: reqwest::Client,
}

impl GithubIntegration {
    pub fn new(config: &GithubConfig) -> Self {
        GithubIntegration {
            token: config.token.clone(),
            client: reqwest::Client::new(),
        }
    }

    fn api_url(path: &str) -> String {
        format!("https://api.github.com{}", path)
    }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let resp = self
            .client
            .get(Self::api_url(path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dipralix/1.0")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| format!("GitHub API request failed: {}", e))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if !status.is_success() {
            return Err(format!("GitHub API HTTP {}: {}", status.as_u16(), truncate(&body, 400)));
        }

        serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse GitHub response: {} - body: {}", e, truncate(&body, 200)))
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        let resp = self
            .client
            .post(Self::api_url(path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dipralix/1.0")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("GitHub API request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if !status.is_success() {
            return Err(format!("GitHub API HTTP {}: {}", status.as_u16(), truncate(&text, 400)));
        }

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse GitHub response: {} - body: {}", e, truncate(&text, 200)))
    }

    async fn patch(&self, path: &str, body: &Value) -> Result<Value, String> {
        let resp = self
            .client
            .patch(Self::api_url(path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dipralix/1.0")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("GitHub API request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if !status.is_success() {
            return Err(format!("GitHub API HTTP {}: {}", status.as_u16(), truncate(&text, 400)));
        }

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse GitHub response: {} - body: {}", e, truncate(&text, 200)))
    }

    fn format_repo_result(repo: &Value) -> String {
        let name = repo["full_name"].as_str().unwrap_or("?");
        let desc = repo["description"].as_str().unwrap_or("");
        let stars = repo["stargazers_count"].as_u64().unwrap_or(0);
        let lang = repo["language"].as_str().unwrap_or("");
        let private = repo["private"].as_bool().unwrap_or(false);
        let visibility = if private { "private" } else { "public" };
        format!("{:<50} {} stars  {}  {}  {}", name, stars, visibility, lang, desc)
    }

    fn format_issue_result(issue: &Value) -> String {
        let number = issue["number"].as_u64().unwrap_or(0);
        let title = issue["title"].as_str().unwrap_or("?");
        let state = issue["state"].as_str().unwrap_or("?");
        let user = issue["user"]["login"].as_str().unwrap_or("?");
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .map(|a| a.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();
        let label_str = if labels.is_empty() {
            String::new()
        } else {
            format!(" [{}]", labels.join(", "))
        };
        format!("#{} {:<60} {}  by {}{}", number, title, state, user, label_str)
    }

    fn format_pr_result(pr: &Value) -> String {
        let number = pr["number"].as_u64().unwrap_or(0);
        let title = pr["title"].as_str().unwrap_or("?");
        let state = pr["state"].as_str().unwrap_or("?");
        let user = pr["user"]["login"].as_str().unwrap_or("?");
        let draft = pr["draft"].as_bool().unwrap_or(false);
        let merged = pr["merged"].as_bool().unwrap_or(false);
        let status = if merged { "merged" } else if draft { "draft" } else { state };
        format!("#{} {:<60} {}  by {}", number, title, status, user)
    }
}

impl IntegrationService for GithubIntegration {
    fn name(&self) -> &str {
        "github"
    }

    fn tool_declarations(&self) -> Vec<FunctionDeclaration> {
        vec![
            FunctionDeclaration {
                name: "list_repos".to_string(),
                description: "List repositories for the authenticated user. Optional: filter by visibility (all, public, private).".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "visibility": { "type": "STRING", "description": "Filter: all, public, or private (default: all)" },
                        "sort": { "type": "STRING", "description": "Sort: created, updated, pushed, full_name (default: updated)" },
                        "per_page": { "type": "INTEGER", "description": "Results per page (default: 10, max: 100)" }
                    },
                    "required": []
                }),
            },
            FunctionDeclaration {
                name: "get_repo".to_string(),
                description: "Get details about a specific repository. Provide owner/repo (e.g. 'torvalds/linux').".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" }
                    },
                    "required": ["repo"]
                }),
            },
            FunctionDeclaration {
                name: "create_issue".to_string(),
                description: "Create a new GitHub issue in a repository.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "title": { "type": "STRING", "description": "Issue title" },
                        "body": { "type": "STRING", "description": "Issue body (markdown)" },
                        "labels": { "type": "ARRAY", "items": { "type": "STRING" }, "description": "Label names" }
                    },
                    "required": ["repo", "title"]
                }),
            },
            FunctionDeclaration {
                name: "list_issues".to_string(),
                description: "List issues for a repository. Supports filtering by state, labels, and assignee.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "state": { "type": "STRING", "description": "open, closed, or all (default: open)" },
                        "labels": { "type": "STRING", "description": "Comma-separated label names" },
                        "per_page": { "type": "INTEGER", "description": "Results per page (default: 10)" }
                    },
                    "required": ["repo"]
                }),
            },
            FunctionDeclaration {
                name: "get_issue".to_string(),
                description: "Get a specific issue by number.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "issue_number": { "type": "INTEGER", "description": "Issue number" }
                    },
                    "required": ["repo", "issue_number"]
                }),
            },
            FunctionDeclaration {
                name: "comment_issue".to_string(),
                description: "Add a comment to an existing issue or PR.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "issue_number": { "type": "INTEGER", "description": "Issue or PR number" },
                        "body": { "type": "STRING", "description": "Comment body (markdown)" }
                    },
                    "required": ["repo", "issue_number", "body"]
                }),
            },
            FunctionDeclaration {
                name: "close_issue".to_string(),
                description: "Close an open issue.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "issue_number": { "type": "INTEGER", "description": "Issue number" }
                    },
                    "required": ["repo", "issue_number"]
                }),
            },
            FunctionDeclaration {
                name: "create_pr".to_string(),
                description: "Create a new pull request.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "title": { "type": "STRING", "description": "PR title" },
                        "head": { "type": "STRING", "description": "Source branch name" },
                        "base": { "type": "STRING", "description": "Target branch (default: main)" },
                        "body": { "type": "STRING", "description": "PR description (markdown)" },
                        "draft": { "type": "BOOLEAN", "description": "Create as draft PR (default: false)" }
                    },
                    "required": ["repo", "title", "head"]
                }),
            },
            FunctionDeclaration {
                name: "list_prs".to_string(),
                description: "List pull requests for a repository.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "state": { "type": "STRING", "description": "open, closed, or all (default: open)" },
                        "per_page": { "type": "INTEGER", "description": "Results per page (default: 10)" }
                    },
                    "required": ["repo"]
                }),
            },
            FunctionDeclaration {
                name: "get_pr".to_string(),
                description: "Get details about a specific pull request.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "pr_number": { "type": "INTEGER", "description": "PR number" }
                    },
                    "required": ["repo", "pr_number"]
                }),
            },
            FunctionDeclaration {
                name: "search_code".to_string(),
                description: "Search GitHub code across all public repositories. Returns matching files with paths and repository info.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "query": { "type": "STRING", "description": "Search query (supports GitHub search syntax: language:rust, repo:org/name, etc.)" },
                        "per_page": { "type": "INTEGER", "description": "Results per page (default: 10, max: 100)" }
                    },
                    "required": ["query"]
                }),
            },
            FunctionDeclaration {
                name: "list_branches".to_string(),
                description: "List branches for a repository.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "repo": { "type": "STRING", "description": "Repository in owner/repo format" },
                        "per_page": { "type": "INTEGER", "description": "Results per page (default: 10)" }
                    },
                    "required": ["repo"]
                }),
            },
        ]
    }

    fn call_tool(&self, tool_name: &str, args: Value) -> ToolResult {
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => return ToolResult::err("No async runtime available for GitHub API call"),
        };

        match tool_name {
            "list_repos" => {
                let visibility = args.get("visibility").and_then(|v| v.as_str()).unwrap_or("all");
                let sort = args.get("sort").and_then(|v| v.as_str()).unwrap_or("updated");
                let per_page = args.get("per_page").and_then(|v| v.as_u64()).unwrap_or(10);
                let path = format!("/user/repos?visibility={}&sort={}&per_page={}&affiliation=owner,collaborator,organization_member", visibility, sort, per_page);

                let result = rt.block_on(self.get(&path));
                match result {
                    Ok(repos) => {
                        let _empty: Vec<serde_json::Value> = Vec::new(); let arr = repos.as_array().unwrap_or(&_empty);
                        if arr.is_empty() {
                            return ToolResult::ok("No repositories found.");
                        }
                        let lines: Vec<String> = arr.iter().map(Self::format_repo_result).collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "get_repo" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo (owner/name)"),
                };
                let result = rt.block_on(self.get(&format!("/repos/{}", repo)));
                match result {
                    Ok(repo_data) => {
                        let name = repo_data["full_name"].as_str().unwrap_or("?");
                        let desc = repo_data["description"].as_str().unwrap_or("(no description)");
                        let stars = repo_data["stargazers_count"].as_u64().unwrap_or(0);
                        let forks = repo_data["forks_count"].as_u64().unwrap_or(0);
                        let issues = repo_data["open_issues_count"].as_u64().unwrap_or(0);
                        let lang = repo_data["language"].as_str().unwrap_or("unknown");
                        let default_branch = repo_data["default_branch"].as_str().unwrap_or("?");
                        let private = repo_data["private"].as_bool().unwrap_or(false);
                        let created = repo_data["created_at"].as_str().unwrap_or("?");
                        let updated = repo_data["updated_at"].as_str().unwrap_or("?");
                        let clone_url = repo_data["clone_url"].as_str().unwrap_or("");
                        let html_url = repo_data["html_url"].as_str().unwrap_or("");
                        ToolResult::ok(format!(
                            "{}\n  Description: {}\n  Language: {}\n  Stars: {}  Forks: {}  Open Issues: {}\n  Default branch: {}\n  Visibility: {}\n  Created: {}\n  Updated: {}\n  Clone: {}\n  URL: {}",
                            name, desc, lang, stars, forks, issues, default_branch,
                            if private { "private" } else { "public" },
                            created, updated, clone_url, html_url
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "create_issue" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let title = match args.get("title").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ToolResult::err("Missing required argument: title"),
                };
                let body_str = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
                let labels = args.get("labels").cloned().unwrap_or(json!([]));
                let payload = json!({ "title": title, "body": body_str, "labels": labels });
                let result = rt.block_on(self.post(&format!("/repos/{}/issues", repo), &payload));
                match result {
                    Ok(issue) => {
                        let num = issue["number"].as_u64().unwrap_or(0);
                        let url = issue["html_url"].as_str().unwrap_or("");
                        ToolResult::ok(format!("Created issue #{}: {}\n{}", num, title, url))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "list_issues" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let state = args.get("state").and_then(|v| v.as_str()).unwrap_or("open");
                let per_page = args.get("per_page").and_then(|v| v.as_u64()).unwrap_or(10);
                let mut path = format!("/repos/{}/issues?state={}&per_page={}", repo, state, per_page);
                if let Some(labels) = args.get("labels").and_then(|v| v.as_str()) {
                    path.push_str(&format!("&labels={}", labels));
                }
                let result = rt.block_on(self.get(&path));
                match result {
                    Ok(issues) => {
                        let _empty: Vec<serde_json::Value> = Vec::new(); let arr = issues.as_array().unwrap_or(&_empty);
                        if arr.is_empty() {
                            return ToolResult::ok("No issues found.");
                        }
                        let lines: Vec<String> = arr.iter().map(Self::format_issue_result).collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "get_issue" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let number = match args.get("issue_number").and_then(|v| v.as_u64()) {
                    Some(n) => n,
                    None => return ToolResult::err("Missing required argument: issue_number"),
                };
                let result = rt.block_on(self.get(&format!("/repos/{}/issues/{}", repo, number)));
                match result {
                    Ok(issue) => {
                        let title = issue["title"].as_str().unwrap_or("?");
                        let state = issue["state"].as_str().unwrap_or("?");
                        let user = issue["user"]["login"].as_str().unwrap_or("?");
                        let body = issue["body"].as_str().unwrap_or("");
                        let url = issue["html_url"].as_str().unwrap_or("");
                        let labels: Vec<&str> = issue["labels"].as_array()
                            .map(|a| a.iter().filter_map(|l| l["name"].as_str()).collect())
                            .unwrap_or_default();
                        let label_display = if labels.is_empty() { "none".to_string() } else { labels.join(", ") };
                        ToolResult::ok(format!(
                            "#{} {} [{}] by {}\n  Labels: {}\n  URL: {}\n\n{}",
                            number, title, state, user,
                            label_display,
                            url, body
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "comment_issue" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let number = match args.get("issue_number").and_then(|v| v.as_u64()) {
                    Some(n) => n,
                    None => return ToolResult::err("Missing required argument: issue_number"),
                };
                let body_str = match args.get("body").and_then(|v| v.as_str()) {
                    Some(b) => b,
                    None => return ToolResult::err("Missing required argument: body"),
                };
                let payload = json!({ "body": body_str });
                let result = rt.block_on(self.post(&format!("/repos/{}/issues/{}/comments", repo, number), &payload));
                match result {
                    Ok(comment) => {
                        let url = comment["html_url"].as_str().unwrap_or("");
                        ToolResult::ok(format!("Comment added: {}", url))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "close_issue" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let number = match args.get("issue_number").and_then(|v| v.as_u64()) {
                    Some(n) => n,
                    None => return ToolResult::err("Missing required argument: issue_number"),
                };
                let payload = json!({ "state": "closed" });
                let result = rt.block_on(self.patch(&format!("/repos/{}/issues/{}", repo, number), &payload));
                match result {
                    Ok(_) => ToolResult::ok(format!("Closed issue #{} in {}", number, repo)),
                    Err(e) => ToolResult::err(e),
                }
            }
            "create_pr" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let title = match args.get("title").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ToolResult::err("Missing required argument: title"),
                };
                let head = match args.get("head").and_then(|v| v.as_str()) {
                    Some(h) => h,
                    None => return ToolResult::err("Missing required argument: head (source branch)"),
                };
                let base = args.get("base").and_then(|v| v.as_str()).unwrap_or("main");
                let body_str = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
                let draft = args.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
                let payload = json!({
                    "title": title,
                    "head": head,
                    "base": base,
                    "body": body_str,
                    "draft": draft
                });
                let result = rt.block_on(self.post(&format!("/repos/{}/pulls", repo), &payload));
                match result {
                    Ok(pr) => {
                        let num = pr["number"].as_u64().unwrap_or(0);
                        let url = pr["html_url"].as_str().unwrap_or("");
                        let draft_label = if draft { " (draft)" } else { "" };
                        ToolResult::ok(format!("Created PR #{}:{}{}\n  {} -> {}\n  {}", num, title, draft_label, head, base, url))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "list_prs" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let state = args.get("state").and_then(|v| v.as_str()).unwrap_or("open");
                let per_page = args.get("per_page").and_then(|v| v.as_u64()).unwrap_or(10);
                let path = format!("/repos/{}/pulls?state={}&per_page={}", repo, state, per_page);
                let result = rt.block_on(self.get(&path));
                match result {
                    Ok(prs) => {
                        let _empty: Vec<serde_json::Value> = Vec::new(); let arr = prs.as_array().unwrap_or(&_empty);
                        if arr.is_empty() {
                            return ToolResult::ok("No pull requests found.");
                        }
                        let lines: Vec<String> = arr.iter().map(Self::format_pr_result).collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "get_pr" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let number = match args.get("pr_number").and_then(|v| v.as_u64()) {
                    Some(n) => n,
                    None => return ToolResult::err("Missing required argument: pr_number"),
                };
                let result = rt.block_on(self.get(&format!("/repos/{}/pulls/{}", repo, number)));
                match result {
                    Ok(pr) => {
                        let title = pr["title"].as_str().unwrap_or("?");
                        let state = pr["state"].as_str().unwrap_or("?");
                        let merged = pr["merged"].as_bool().unwrap_or(false);
                        let user = pr["user"]["login"].as_str().unwrap_or("?");
                        let body = pr["body"].as_str().unwrap_or("");
                        let url = pr["html_url"].as_str().unwrap_or("");
                        let head = pr["head"]["ref"].as_str().unwrap_or("?");
                        let base = pr["base"]["ref"].as_str().unwrap_or("?");
                        let additions = pr["additions"].as_u64().unwrap_or(0);
                        let deletions = pr["deletions"].as_u64().unwrap_or(0);
                        let files = pr["changed_files"].as_u64().unwrap_or(0);
                        let status = if merged { "merged" } else { state };
                        ToolResult::ok(format!(
                            "#{} {} [{}] by {}\n  {} -> {}  (+{} -{} in {} files)\n  URL: {}\n\n{}",
                            number, title, status, user, head, base, additions, deletions, files, url, body
                        ))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "search_code" => {
                let query = match args.get("query").and_then(|v| v.as_str()) {
                    Some(q) => q,
                    None => return ToolResult::err("Missing required argument: query"),
                };
                let per_page = args.get("per_page").and_then(|v| v.as_u64()).unwrap_or(10);
                let path = format!("/search/code?q={}&per_page={}", url_encode(query), per_page);
                let result = rt.block_on(self.get(&path));
                match result {
                    Ok(data) => {
                        let _empty: Vec<serde_json::Value> = Vec::new(); let items = data["items"].as_array().unwrap_or(&_empty);
                        if items.is_empty() {
                            return ToolResult::ok("No code results found.");
                        }
                        let lines: Vec<String> = items.iter().map(|item| {
                            let repo_name = item["repository"]["full_name"].as_str().unwrap_or("?");
                            let path = item["path"].as_str().unwrap_or("?");
                            let url = item["html_url"].as_str().unwrap_or("");
                            format!("{}/{}  {}", repo_name, path, url)
                        }).collect();
                        let total = data["total_count"].as_u64().unwrap_or(0);
                        ToolResult::ok(format!("{} results (showing {}):\n{}", total, items.len(), lines.join("\n")))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            "list_branches" => {
                let repo = match args.get("repo").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ToolResult::err("Missing required argument: repo"),
                };
                let per_page = args.get("per_page").and_then(|v| v.as_u64()).unwrap_or(10);
                let path = format!("/repos/{}/branches?per_page={}", repo, per_page);
                let result = rt.block_on(self.get(&path));
                match result {
                    Ok(branches) => {
                        let _empty: Vec<serde_json::Value> = Vec::new(); let arr = branches.as_array().unwrap_or(&_empty);
                        if arr.is_empty() {
                            return ToolResult::ok("No branches found.");
                        }
                        let lines: Vec<String> = arr.iter().map(|b| {
                            let name = b["name"].as_str().unwrap_or("?");
                            let protected = b["protected"].as_bool().unwrap_or(false);
                            format!("{}{}", name, if protected { " [protected]" } else { "" })
                        }).collect();
                        ToolResult::ok(lines.join("\n"))
                    }
                    Err(e) => ToolResult::err(e),
                }
            }
            _ => ToolResult::err(format!("Unknown GitHub tool: {}", tool_name)),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn url_encode(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('#', "%23")
        .replace('&', "%26")
        .replace('?', "%3F")
        .replace('=', "%3D")
        .replace('+', "%2B")
        .replace('/', "%2F")
        .replace(':', "%3A")
}
