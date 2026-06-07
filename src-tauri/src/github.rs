use std::process::Command;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::store;

const KEYRING_SERVICE: &str = "dev.mainbound.app";
/// pre-rename service name (the app used to be called "tide")
const LEGACY_KEYRING_SERVICE: &str = "dev.tide.app";
const LEGACY_KEYRING_USER: &str = "github-token";
const API: &str = "https://api.github.com";

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("mainbound")
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
    if let Ok(token) = entry_for(login).ok()?.get_password() {
        return Some(token);
    }
    // one-time migration from the old keychain service name
    let user = format!("github-token:{login}");
    let old = keyring::Entry::new(LEGACY_KEYRING_SERVICE, &user).ok()?;
    let token = old.get_password().ok()?;
    if let Ok(new) = entry_for(login) {
        if new.set_password(&token).is_ok() {
            let _ = old.delete_credential();
        }
    }
    Some(token)
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
    let Ok(legacy) = keyring::Entry::new(LEGACY_KEYRING_SERVICE, LEGACY_KEYRING_USER) else { return };
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
pub struct CheckRun {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub url: Option<String>,
}

/// One entry of the GitHub-style conversation timeline.
#[derive(Serialize, Clone)]
pub struct TimelineItem {
    /// "comment" | "review" | "commit" | "event"
    pub kind: String,
    pub author: String,
    pub avatar_url: Option<String>,
    /// OWNER | MEMBER | COLLABORATOR | CONTRIBUTOR | NONE
    pub association: Option<String>,
    pub body: String,
    pub created_at: String,
    pub sha: Option<String>,
    /// approved | changes_requested | commented | dismissed
    pub review_state: Option<String>,
    /// merged | closed | reopened | review_requested | head_ref_force_pushed
    pub event: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ThreadComment {
    /// REST databaseId — used for replies
    pub id: u64,
    pub author: String,
    pub avatar_url: Option<String>,
    pub association: Option<String>,
    pub body: String,
    pub created_at: String,
    pub diff_hunk: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ReviewThread {
    /// GraphQL node id — used for resolve/unresolve
    pub id: String,
    pub resolved: bool,
    pub outdated: bool,
    pub path: String,
    pub line: Option<u64>,
    pub comments: Vec<ThreadComment>,
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
    pub timeline: Vec<TimelineItem>,
    pub threads: Vec<ReviewThread>,
    pub checks: Vec<CheckRun>,
}

async fn graphql(token: &str, query: &str, variables: serde_json::Value) -> AppResult<serde_json::Value> {
    let resp = http()
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .json(&json!({ "query": query, "variables": variables }))
        .send()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?;
    if let Some(errors) = body["errors"].as_array() {
        if let Some(first) = errors.first() {
            return Err(AppError::Pty(format!(
                "github: {}",
                first["message"].as_str().unwrap_or("graphql error")
            )));
        }
    }
    Ok(body["data"].clone())
}

const THREADS_QUERY: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 50) {
        nodes {
          id isResolved isOutdated path line
          comments(first: 50) {
            nodes {
              databaseId body createdAt diffHunk authorAssociation
              author { login avatarUrl }
            }
          }
        }
      }
    }
  }
}"#;

async fn fetch_threads(token: &str, owner: &str, name: &str, number: u64) -> Vec<ReviewThread> {
    let Ok(data) = graphql(
        token,
        THREADS_QUERY,
        json!({ "owner": owner, "name": name, "number": number }),
    )
    .await
    else {
        return Vec::new();
    };
    let mut threads = Vec::new();
    let nodes = data["repository"]["pullRequest"]["reviewThreads"]["nodes"].clone();
    if let Some(list) = nodes.as_array() {
        for t in list {
            let mut comments = Vec::new();
            if let Some(cs) = t["comments"]["nodes"].as_array() {
                for c in cs {
                    comments.push(ThreadComment {
                        id: c["databaseId"].as_u64().unwrap_or(0),
                        author: c["author"]["login"].as_str().unwrap_or_default().to_string(),
                        avatar_url: c["author"]["avatarUrl"].as_str().map(String::from),
                        association: c["authorAssociation"].as_str().map(String::from),
                        body: c["body"].as_str().unwrap_or_default().to_string(),
                        created_at: c["createdAt"].as_str().unwrap_or_default().to_string(),
                        diff_hunk: c["diffHunk"].as_str().map(String::from),
                    });
                }
            }
            threads.push(ReviewThread {
                id: t["id"].as_str().unwrap_or_default().to_string(),
                resolved: t["isResolved"].as_bool().unwrap_or(false),
                outdated: t["isOutdated"].as_bool().unwrap_or(false),
                path: t["path"].as_str().unwrap_or_default().to_string(),
                line: t["line"].as_u64(),
                comments,
            });
        }
    }
    threads
}

