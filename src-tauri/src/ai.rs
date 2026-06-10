use std::io::Write;
use std::process::{Command, Stdio};
use std::{env, fs};

use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const MAX_DIFF_CHARS: usize = 12_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AiProvider {
    Claude,
    Codex,
}

impl AiProvider {
    fn parse(value: Option<String>) -> Self {
        let normalized = value.map(|v| v.trim().to_ascii_lowercase());
        match normalized.as_deref() {
            Some("codex") => Self::Codex,
            Some("claude") | Some("") | None => Self::Claude,
            Some(_) => Self::Claude,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct CommitSuggestion {
    pub summary: String,
    pub description: String,
}

fn run_git(repo: &str, args: &[&str]) -> AppResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| AppError::Pty(format!("failed to run git: {e}")))?;
    if !output.status.success() {
        return Err(AppError::Pty(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

/// Builds a login-shell command that runs `script` with the user's PATH/rc
/// loaded, so CLIs installed via nvm/homebrew/etc. are found.
/// - macOS/Linux: `$SHELL -lc "<script>"` (falls back to /bin/sh)
/// - Windows: `powershell -Command "<script>"`
fn login_shell(script: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-Command", script]);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut cmd = Command::new(shell);
        cmd.args(["-lc", script]);
        cmd
    }
}

/// Runs the user's `claude` CLI with the prompt on stdin. Spawned via a
/// login shell so the CLI is found on the user's PATH; cwd is the repo
/// so the model sees project context.
fn run_claude(repo: &str, prompt: &str) -> AppResult<String> {
    let mut child = login_shell("claude -p --model haiku --output-format text")
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Pty(format!("failed to start shell: {e}")))?;

    child
        .stdin
        .take()
        .ok_or_else(|| AppError::Pty("failed to open claude stdin".into()))?
        .write_all(prompt.as_bytes())?;

    let output = child
        .wait_with_output()
        .map_err(|e| AppError::Pty(format!("claude failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("command not found") {
            return Err(AppError::Pty(
                "claude CLI not found — install Claude Code or add it to PATH".into(),
            ));
        }
        return Err(AppError::Pty(format!("claude: {}", stderr.trim())));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_codex(repo: &str, prompt: &str) -> AppResult<String> {
    let out_path = env::temp_dir().join(format!("mainbound-codex-{}.txt", Uuid::new_v4()));
    let command = format!(
        "codex exec --output-last-message {}",
        shell_quote(out_path.to_string_lossy().as_ref())
    );

    let mut child = login_shell(&command)
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Pty(format!("failed to start shell: {e}")))?;

    child
        .stdin
        .take()
        .ok_or_else(|| AppError::Pty("failed to open codex stdin".into()))?
        .write_all(prompt.as_bytes())?;

    let output = child
        .wait_with_output()
        .map_err(|e| AppError::Pty(format!("codex failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("command not found") {
            return Err(AppError::Pty(
                "codex CLI not found — install Codex or add it to PATH".into(),
            ));
        }
        return Err(AppError::Pty(format!("codex: {}", stderr.trim())));
    }

    let raw = fs::read_to_string(&out_path)
        .map_err(|e| AppError::Pty(format!("failed to read codex output: {e}")))?;
    let _ = fs::remove_file(&out_path);
    Ok(raw)
}

fn run_ai_cli(provider: AiProvider, repo: &str, prompt: &str) -> AppResult<String> {
    match provider {
        AiProvider::Claude => run_claude(repo, prompt),
        AiProvider::Codex => run_codex(repo, prompt),
    }
}

/// Pulls the first JSON object out of the model output (tolerates prose
/// or markdown code fences around it).
fn extract_json(raw: &str) -> Option<serde_json::Value> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    serde_json::from_str(raw.get(start..=end)?).ok()
}

#[derive(Serialize, Clone)]
pub struct PrSuggestion {
    pub title: String,
    pub body: String,
}

#[tauri::command]
pub async fn ai_pr_message(repo: String, base: String, provider: Option<String>) -> AppResult<PrSuggestion> {
    tauri::async_runtime::spawn_blocking(move || {
        let provider = AiProvider::parse(provider);
        // base may only exist on the remote
        let base_ref = if run_git(&repo, &["rev-parse", "--verify", &base]).is_ok() {
            base.clone()
        } else {
            format!("origin/{base}")
        };

        let commits = run_git(&repo, &["log", "--oneline", &format!("{base_ref}..HEAD")])?;
        if commits.trim().is_empty() {
            return Err(AppError::Pty(format!(
                "no commits ahead of {base} — commit and push your work first"
            )));
        }
        let range = format!("{base_ref}...HEAD");
        let stat = run_git(&repo, &["diff", "--stat", &range])?;
        let mut diff = run_git(&repo, &["diff", &range])?;
        if diff.len() > MAX_DIFF_CHARS {
            let mut cut = MAX_DIFF_CHARS;
            while !diff.is_char_boundary(cut) {
                cut -= 1;
            }
            diff.truncate(cut);
            diff.push_str("\n[... diff truncated ...]");
        }

        let prompt = format!(
            "Generate a pull request title and description for the branch changes below.\n\
             Reply with ONLY a JSON object, no other text:\n\
             {{\"title\": \"<concise PR title, max 72 chars, conventional commit style>\", \"body\": \"<markdown description: a short Summary paragraph, then a '## Changes' bullet list of the key changes. Keep it factual and scannable.>\"}}\n\n\
             Commits on this branch:\n{commits}\n\nFile stats:\n{stat}\n\nDiff (may be truncated):\n{diff}"
        );

        let raw = run_ai_cli(provider, &repo, &prompt)?;
        let json = extract_json(&raw)
            .ok_or_else(|| AppError::Pty(format!("unexpected AI output: {}", raw.trim())))?;
        let title = json["title"].as_str().unwrap_or_default().trim().to_string();
        if title.is_empty() {
            return Err(AppError::Pty("AI returned an empty title".into()));
        }
        Ok(PrSuggestion {
            title,
            body: json["body"].as_str().unwrap_or_default().trim().to_string(),
        })
    })
    .await
    .map_err(|e| AppError::Pty(e.to_string()))?
}

#[tauri::command]
pub async fn ai_branch_name(repo: String, task: String, provider: Option<String>) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let provider = AiProvider::parse(provider);
        // Give the model context: current branch + unstaged file names
        let current = run_git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap_or_default();
        let files = run_git(&repo, &["status", "--short"])
            .unwrap_or_default();
        let file_list: String = files
            .lines()
            .take(20)
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Generate a git branch name for the following task.\n\
             Reply with ONLY the branch name, no explanation, no quotes.\n\
             Rules: lowercase, hyphens only, max 50 chars, start with feat/, fix/, chore/, refactor/, or docs/.\n\
             Current branch: {current}\n\
             Changed files:\n{file_list}\n\n\
             Task: {task}"
        );

        let raw = run_ai_cli(provider, &repo, &prompt)?;
        let name = raw.trim()
            .trim_matches('`')
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        if name.is_empty() || name.contains('\n') {
            return Err(AppError::Pty("AI returned an invalid branch name".into()));
        }
        Ok(name)
    })
    .await
    .map_err(|e| AppError::Pty(e.to_string()))?
}

#[tauri::command]
pub async fn ai_commit_message(repo: String, provider: Option<String>) -> AppResult<CommitSuggestion> {
    tauri::async_runtime::spawn_blocking(move || {
        let provider = AiProvider::parse(provider);
        let stat = run_git(&repo, &["diff", "--cached", "--stat"])?;
        if stat.trim().is_empty() {
            return Err(AppError::Pty(
                "nothing staged — stage some changes first".into(),
            ));
        }
        let mut diff = run_git(&repo, &["diff", "--cached"])?;
        if diff.len() > MAX_DIFF_CHARS {
            // keep a char-boundary-safe prefix
            let mut cut = MAX_DIFF_CHARS;
            while !diff.is_char_boundary(cut) {
                cut -= 1;
            }
            diff.truncate(cut);
            diff.push_str("\n[... diff truncated ...]");
        }

        let prompt = format!(
            "Generate a git commit message for the staged changes below.\n\
             Reply with ONLY a JSON object, no other text:\n\
             {{\"summary\": \"<conventional commit summary, imperative, max 72 chars>\", \"description\": \"<optional body explaining what and why; empty string if the change is trivial>\"}}\n\
             Use a conventional commit prefix (feat:, fix:, chore:, refactor:, docs:, style:, test:).\n\n\
             File stats:\n{stat}\n\nDiff (may be truncated):\n{diff}"
        );

        let raw = run_ai_cli(provider, &repo, &prompt)?;
        let json = extract_json(&raw)
            .ok_or_else(|| AppError::Pty(format!("unexpected AI output: {}", raw.trim())))?;
        let summary = json["summary"].as_str().unwrap_or_default().trim().to_string();
        if summary.is_empty() {
            return Err(AppError::Pty("AI returned an empty summary".into()));
        }
        Ok(CommitSuggestion {
            summary,
            description: json["description"].as_str().unwrap_or_default().trim().to_string(),
        })
    })
    .await
    .map_err(|e| AppError::Pty(e.to_string()))?
}
