use std::process::Command;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::store;

const KEYRING_SERVICE: &str = "dev.tide.app";
const LEGACY_KEYRING_USER: &str = "github-token";
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
// Token storage — macOS Keychain, one entry per account login.
// Tokens NEVER cross into the webview; logins (not secret) live in the
// persisted app state for the account switcher.
// ---------------------------------------------------------------------------

fn entry_for(login: &str) -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, &format!("github-token:{login}"))
        .map_err(|e| AppError::Pty(format!("keychain: {e}")))
}

fn token_for(login: &str) -> Option<String> {
    entry_for(login).ok()?.get_password().ok()
}

fn active_login(state: &AppState) -> Option<String> {
    state.store.lock().gh_active.clone()
}

fn load_token(state: &AppState) -> Option<String> {
    token_for(&active_login(state)?)
}

fn require_token(state: &AppState) -> AppResult<String> {
    load_token(state).ok_or_else(|| AppError::Pty("not connected to GitHub".into()))
}

/// Registers `login` (storing its token), makes it the active account.
fn register_account(state: &AppState, login: &str, token: &str) -> AppResult<()> {
    entry_for(login)?
        .set_password(token)
        .map_err(|e| AppError::Pty(format!("keychain: {e}")))?;
    let mut persisted = state.store.lock();
    if !persisted.gh_accounts.iter().any(|a| a == login) {
        persisted.gh_accounts.push(login.to_string());
    }
    persisted.gh_active = Some(login.to_string());
    store::save(&persisted)
}