fn parse_timeline(items: &serde_json::Value) -> Vec<TimelineItem> {
    let mut timeline = Vec::new();
    let Some(list) = items.as_array() else { return timeline };
    for item in list {
        let event = item["event"].as_str().unwrap_or_default();
        match event {
            "commented" => timeline.push(TimelineItem {
                kind: "comment".into(),
                author: item["user"]["login"].as_str().unwrap_or_default().to_string(),
                avatar_url: item["user"]["avatar_url"].as_str().map(String::from),
                association: item["author_association"].as_str().map(String::from),
                body: item["body"].as_str().unwrap_or_default().to_string(),
                created_at: item["created_at"].as_str().unwrap_or_default().to_string(),
                sha: None,
                review_state: None,
                event: None,
            }),
            "reviewed" => {
                let review_state = item["state"].as_str().unwrap_or_default().to_string();
                let body = item["body"].as_str().unwrap_or_default().to_string();
                // empty "commented" review shells carry no info (their
                // substance lives in the review threads)
                if body.is_empty() && review_state == "commented" {
                    continue;
                }
                timeline.push(TimelineItem {
                    kind: "review".into(),
                    author: item["user"]["login"].as_str().unwrap_or_default().to_string(),
                    avatar_url: item["user"]["avatar_url"].as_str().map(String::from),
                    association: item["author_association"].as_str().map(String::from),
                    body,
                    created_at: item["submitted_at"].as_str().unwrap_or_default().to_string(),
                    sha: None,
                    review_state: Some(review_state),
                    event: None,
                });
            }
            "committed" => timeline.push(TimelineItem {
                kind: "commit".into(),
                author: item["author"]["name"].as_str().unwrap_or_default().to_string(),
                avatar_url: None,
                association: None,
                body: item["message"]
                    .as_str()
                    .unwrap_or_default()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                created_at: item["author"]["date"].as_str().unwrap_or_default().to_string(),
                sha: item["sha"].as_str().map(|s| s.get(..7).unwrap_or(s).to_string()),
                review_state: None,
                event: None,
            }),
            "merged" | "closed" | "reopened" | "review_requested" | "head_ref_force_pushed" => {
                timeline.push(TimelineItem {
                    kind: "event".into(),
                    author: item["actor"]["login"].as_str().unwrap_or_default().to_string(),
                    avatar_url: item["actor"]["avatar_url"].as_str().map(String::from),
                    association: None,
                    body: item["requested_reviewer"]["login"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    created_at: item["created_at"].as_str().unwrap_or_default().to_string(),
                    sha: None,
                    review_state: None,
                    event: Some(event.to_string()),
                });
            }
            _ => {}
        }
    }
    timeline.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    timeline
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
    let timeline_raw = api_get(
        &token,
        &format!("{base}/issues/{number}/timeline?per_page=100"),
    )
    .await
    .unwrap_or_default();
    let threads = fetch_threads(&token, &owner, &name, number).await;
    let sha = pr["head"]["sha"].as_str().unwrap_or_default().to_string();
    let check_runs = api_get(&token, &format!("{base}/commits/{sha}/check-runs?per_page=100"))
        .await
        .unwrap_or_default();

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
        timeline: parse_timeline(&timeline_raw),
        threads,
        checks,
    })
}

#[derive(Serialize, Clone)]
pub struct PrFile {
    pub filename: String,
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    /// unified hunks; None for binary/very large files
    pub patch: Option<String>,
}

