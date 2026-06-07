use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Cheap branch lookup (no subprocess) for sidebars
// ---------------------------------------------------------------------------

/// Resolves the `.git` directory for a repo root, following the
/// `gitdir: <path>` indirection used by worktrees and submodules.
fn git_dir(repo: &Path) -> Option<PathBuf> {
    let dot_git = repo.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    if dot_git.is_file() {
        let contents = fs::read_to_string(&dot_git).ok()?;
        let target = contents.strip_prefix("gitdir:")?.trim();
        let path = PathBuf::from(target);
        return Some(if path.is_absolute() { path } else { repo.join(path) });
    }
    None
}

/// Reads the current branch straight from `.git/HEAD` — much cheaper
/// than spawning `git rev-parse` for every sidebar entry.
pub fn current_branch(repo: &Path) -> Option<String> {
    let head = fs::read_to_string(git_dir(repo)?.join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref: refs/heads/") {
        Some(branch) => Some(branch.to_string()),
        // Detached HEAD: show the short hash
        None => Some(head.get(..7).unwrap_or(head).to_string()),
    }
}

/// Walks up from `path` to find the enclosing git repo root, if any.
pub fn repo_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

#[tauri::command]
pub fn git_branch(path: String) -> Option<String> {
    let root = repo_root(Path::new(&path))?;
    current_branch(&root)
}

#[tauri::command]
pub fn git_repo_root(path: String) -> Option<String> {
    repo_root(Path::new(&path)).map(|p| p.to_string_lossy().into_owned())
}

#[derive(Serialize, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub current: bool,
}

#[tauri::command]
pub fn git_branches(repo: String) -> AppResult<Vec<BranchInfo>> {
    let out = run_git_str(&repo, &["branch", "--format=%(refname:short)\t%(HEAD)"], &[])?;
    Ok(out
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut cols = line.split('\t');
            BranchInfo {
                name: cols.next().unwrap_or_default().to_string(),
                current: cols.next().map(|h| h.trim() == "*").unwrap_or(false),
            }
        })
        .collect())
}

/// Checkout (or create with `-b`) a branch. Surfaces real git errors —
/// e.g. "would be overwritten by checkout" — verbatim to the UI.
#[tauri::command]
pub fn git_checkout(repo: String, branch: String, create: bool) -> AppResult<()> {
    let mut args = vec!["checkout"];
    if create {
        args.push("-b");
    }
    args.push(&branch);
    run_git(&repo, &args, &[]).map(|_| ())
}

// ---------------------------------------------------------------------------
// git CLI plumbing
// ---------------------------------------------------------------------------

