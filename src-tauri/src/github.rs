use std::process::Command;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{AppError, AppResult};

const KEYRING_SERVICE: &str = "dev.tide.app";
const KEYRING_USER: &str = "github-token";
const API: &str = "https://api.github.com";

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("tide")
            .build()
            .expect("reqwest client")
    })
}

// ---------------------------------------------------------------------------
// Token storage — macOS Keychain. The token NEVER crosses into the webview.
// ---------------------------------------------------------------------------

fn keyring_entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| AppError::Pty(format!("keychain: {e}")))
}

fn save_token(token: &str) -> AppResult<()> {
    keyring_entry()?
        .set_password(token)
        .map_err(|e| AppError::Pty(format!("keychain: {e}")))
}

fn load_token() -> Option<String> {
    keyring_entry().ok()?.get_password().ok()
}

fn delete_token() {
    if let Ok(entry) = keyring_entry() {
        let _ = entry.delete_credential();
    }
}

fn require_token() -> AppResult<String> {
    load_token().ok_or_else(|| AppError::Pty("not connected to GitHub".into()))
}

// ---------------------------------------------------------------------------
// REST helpers
// ---------------------------------------------------------------------------

async fn api_get(token: &str, path: &str) -> AppResult<serde_json::Value> {
    let resp = http()
        .get(format!("{API}{path}"))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?;
    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?;
    if !status.is_success() {
        let msg = body["message"].as_str().unwrap_or("request failed");
        return Err(AppError::Pty(format!("github ({status}): {msg}")));
    }
    Ok(body)
}

async fn api_post(token: &str, path: &str, payload: serde_json::Value) -> AppResult<serde_json::Value> {
    let resp = http()
        .post(format!("{API}{path}"))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?;
    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?;
    if !status.is_success() {
        // surface validation details (e.g. "A pull request already exists")
        let mut msg = body["message"].as_str().unwrap_or("request failed").to_string();
        if let Some(errors) = body["errors"].as_array() {
            for err in errors {
                if let Some(detail) = err["message"].as_str() {
                    msg.push_str(&format!(": {detail}"));
                }
            }
        }
        return Err(AppError::Pty(format!("github ({status}): {msg}")));
    }
    Ok(body)
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct GhUser {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

fn parse_user(v: &serde_json::Value) -> GhUser {
    GhUser {
        login: v["login"].as_str().unwrap_or_default().to_string(),
        name: v["name"].as_str().map(String::from),
        avatar_url: v["avatar_url"].as_str().map(String::from),
    }
}

#[tauri::command]
pub async fn gh_status() -> Option<GhUser> {
    let token = load_token()?;
    let user = api_get(&token, "/user").await.ok()?;
    Some(parse_user(&user))
}

#[tauri::command]
pub async fn gh_set_pat(token: String) -> AppResult<GhUser> {
    let user = api_get(&token, "/user").await?;
    save_token(&token)?;
    Ok(parse_user(&user))
}

#[tauri::command]
pub fn gh_logout() {
    delete_token();
}

#[derive(Serialize, Clone)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[tauri::command]
pub async fn gh_device_start(client_id: String) -> AppResult<DeviceCode> {
    #[derive(Deserialize)]
    struct Resp {
        device_code: String,
        user_code: String,
        verification_uri: String,
        interval: u64,
        expires_in: u64,
    }
    let resp: Resp = http()
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .json(&json!({ "client_id": client_id, "scope": "repo" }))
        .send()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Pty(format!("github device flow: {e}")))?;
    Ok(DeviceCode {
        device_code: resp.device_code,
        user_code: resp.user_code,
        verification_uri: resp.verification_uri,
        interval: resp.interval,
        expires_in: resp.expires_in,
    })
}

#[derive(Serialize, Clone)]
pub struct PollResult {
    /// "ok" | "pending" | "slow_down" | "expired" | "denied"
    pub status: String,
    pub user: Option<GhUser>,
}

#[tauri::command]
pub async fn gh_device_poll(client_id: String, device_code: String) -> AppResult<PollResult> {
    let body: serde_json::Value = http()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&json!({
            "client_id": client_id,
            "device_code": device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
        }))
        .send()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?;

    if let Some(token) = body["access_token"].as_str() {
        let user = api_get(token, "/user").await?;
        save_token(token)?;
        return Ok(PollResult { status: "ok".into(), user: Some(parse_user(&user)) });
    }
    let status = match body["error"].as_str() {
        Some("authorization_pending") => "pending",
        Some("slow_down") => "slow_down",
        Some("expired_token") => "expired",
        Some("access_denied") => "denied",
        other => {
            return Err(AppError::Pty(format!(
                "github device flow: {}",
                other.unwrap_or("unknown error")
            )));
        }
    };
    Ok(PollResult { status: status.into(), user: None })
}

// ---------------------------------------------------------------------------
// Remote info + push/pull (git CLI — uses the user's credential helper)
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct RemoteInfo {
    pub owner: String,
    pub name: String,
}

/// Parses `git@github.com:owner/repo.git` and
/// `https://github.com/owner/repo(.git)` remote URLs.
fn parse_github_remote(url: &str) -> Option<RemoteInfo> {
    let rest = url
        .trim()
        .strip_prefix("git@github.com:")
        .or_else(|| url.trim().strip_prefix("https://github.com/"))
        .or_else(|| url.trim().strip_prefix("ssh://git@github.com/"))?;
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let mut parts = rest.splitn(2, '/');
    Some(RemoteInfo {
        owner: parts.next()?.to_string(),
        name: parts.next()?.trim_end_matches('/').to_string(),
    })
}

