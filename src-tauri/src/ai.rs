use std::io::Write;
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::error::{AppError, AppResult};

const MAX_DIFF_CHARS: usize = 12_000;

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

/// Runs the user's `claude` CLI with the prompt on stdin. Spawned via a
/// login shell so the CLI is found on the user's PATH; cwd is the repo
/// so the model sees project context.
fn run_claude(repo: &str, prompt: &str) -> AppResult<String> {
    let mut child = Command::new("/bin/zsh")
        .args(["-lc", "claude -p --model haiku --output-format text"])
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

/// Pulls the first JSON object out of the model output (tolerates prose
/// or markdown code fences around it).
fn extract_json(raw: &str) -> Option<serde_json::Value> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    serde_json::from_str(raw.get(start..=end)?).ok()
}

#[tauri::command]
pub async fn ai_commit_message(repo: String) -> AppResult<CommitSuggestion> {
    tauri::async_runtime::spawn_blocking(move || {
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

        let raw = run_claude(&repo, &prompt)?;
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
