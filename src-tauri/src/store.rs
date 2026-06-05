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

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PersistedState {
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub last_workspace: Option<String>,
}

fn state_file() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("dev.tide.app")
        .join("state.json")
}

pub fn load() -> PersistedState {
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