/// One-time migration from the single-account keyring entry.
async fn migrate_legacy(state: &AppState) {
    let needs = { state.store.lock().gh_accounts.is_empty() };
    if !needs {
        return;
    }
    let Ok(legacy) = keyring::Entry::new(KEYRING_SERVICE, LEGACY_KEYRING_USER) else { return };
    let Ok(token) = legacy.get_password() else { return };
    if let Ok(user) = api_get(&token, "/user").await {
        let login = user["login"].as_str().unwrap_or_default().to_string();
        if !login.is_empty() && register_account(state, &login, &token).is_ok() {
            let _ = legacy.delete_credential();
        }
    }
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

#[derive(Serialize, Clone)]
pub struct GhStatus {
    pub user: Option<GhUser>,
    pub accounts: Vec<String>,
    pub active: Option<String>,
}

#[tauri::command]
pub async fn gh_status(state: State<'_, AppState>) -> Result<GhStatus, AppError> {
    migrate_legacy(&state).await;
    let (accounts, active) = {
        let persisted = state.store.lock();
        (persisted.gh_accounts.clone(), persisted.gh_active.clone())
    };
    let user = match active.as_deref().and_then(token_for) {
        Some(token) => api_get(&token, "/user").await.ok().map(|v| parse_user(&v)),
        None => None,
    };
    Ok(GhStatus { user, accounts, active })
}

#[tauri::command]
pub async fn gh_set_pat(state: State<'_, AppState>, token: String) -> Result<GhUser, AppError> {
    let user = api_get(&token, "/user").await?;
    let parsed = parse_user(&user);
    register_account(&state, &parsed.login, &token)?;
    Ok(parsed)
}

#[tauri::command]
pub fn gh_switch(state: State<'_, AppState>, login: String) -> AppResult<()> {
    let mut persisted = state.store.lock();
    if !persisted.gh_accounts.iter().any(|a| a == &login) {
        return Err(AppError::Pty(format!("unknown account: {login}")));
    }
    persisted.gh_active = Some(login);
    store::save(&persisted)
}

/// Signs out `login` (or the active account when omitted).
#[tauri::command]
pub fn gh_logout(state: State<'_, AppState>, login: Option<String>) -> AppResult<()> {
    let mut persisted = state.store.lock();
    let Some(target) = login.or_else(|| persisted.gh_active.clone()) else {
        return Ok(());
    };
    if let Ok(entry) = entry_for(&target) {
        let _ = entry.delete_credential();
    }
    persisted.gh_accounts.retain(|a| a != &target);
    if persisted.gh_active.as_deref() == Some(&target) {
        persisted.gh_active = persisted.gh_accounts.first().cloned();
    }
    store::save(&persisted)
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
pub async fn gh_device_poll(
    state: State<'_, AppState>,
    client_id: String,
    device_code: String,
) -> Result<PollResult, AppError> {
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
        let parsed = parse_user(&user);
        register_account(&state, &parsed.login, token)?;
        return Ok(PollResult { status: "ok".into(), user: Some(parsed) });
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

async fn api_put(token: &str, path: &str, payload: serde_json::Value) -> AppResult<serde_json::Value> {
    let resp = http()
        .put(format!("{API}{path}"))
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
        let msg = body["message"].as_str().unwrap_or("request failed");
        return Err(AppError::Pty(format!("github ({status}): {msg}")));
    }
    Ok(body)
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
pub async fn gh_pr_for_branch(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    branch: String,
) -> Result<Option<Pr>, AppError> {
    let token = require_token(&state)?;
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
pub async fn gh_pr_reviews(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    number: u64,
) -> Result<ReviewSummary, AppError> {
    let token = require_token(&state)?;
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
pub async fn gh_list_prs(
    state: State<'_, AppState>,
    owner: String,
    name: String,
) -> Result<Vec<Pr>, AppError> {
    let token = require_token(&state)?;
    let body = api_get(&token, &format!("/repos/{owner}/{name}/pulls?state=open&per_page=30")).await?;
    Ok(body.as_array().map(|prs| prs.iter().map(parse_pr).collect()).unwrap_or_default())
}

#[tauri::command]
pub async fn gh_create_pr(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    head: String,
    base: String,
    title: String,
    body: Option<String>,
) -> Result<Pr, AppError> {
    let token = require_token(&state)?;
    let pr = api_post(
        &token,
        &format!("/repos/{owner}/{name}/pulls"),
        json!({ "head": head, "base": base, "title": title, "body": body.unwrap_or_default() }),
    )
    .await?;
    Ok(parse_pr(&pr))
}

// ---------------------------------------------------------------------------
// PR detail — full conversation + checks, so the user never needs the
// GitHub website to follow a PR.
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct PrComment {
    pub author: String,
    pub avatar_url: Option<String>,
    pub body: String,
    pub created_at: String,
    /// file path for inline review comments
    pub path: Option<String>,
    pub line: Option<u64>,
    /// surrounding diff context for inline review comments
    pub diff_hunk: Option<String>,
    /// "comment" | "review:APPROVED" | "review:CHANGES_REQUESTED" | "review:COMMENTED" | "inline"
    pub kind: String,
}

#[derive(Serialize, Clone)]
pub struct CheckRun {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub url: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PrDetail {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub merged: bool,
    pub mergeable: Option<bool>,
    pub draft: bool,
    pub head_ref: String,
    pub base_ref: String,
    pub head_sha: String,
    pub author: String,
    pub author_avatar: Option<String>,
    pub html_url: String,
    pub additions: u64,
    pub deletions: u64,
    pub commits: u64,
    pub changed_files: u64,
    pub comments: Vec<PrComment>,
    pub checks: Vec<CheckRun>,
}

#[tauri::command]
pub async fn gh_pr_detail(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    number: u64,
) -> Result<PrDetail, AppError> {
    let token = require_token(&state)?;
    let base = format!("/repos/{owner}/{name}");

    let pr = api_get(&token, &format!("{base}/pulls/{number}")).await?;
    let issue_comments = api_get(&token, &format!("{base}/issues/{number}/comments?per_page=100"))
        .await
        .unwrap_or_default();
    let reviews = api_get(&token, &format!("{base}/pulls/{number}/reviews?per_page=100"))
        .await
        .unwrap_or_default();
    let inline = api_get(&token, &format!("{base}/pulls/{number}/comments?per_page=100"))
        .await
        .unwrap_or_default();
    let sha = pr["head"]["sha"].as_str().unwrap_or_default().to_string();
    let check_runs = api_get(&token, &format!("{base}/commits/{sha}/check-runs?per_page=100"))
        .await
        .unwrap_or_default();

    let mut comments: Vec<PrComment> = Vec::new();
    if let Some(list) = issue_comments.as_array() {
        for c in list {
            comments.push(PrComment {
                author: c["user"]["login"].as_str().unwrap_or_default().to_string(),
                avatar_url: c["user"]["avatar_url"].as_str().map(String::from),
                body: c["body"].as_str().unwrap_or_default().to_string(),
                created_at: c["created_at"].as_str().unwrap_or_default().to_string(),
                path: None,
                line: None,
                diff_hunk: None,
                kind: "comment".into(),
            });
        }
    }
    if let Some(list) = reviews.as_array() {
        for r in list {
            let review_state = r["state"].as_str().unwrap_or_default();
            let body = r["body"].as_str().unwrap_or_default();
            // skip empty COMMENTED shells (their substance is in inline comments)
            if body.is_empty() && review_state == "COMMENTED" {
                continue;
            }
            comments.push(PrComment {
                author: r["user"]["login"].as_str().unwrap_or_default().to_string(),
                avatar_url: r["user"]["avatar_url"].as_str().map(String::from),
                body: body.to_string(),
                created_at: r["submitted_at"].as_str().unwrap_or_default().to_string(),
                path: None,
                line: None,
                diff_hunk: None,
                kind: format!("review:{review_state}"),
            });
        }
    }
    if let Some(list) = inline.as_array() {
        for c in list {
            comments.push(PrComment {
                author: c["user"]["login"].as_str().unwrap_or_default().to_string(),
                avatar_url: c["user"]["avatar_url"].as_str().map(String::from),
                body: c["body"].as_str().unwrap_or_default().to_string(),
                created_at: c["created_at"].as_str().unwrap_or_default().to_string(),
                path: c["path"].as_str().map(String::from),
                line: c["line"].as_u64().or_else(|| c["original_line"].as_u64()),
                diff_hunk: c["diff_hunk"].as_str().map(String::from),
                kind: "inline".into(),
            });
        }
    }
    comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let mut checks: Vec<CheckRun> = Vec::new();
    if let Some(runs) = check_runs["check_runs"].as_array() {
        for run in runs {
            checks.push(CheckRun {
                name: run["name"].as_str().unwrap_or_default().to_string(),
                status: run["status"].as_str().unwrap_or_default().to_string(),
                conclusion: run["conclusion"].as_str().map(String::from),
                url: run["html_url"].as_str().map(String::from),
            });
        }
    }

    Ok(PrDetail {
        number,
        title: pr["title"].as_str().unwrap_or_default().to_string(),
        body: pr["body"].as_str().unwrap_or_default().to_string(),
        state: pr["state"].as_str().unwrap_or_default().to_string(),
        merged: pr["merged"].as_bool().unwrap_or(false),
        mergeable: pr["mergeable"].as_bool(),
        draft: pr["draft"].as_bool().unwrap_or(false),
        head_ref: pr["head"]["ref"].as_str().unwrap_or_default().to_string(),
        base_ref: pr["base"]["ref"].as_str().unwrap_or_default().to_string(),
        head_sha: sha,
        author: pr["user"]["login"].as_str().unwrap_or_default().to_string(),
        author_avatar: pr["user"]["avatar_url"].as_str().map(String::from),
        html_url: pr["html_url"].as_str().unwrap_or_default().to_string(),
        additions: pr["additions"].as_u64().unwrap_or(0),
        deletions: pr["deletions"].as_u64().unwrap_or(0),
        commits: pr["commits"].as_u64().unwrap_or(0),
        changed_files: pr["changed_files"].as_u64().unwrap_or(0),
        comments,
        checks,
    })
}

#[tauri::command]
pub async fn gh_pr_comment(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    number: u64,
    body: String,
) -> Result<(), AppError> {
    let token = require_token(&state)?;
    api_post(
        &token,
        &format!("/repos/{owner}/{name}/issues/{number}/comments"),
        json!({ "body": body }),
    )
    .await
    .map(|_| ())
}

/// `method`: "merge" | "squash" | "rebase". Destructive-ish — the UI
/// confirms explicitly before calling this.
#[tauri::command]
pub async fn gh_pr_merge(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    number: u64,
    method: String,
) -> Result<(), AppError> {
    let token = require_token(&state)?;
    api_put(
        &token,
        &format!("/repos/{owner}/{name}/pulls/{number}/merge"),
        json!({ "merge_method": method }),
    )
    .await
    .map(|_| ())
}

#[derive(Serialize, Clone)]
pub struct CheckSummary {
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
    pub pending: u64,
}

#[tauri::command]
pub async fn gh_pr_checks(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    sha: String,
) -> Result<CheckSummary, AppError> {
    let token = require_token(&state)?;
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