#[tauri::command]
pub async fn gh_pr_files(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    number: u64,
) -> Result<Vec<PrFile>, AppError> {
    let token = require_token(&state)?;
    let body = api_get(
        &token,
        &format!("/repos/{owner}/{name}/pulls/{number}/files?per_page=100"),
    )
    .await?;
    Ok(body
        .as_array()
        .map(|files| {
            files
                .iter()
                .map(|f| PrFile {
                    filename: f["filename"].as_str().unwrap_or_default().to_string(),
                    status: f["status"].as_str().unwrap_or_default().to_string(),
                    additions: f["additions"].as_u64().unwrap_or(0),
                    deletions: f["deletions"].as_u64().unwrap_or(0),
                    patch: f["patch"].as_str().map(String::from),
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Reply inside an inline review thread (`comment_id` = thread root).
#[tauri::command]
pub async fn gh_pr_reply_thread(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    number: u64,
    comment_id: u64,
    body: String,
) -> Result<(), AppError> {
    let token = require_token(&state)?;
    api_post(
        &token,
        &format!("/repos/{owner}/{name}/pulls/{number}/comments"),
        json!({ "body": body, "in_reply_to": comment_id }),
    )
    .await
    .map(|_| ())
}

/// Resolve/unresolve a review thread (GraphQL-only operation).
#[tauri::command]
pub async fn gh_pr_resolve_thread(
    state: State<'_, AppState>,
    thread_id: String,
    resolved: bool,
) -> Result<(), AppError> {
    let token = require_token(&state)?;
    let mutation = if resolved {
        "mutation($id: ID!) { resolveReviewThread(input: { threadId: $id }) { thread { isResolved } } }"
    } else {
        "mutation($id: ID!) { unresolveReviewThread(input: { threadId: $id }) { thread { isResolved } } }"
    };
    graphql(&token, mutation, json!({ "id": thread_id })).await.map(|_| ())
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

// ---------------------------------------------------------------------------
// GitHub Issues
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct GhIssue {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub body: String,
    pub author: String,
    pub created_at: String,
    pub html_url: String,
    pub labels: Vec<String>,
    pub comments: u64,
}

#[tauri::command]
pub async fn gh_issues_list(
    state: State<'_, AppState>,
    owner: String,
    name: String,
) -> Result<Vec<GhIssue>, AppError> {
    let token = require_token(&state)?;
    // issues endpoint returns both issues and PRs; filter out PRs
    let body = api_get(
        &token,
        &format!("/repos/{owner}/{name}/issues?state=open&per_page=30&pulls=false"),
    )
    .await?;
    Ok(body
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|i| i["pull_request"].is_null())
                .map(|i| GhIssue {
                    number: i["number"].as_u64().unwrap_or(0),
                    title: i["title"].as_str().unwrap_or_default().to_string(),
                    state: i["state"].as_str().unwrap_or_default().to_string(),
                    body: i["body"].as_str().unwrap_or_default().to_string(),
                    author: i["user"]["login"].as_str().unwrap_or_default().to_string(),
                    created_at: i["created_at"].as_str().unwrap_or_default().to_string(),
                    html_url: i["html_url"].as_str().unwrap_or_default().to_string(),
                    labels: i["labels"]
                        .as_array()
                        .map(|ls| {
                            ls.iter()
                                .filter_map(|l| l["name"].as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    comments: i["comments"].as_u64().unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Convert draft PR to ready for review (GraphQL)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn gh_pr_mark_ready(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    number: u64,
) -> Result<(), AppError> {
    let token = require_token(&state)?;
    // First get the node ID of the PR
    let pr = api_get(&token, &format!("/repos/{owner}/{name}/pulls/{number}")).await?;
    let node_id = pr["node_id"].as_str().unwrap_or_default();
    let mutation = r#"
mutation($id: ID!) {
  markPullRequestReadyForReview(input: { pullRequestId: $id }) {
    pullRequest { isDraft }
  }
}"#;
    graphql(&token, mutation, json!({ "id": node_id }))
        .await
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// CI / workflow logs
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub created_at: String,
    pub html_url: String,
}

#[derive(Serialize, Clone)]
pub struct WorkflowJob {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[tauri::command]
pub async fn gh_workflow_runs(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    branch: Option<String>,
) -> Result<Vec<WorkflowRun>, AppError> {
    let token = require_token(&state)?;
    let branch_q = branch
        .as_deref()
        .map(|b| format!("&branch={b}"))
        .unwrap_or_default();
    let body = api_get(
        &token,
        &format!("/repos/{owner}/{name}/actions/runs?per_page=10{branch_q}"),
    )
    .await?;
    Ok(body["workflow_runs"]
        .as_array()
        .map(|runs| {
            runs.iter()
                .map(|r| WorkflowRun {
                    id: r["id"].as_u64().unwrap_or(0),
                    name: r["name"].as_str().unwrap_or_default().to_string(),
                    status: r["status"].as_str().unwrap_or_default().to_string(),
                    conclusion: r["conclusion"].as_str().map(String::from),
                    created_at: r["created_at"].as_str().unwrap_or_default().to_string(),
                    html_url: r["html_url"].as_str().unwrap_or_default().to_string(),
                })
                .collect()
        })
        .unwrap_or_default())
}

#[tauri::command]
pub async fn gh_workflow_jobs(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    run_id: u64,
) -> Result<Vec<WorkflowJob>, AppError> {
    let token = require_token(&state)?;
    let body = api_get(
        &token,
        &format!("/repos/{owner}/{name}/actions/runs/{run_id}/jobs?per_page=30"),
    )
    .await?;
    Ok(body["jobs"]
        .as_array()
        .map(|jobs| {
            jobs.iter()
                .map(|j| WorkflowJob {
                    id: j["id"].as_u64().unwrap_or(0),
                    name: j["name"].as_str().unwrap_or_default().to_string(),
                    status: j["status"].as_str().unwrap_or_default().to_string(),
                    conclusion: j["conclusion"].as_str().map(String::from),
                    started_at: j["started_at"].as_str().map(String::from),
                    completed_at: j["completed_at"].as_str().map(String::from),
                })
                .collect()
        })
        .unwrap_or_default())
}

#[tauri::command]
pub async fn gh_job_log(
    state: State<'_, AppState>,
    owner: String,
    name: String,
    job_id: u64,
) -> Result<String, AppError> {
    let token = require_token(&state)?;
    // GitHub returns a redirect to a pre-signed S3 URL; reqwest follows it
    let text = http()
        .get(format!("{API}/repos/{owner}/{name}/actions/jobs/{job_id}/logs"))
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| AppError::Pty(format!("github: {e}")))?
        .text()
        .await
        .map_err(|e| AppError::Pty(format!("github log: {e}")))?;
    // Strip ANSI escape codes for clean display
    let clean = strip_ansi(&text);
    // Return last 200 lines to keep it manageable
    let lines: Vec<&str> = clean.lines().collect();
    let start = lines.len().saturating_sub(200);
    Ok(lines[start..].join("\n"))
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                // consume until letter
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
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
