use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

#[derive(Serialize, Deserialize, Clone)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
}

fn default_zoom() -> f64 {
    1.0
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PersistedState {
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub last_workspace: Option<String>,
    /// GitHub account logins (tokens live in the Keychain, one entry each)
    #[serde(default)]
    pub gh_accounts: Vec<String>,
    #[serde(default)]
    pub gh_active: Option<String>,
    /// webview zoom factor (⌘+/⌘-/⌘0)
    #[serde(default = "default_zoom")]
    pub zoom: f64,
    /// frontend-owned session layout snapshot (tabs, splits, cwds)
    #[serde(default)]
    pub sessions: serde_json::Value,
}

fn state_file() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("dev.mainbound.app")
        .join("state.json")
}

/// Pre-rename location (the app used to be called "tide").
fn legacy_state_file() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("dev.tide.app")
        .join("state.json")
}

pub fn load() -> PersistedState {
    // one-time migration from the old app identifier
    if !state_file().exists() && legacy_state_file().exists() {
        if let Some(parent) = state_file().parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::copy(legacy_state_file(), state_file());
    }
    fs::read_to_string(state_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(state: &PersistedState) -> AppResult<()> {
    let path = state_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}