/// Runs git in `repo`. `ok_codes` lists exit codes that are not errors
/// (e.g. `git diff` exits 1 when differences exist).
fn run_git(repo: &str, args: &[&str], ok_codes: &[i32]) -> AppResult<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| AppError::Pty(format!("failed to run git: {e}")))?;
    let code = output.status.code().unwrap_or(-1);
    if code == 0 || ok_codes.contains(&code) {
        Ok(output.stdout)
    } else {
        Err(AppError::Pty(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

fn run_git_str(repo: &str, args: &[&str], ok_codes: &[i32]) -> AppResult<String> {
    Ok(String::from_utf8_lossy(&run_git(repo, args, ok_codes)?).into_owned())
}

// ---------------------------------------------------------------------------
// Status (porcelain v2)
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct FileChange {
    pub path: String,
    pub orig_path: Option<String>,
    /// Single status letter for the UI badge: M, A, D, R, C, U (untracked), ! (conflict)
    pub status: String,
    pub staged: bool,
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

#[derive(Serialize, Clone, Default)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub oid: Option<String>,
    pub ahead: i32,
    pub behind: i32,
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub conflicts: Vec<String>,
}

/// Parses `git diff --numstat -z` output into path -> (added, removed).
/// Binary files report "-" and map to None.
fn numstat(repo: &str, staged: bool) -> HashMap<String, (Option<u32>, Option<u32>)> {
    let mut args = vec!["diff", "--numstat", "-z"];
    if staged {
        args.insert(1, "--cached");
    }
    let Ok(out) = run_git_str(repo, &args, &[1]) else {
        return HashMap::new();
    };
    let mut map = HashMap::new();
    // -z format: "added\tremoved\tpath\0" — renames use "added\tremoved\t\0old\0new\0"
    let mut fields = out.split('\0').peekable();
    while let Some(entry) = fields.next() {
        if entry.is_empty() {
            continue;
        }
        let mut cols = entry.splitn(3, '\t');
        let added = cols.next().unwrap_or("-");
        let removed = cols.next().unwrap_or("-");
        let path = cols.next().unwrap_or("");
        let path = if path.is_empty() {
            // rename: old path then new path follow as separate NUL fields
            let _old = fields.next();
            fields.next().unwrap_or("")
        } else {
            path
        };
        map.insert(
            path.to_string(),
            (added.parse().ok(), removed.parse().ok()),
        );
    }
    map
}

fn count_lines(repo: &str, rel_path: &str) -> Option<u32> {
    let full = Path::new(repo).join(rel_path);
    let meta = fs::metadata(&full).ok()?;
    if meta.len() > 5_000_000 {
        return None; // don't read huge files just for a badge
    }
    let content = fs::read(&full).ok()?;
    if content.contains(&0) {
        return None; // binary
    }
    Some(content.iter().filter(|&&b| b == b'\n').count() as u32)
}

#[tauri::command]
pub fn git_status(repo: String) -> AppResult<GitStatus> {
    let out = run_git(&repo, &["status", "--porcelain=v2", "--branch", "-z"], &[])?;
    let out = String::from_utf8_lossy(&out);

    let stat_staged = numstat(&repo, true);
    let stat_unstaged = numstat(&repo, false);

    let mut status = GitStatus::default();
    let mut fields = out.split('\0').peekable();

    while let Some(entry) = fields.next() {
        if entry.is_empty() {
            continue;
        }
        if let Some(header) = entry.strip_prefix("# ") {
            if let Some(head) = header.strip_prefix("branch.head ") {
                if head != "(detached)" {
                    status.branch = Some(head.to_string());
                }
            } else if let Some(oid) = header.strip_prefix("branch.oid ") {
                status.oid = Some(oid.get(..7).unwrap_or(oid).to_string());
            } else if let Some(ab) = header.strip_prefix("branch.ab ") {
                for part in ab.split(' ') {
                    if let Some(n) = part.strip_prefix('+') {
                        status.ahead = n.parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_prefix('-') {
                        status.behind = n.parse().unwrap_or(0);
                    }
                }
            }
            continue;
        }

        let kind = entry.chars().next().unwrap_or(' ');
        match kind {
            '1' | '2' => {
                // 1 XY sub mH mI mW hH hI path
                // 2 XY sub mH mI mW hH hI X<score> path (then \0 origPath)
                let cols: Vec<&str> = entry.splitn(if kind == '1' { 9 } else { 10 }, ' ').collect();
                let xy = cols.get(1).unwrap_or(&"..");
                let path = cols.last().unwrap_or(&"").to_string();
                let orig_path = if kind == '2' {
                    fields.next().map(str::to_string)
                } else {
                    None
                };
                let x = xy.chars().next().unwrap_or('.');
                let y = xy.chars().nth(1).unwrap_or('.');
                if x != '.' {
                    let (a, r) = stat_staged.get(&path).copied().unwrap_or((None, None));
                    status.staged.push(FileChange {
                        path: path.clone(),
                        orig_path: orig_path.clone(),
                        status: x.to_string(),
                        staged: true,
                        added: a,
                        removed: r,
                    });
                }
                if y != '.' {
                    let (a, r) = stat_unstaged.get(&path).copied().unwrap_or((None, None));
                    status.unstaged.push(FileChange {
                        path,
                        orig_path,
                        status: y.to_string(),
                        staged: false,
                        added: a,
                        removed: r,
                    });
                }
            }
            'u' => {
                let cols: Vec<&str> = entry.splitn(11, ' ').collect();
                let path = cols.last().unwrap_or(&"").to_string();
                status.conflicts.push(path.clone());
                status.unstaged.push(FileChange {
                    path,
                    orig_path: None,
                    status: "!".into(),
                    staged: false,
                    added: None,
                    removed: None,
                });
            }
            '?' => {
                let path = entry[2..].to_string();
                let lines = count_lines(&repo, &path);
                status.unstaged.push(FileChange {
                    path,
                    orig_path: None,
                    status: "U".into(),
                    staged: false,
                    added: lines,
                    removed: lines.map(|_| 0),
                });
            }
            _ => {}
        }
    }

    if status.branch.is_none() {
        status.branch = current_branch(Path::new(&repo));
    }
    Ok(status)
}

// ---------------------------------------------------------------------------
// Diff / stage / commit / log
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn git_diff(repo: String, path: String, staged: bool, untracked: bool) -> AppResult<String> {
    if untracked {
        // No index entry yet: diff against /dev/null to show the whole file
        return run_git_str(
            &repo,
            &["diff", "--no-index", "--", "/dev/null", &path],
            &[1],
        );
    }
    let mut args = vec!["diff"];
    if staged {
        args.push("--cached");
    }
    args.extend(["--", path.as_str()]);
    run_git_str(&repo, &args, &[1])
}

/// Discards working-tree changes: `git restore` for tracked files,
/// `git clean -f` (delete) for untracked ones. Destructive — the UI
/// must confirm explicitly before calling this.
#[tauri::command]
pub fn git_discard(repo: String, tracked: Vec<String>, untracked: Vec<String>) -> AppResult<()> {
    if !tracked.is_empty() {
        let mut args = vec!["restore", "--"];
        args.extend(tracked.iter().map(String::as_str));
        run_git(&repo, &args, &[])?;
    }
    if !untracked.is_empty() {
        let mut args = vec!["clean", "-f", "--"];
        args.extend(untracked.iter().map(String::as_str));
        run_git(&repo, &args, &[])?;
    }
    Ok(())
}

#[tauri::command]
pub fn git_stage(repo: String, paths: Vec<String>) -> AppResult<()> {
    let mut args = vec!["add", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_git(&repo, &args, &[]).map(|_| ())
}

#[tauri::command]
pub fn git_stage_all(repo: String) -> AppResult<()> {
    run_git(&repo, &["add", "-A"], &[]).map(|_| ())
}

#[tauri::command]
pub fn git_unstage(repo: String, paths: Vec<String>) -> AppResult<()> {
    let mut args = vec!["restore", "--staged", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_git(&repo, &args, &[]).map(|_| ())
}

#[derive(Serialize, Clone)]
pub struct CommitResult {
    pub hash: String,
}

#[tauri::command]
pub fn git_commit(
    repo: String,
    summary: String,
    description: Option<String>,
) -> AppResult<CommitResult> {
    let mut args = vec!["commit", "-m", summary.as_str()];
    let desc = description.unwrap_or_default();
    if !desc.trim().is_empty() {
        args.extend(["-m", desc.as_str()]);
    }
    run_git(&repo, &args, &[])?;
    let hash = run_git_str(&repo, &["rev-parse", "--short", "HEAD"], &[])?;
    Ok(CommitResult {
        hash: hash.trim().to_string(),
    })
}

#[derive(Serialize, Clone)]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

/// Reads `.github/pull_request_template.md` (or `PULL_REQUEST_TEMPLATE.md`
/// at the repo root) if present.
#[tauri::command]
pub fn git_pr_template(repo: String) -> Option<String> {
    let candidates = [
        ".github/pull_request_template.md",
        ".github/PULL_REQUEST_TEMPLATE.md",
        "pull_request_template.md",
        "PULL_REQUEST_TEMPLATE.md",
    ];
    for name in candidates {
        let path = Path::new(&repo).join(name);
        if let Ok(content) = fs::read_to_string(&path) {
            return Some(content);
        }
    }
    None
}

#[tauri::command]
pub fn git_log(repo: String, limit: u32) -> AppResult<Vec<Commit>> {
    let n = limit.to_string();
    let out = run_git_str(
        &repo,
        &["log", "-n", &n, "--format=%H%x1f%h%x1f%an%x1f%aI%x1f%s%x1e"],
        // exit 128 on empty repos (no HEAD yet) — treat as empty history
        &[128],
    )?;
    Ok(out
        .split('\u{1e}')
        .filter_map(|record| {
            let record = record.trim_start_matches(['\n', '\r']);
            let mut cols = record.split('\u{1f}');
            Some(Commit {
                hash: cols.next()?.to_string(),
                short_hash: cols.next()?.to_string(),
                author: cols.next()?.to_string(),
                date: cols.next()?.to_string(),
                subject: cols.next()?.to_string(),
            })
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Stash management
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct StashEntry {
    pub index: u32,
    pub message: String,
    pub branch: String,
    pub date: String,
}

#[tauri::command]
pub fn git_stash_list(repo: String) -> AppResult<Vec<StashEntry>> {
    let out = run_git_str(
        &repo,
        &["stash", "list", "--format=%gd\x1f%gs\x1f%ai\x1e"],
        &[],
    )?;
    Ok(out
        .split('\x1e')
        .filter_map(|record| {
            let record = record.trim();
            if record.is_empty() {
                return None;
            }
            let mut cols = record.split('\x1f');
            let selector = cols.next()?.trim().to_string();
            let subject = cols.next()?.trim().to_string();
            let date = cols.next()?.trim().get(..10).unwrap_or("").to_string();
            let index: u32 = selector
                .trim_start_matches("stash@{")
                .trim_end_matches('}')
                .parse()
                .unwrap_or(0);
            let (branch, message) = if let Some(rest) = subject.strip_prefix("WIP on ") {
                if let Some(i) = rest.find(':') {
                    (rest[..i].trim().to_string(), rest[i + 1..].trim().to_string())
                } else {
                    (rest.to_string(), String::new())
                }
            } else if let Some(rest) = subject.strip_prefix("On ") {
                if let Some(i) = rest.find(':') {
                    (rest[..i].trim().to_string(), rest[i + 1..].trim().to_string())
                } else {
                    (rest.to_string(), String::new())
                }
            } else {
                (String::new(), subject)
            };
            Some(StashEntry { index, message, branch, date })
        })
        .collect())
}

#[tauri::command]
pub fn git_stash_push(repo: String, message: Option<String>) -> AppResult<()> {
    if let Some(ref m) = message {
        if !m.trim().is_empty() {
            return run_git(&repo, &["stash", "push", "-m", m.as_str()], &[]).map(|_| ());
        }
    }
    run_git(&repo, &["stash", "push"], &[]).map(|_| ())
}

#[tauri::command]
pub fn git_stash_apply(repo: String, index: u32) -> AppResult<()> {
    let selector = format!("stash@{{{index}}}");
    run_git(&repo, &["stash", "apply", &selector], &[]).map(|_| ())
}

#[tauri::command]
pub fn git_stash_drop(repo: String, index: u32) -> AppResult<()> {
    let selector = format!("stash@{{{index}}}");
    run_git(&repo, &["stash", "drop", &selector], &[]).map(|_| ())
}

// ---------------------------------------------------------------------------
// Amend last commit
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn git_commit_amend(
    repo: String,
    summary: String,
    description: Option<String>,
) -> AppResult<CommitResult> {
    let mut args = vec!["commit", "--amend", "-m", summary.as_str()];
    let desc = description.unwrap_or_default();
    if !desc.trim().is_empty() {
        args.extend(["-m", desc.as_str()]);
    }
    run_git(&repo, &args, &[])?;
    let hash = run_git_str(&repo, &["rev-parse", "--short", "HEAD"], &[])?;
    Ok(CommitResult { hash: hash.trim().to_string() })
}

// ---------------------------------------------------------------------------
// Cherry-pick
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn git_cherry_pick(repo: String, hash: String) -> AppResult<()> {
    run_git(&repo, &["cherry-pick", &hash], &[]).map(|_| ())
}

// ---------------------------------------------------------------------------
// Blame
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct BlameLine {
    pub line: u32,
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    /// Unix timestamp (seconds) — formatted to a date string in JS
    pub timestamp: i64,
    pub summary: String,
}

#[tauri::command]
pub fn git_blame(repo: String, path: String) -> AppResult<Vec<BlameLine>> {
    let out = run_git(&repo, &["blame", "--line-porcelain", "--", &path], &[])?;
    let text = String::from_utf8_lossy(&out);

    let mut result: Vec<BlameLine> = Vec::new();
    let mut cur_hash = String::new();
    let mut cur_author = String::new();
    let mut cur_ts: i64 = 0;
    let mut cur_summary = String::new();
    let mut cur_line: u32 = 0;

    for line in text.lines() {
        if let Some(val) = line.strip_prefix("author ") {
            cur_author = val.to_string();
        } else if let Some(val) = line.strip_prefix("author-time ") {
            cur_ts = val.trim().parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("summary ") {
            cur_summary = val.to_string();
        } else if line.starts_with('\t') {
            if !cur_hash.is_empty() {
                result.push(BlameLine {
                    line: cur_line,
                    short_hash: cur_hash.get(..7).unwrap_or(&cur_hash).to_string(),
                    hash: cur_hash.clone(),
                    author: cur_author.clone(),
                    timestamp: cur_ts,
                    summary: cur_summary.clone(),
                });
            }
        } else {
            let parts: Vec<&str> = line.splitn(4, ' ').collect();
            if parts.len() >= 3
                && parts[0].len() == 40
                && parts[0].chars().all(|c| c.is_ascii_hexdigit())
            {
                cur_hash = parts[0].to_string();
                cur_line = parts[2].parse().unwrap_or(0);
            }
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Per-hunk staging via git apply --cached
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn git_stage_hunk(repo: String, patch: String) -> AppResult<()> {
    use std::io::Write;
    let mut child = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["apply", "--cached", "--whitespace=nowarn"])
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Pty(format!("git apply: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(patch.as_bytes());
    }
    let out = child
        .wait_with_output()
        .map_err(|e| AppError::Pty(format!("git apply wait: {e}")))?;
    if !out.status.success() {
        return Err(AppError::Pty(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Conflict resolution: accept ours/theirs then stage
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn git_conflict_resolve(repo: String, path: String, resolution: String) -> AppResult<()> {
    let side = match resolution.as_str() {
        "ours" => "--ours",
        "theirs" => "--theirs",
        other => return Err(AppError::Pty(format!("unknown resolution: {other}"))),
    };
    run_git(&repo, &["checkout", side, "--", &path], &[]).map(|_| ())?;
    run_git(&repo, &["add", "--", &path], &[]).map(|_| ())
}