fn run_git(repo: &str, args: &[&str]) -> AppResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| AppError::Pty(format!("failed to run git: {e}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(AppError::Pty(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

#[tauri::command]
pub fn gh_remote_info(repo: String) -> Option<RemoteInfo> {
    let url = run_git(&repo, &["remote", "get-url", "origin"]).ok()?;
    parse_github_remote(&url)
}

#[tauri::command]
pub async fn gh_push(repo: String) -> AppResult<()> {
    // -u origin HEAD: pushes the current branch and sets upstream if missing
    tauri::async_runtime::spawn_blocking(move || {
        run_git(&repo, &["push", "-u", "origin", "HEAD"]).map(|_| ())
    })
    .await
    .map_err(|e| AppError::Pty(e.to_string()))?
}

#[tauri::command]
pub async fn gh_pull(repo: String) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || run_git(&repo, &["pull"]).map(|_| ()))
        .await
        .map_err(|e| AppError::Pty(e.to_string()))?
}

// ---------------------------------------------------------------------------
// Pull requests + CI checks
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct Pr {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub draft: bool,
    pub head_ref: String,
    pub base_ref: String,
    pub head_sha: String,
    pub html_url: String,
    pub author: String,
    pub created_at: String,
}

fn parse_pr(v: &serde_json::Value) -> Pr {
    Pr {
        number: v["number"].as_u64().unwrap_or(0),
        title: v["title"].as_str().unwrap_or_default().to_string(),
        state: v["state"].as_str().unwrap_or_default().to_string(),
        draft: v["draft"].as_bool().unwrap_or(false),
        head_ref: v["head"]["ref"].as_str().unwrap_or_default().to_string(),
        base_ref: v["base"]["ref"].as_str().unwrap_or_default().to_string(),
        head_sha: v["head"]["sha"].as_str().unwrap_or_default().to_string(),
        html_url: v["html_url"].as_str().unwrap_or_default().to_string(),
        author: v["user"]["login"].as_str().unwrap_or_default().to_string(),
        created_at: v["created_at"].as_str().unwrap_or_default().to_string(),
    }
}

/// The open PR whose head is `branch` (same-repo heads only — forks
/// would need a different owner prefix).
#[tauri::command]
pub async fn gh_pr_for_branch(owner: String, name: String, branch: String) -> AppResult<Option<Pr>> {
    let token = require_token()?;
    let body = api_get(
        &token,
        &format!("/repos/{owner}/{name}/pulls?head={owner}:{branch}&state=open"),
    )
    .await?;
    Ok(body.as_array().and_then(|prs| prs.first()).map(parse_pr))
}

#[derive(Serialize, Clone, Default)]
pub struct ReviewSummary {
    pub approved: u64,
    pub changes_requested: u64,
    pub commented: u64,
}

/// Latest meaningful review state per reviewer (APPROVED /
/// CHANGES_REQUESTED win over COMMENTED).
#[tauri::command]
pub async fn gh_pr_reviews(owner: String, name: String, number: u64) -> AppResult<ReviewSummary> {
    let token = require_token()?;
    let body = api_get(
        &token,
        &format!("/repos/{owner}/{name}/pulls/{number}/reviews?per_page=100"),
    )
    .await?;

    let mut decisive: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    let mut commenters: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(reviews) = body.as_array() {
        for review in reviews {
            let user = review["user"]["login"].as_str().unwrap_or_default().to_string();
            match review["state"].as_str() {
                Some("APPROVED") => {
                    decisive.insert(user, "approved");
                }
                Some("CHANGES_REQUESTED") => {
                    decisive.insert(user, "changes");
                }
                Some("COMMENTED") => {
                    commenters.insert(user);
                }
                _ => {}
            }
        }
    }
    Ok(ReviewSummary {
        approved: decisive.values().filter(|s| **s == "approved").count() as u64,
        changes_requested: decisive.values().filter(|s| **s == "changes").count() as u64,
        commented: commenters
            .iter()
            .filter(|u| !decisive.contains_key(*u))
            .count() as u64,
    })
}

#[tauri::command]
pub async fn gh_list_prs(owner: String, name: String) -> AppResult<Vec<Pr>> {
    let token = require_token()?;
    let body = api_get(&token, &format!("/repos/{owner}/{name}/pulls?state=open&per_page=30")).await?;
    Ok(body.as_array().map(|prs| prs.iter().map(parse_pr).collect()).unwrap_or_default())
}

#[tauri::command]
pub async fn gh_create_pr(
    owner: String,
    name: String,
    head: String,
    base: String,
    title: String,
    body: Option<String>,
) -> AppResult<Pr> {
    let token = require_token()?;
    let pr = api_post(
        &token,
        &format!("/repos/{owner}/{name}/pulls"),
        json!({ "head": head, "base": base, "title": title, "body": body.unwrap_or_default() }),
    )
    .await?;
    Ok(parse_pr(&pr))
}

#[derive(Serialize, Clone)]
pub struct CheckSummary {
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
    pub pending: u64,
}

#[tauri::command]
pub async fn gh_pr_checks(owner: String, name: String, sha: String) -> AppResult<CheckSummary> {
    let token = require_token()?;
    let body = api_get(&token, &format!("/repos/{owner}/{name}/commits/{sha}/check-runs?per_page=100")).await?;
    let mut summary = CheckSummary { total: 0, passed: 0, failed: 0, pending: 0 };
    if let Some(runs) = body["check_runs"].as_array() {
        for run in runs {
            summary.total += 1;
            match run["conclusion"].as_str() {
                Some("success") | Some("neutral") | Some("skipped") => summary.passed += 1,
                Some("failure") | Some("timed_out") | Some("cancelled") | Some("action_required") => {
                    summary.failed += 1;
                }
                _ => summary.pending += 1, // still running / queued
            }
        }
    }
    Ok(summary)
}
