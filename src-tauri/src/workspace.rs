use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::git;
use crate::state::AppState;
use crate::store::{self, Workspace};

#[derive(Serialize, Clone)]
pub struct RepoInfo {
    pub path: String,
    pub name: String,
    pub branch: Option<String>,
}

const SCAN_MAX_DEPTH: usize = 3;
const SCAN_SKIP: &[&str] = &["node_modules", "target", "dist", ".output", ".nuxt", ".next", "vendor"];

/// Bounded BFS for directories containing a `.git` entry. A repo's
/// subtree is not descended into (nested repos inside repos are rare
/// and usually submodules).
fn scan_repos(root: &Path) -> Vec<RepoInfo> {
    let mut repos = Vec::new();
    let mut queue: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];

    while let Some((dir, depth)) = queue.pop() {
        if dir.join(".git").exists() {
            repos.push(RepoInfo {
                name: dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                branch: git::current_branch(&dir),
                path: dir.to_string_lossy().into_owned(),
            });
            continue;
        }
        if depth >= SCAN_MAX_DEPTH {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !path.is_dir() || name.starts_with('.') || SCAN_SKIP.contains(&name.as_ref()) {
                continue;
            }
            queue.push((path, depth + 1));
        }
    }

    repos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    repos
}

#[tauri::command]
pub fn workspace_list(state: State<'_, AppState>) -> (Vec<Workspace>, Option<String>) {
    let store = state.store.lock();
    (store.workspaces.clone(), store.last_workspace.clone())
}

#[tauri::command]
pub fn workspace_add(state: State<'_, AppState>, path: String) -> AppResult<Workspace> {
    let p = Path::new(&path);
    if !p.is_dir() {
        return Err(AppError::Pty(format!("not a directory: {path}")));
    }
    let mut store = state.store.lock();
    if let Some(existing) = store.workspaces.iter().find(|w| w.path == path) {
        return Ok(existing.clone());
    }
    let workspace = Workspace {
        id: Uuid::new_v4().to_string(),
        name: p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone()),
        path,
    };
    store.workspaces.push(workspace.clone());
    store.last_workspace = Some(workspace.id.clone());
    store::save(&store)?;
    Ok(workspace)
}

#[tauri::command]
pub fn workspace_remove(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let mut store = state.store.lock();
    store.workspaces.retain(|w| w.id != id);
    if store.last_workspace.as_deref() == Some(&id) {
        store.last_workspace = store.workspaces.first().map(|w| w.id.clone());
    }
    store::save(&store)
}

#[tauri::command]
pub fn workspace_set_last(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let mut store = state.store.lock();
    store.last_workspace = Some(id);
    store::save(&store)
}

#[tauri::command]
pub fn repo_scan(path: String) -> Vec<RepoInfo> {
    scan_repos(Path::new(&path))
}

/// Frontend-owned session layout snapshot, persisted for restore-on-launch.
#[tauri::command]
pub fn sessions_save(state: State<'_, AppState>, data: serde_json::Value) -> AppResult<()> {
    let mut store = state.store.lock();
    store.sessions = data;
    store::save(&store)
}

#[tauri::command]
pub fn sessions_load(state: State<'_, AppState>) -> serde_json::Value {
    state.store.lock().sessions.clone()
}
