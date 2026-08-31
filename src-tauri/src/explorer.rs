use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

const MAX_PREVIEW_BYTES: u64 = 1024 * 1024;

#[derive(Serialize)]
pub struct ExplorerEntry {
    name: String,
    path: String,
    kind: String,
    size: Option<u64>,
    modified: Option<u64>,
}

#[derive(Serialize)]
pub struct ExplorerFile {
    path: String,
    content: String,
    size: u64,
    truncated: bool,
}

fn ignored_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".nuxt"
            | ".output"
            | ".next"
            | ".DS_Store"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | "coverage"
    )
}

fn workspace_root(root: &str) -> AppResult<PathBuf> {
    let path = Path::new(root).canonicalize()?;
    if !path.is_dir() {
        return Err(AppError::Pty("workspace root is not a directory".into()));
    }
    Ok(path)
}

fn safe_target(root: &Path, rel: Option<&str>) -> AppResult<PathBuf> {
    let target = match rel {
        Some(path) if !path.is_empty() => root.join(path),
        _ => root.to_path_buf(),
    };
    let canonical = target.canonicalize()?;
    if !canonical.starts_with(root) {
        return Err(AppError::Pty("path escapes workspace".into()));
    }
    Ok(canonical)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[tauri::command]
pub fn explorer_list_dir(root: String, path: Option<String>) -> AppResult<Vec<ExplorerEntry>> {
    let root = workspace_root(&root)?;
    let dir = safe_target(&root, path.as_deref())?;
    if !dir.is_dir() {
        return Err(AppError::Pty("path is not a directory".into()));
    }

    let mut entries = Vec::new();
    for item in fs::read_dir(&dir)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().to_string();
        if ignored_name(&name) {
            continue;
        }

        let file_type = item.file_type()?;
        if !(file_type.is_dir() || file_type.is_file()) {
            continue;
        }

        let metadata = item.metadata().ok();
        entries.push(ExplorerEntry {
            name,
            path: relative_path(&root, &item.path()),
            kind: if file_type.is_dir() { "dir" } else { "file" }.into(),
            size: metadata
                .as_ref()
                .filter(|_| file_type.is_file())
                .map(|m| m.len()),
            modified: metadata
                .and_then(|m| m.modified().ok())
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs()),
        });
    }

    entries.sort_by(|a, b| {
        let a_dir = a.kind == "dir";
        let b_dir = b.kind == "dir";
        b_dir
            .cmp(&a_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
pub fn explorer_read_file(root: String, path: String) -> AppResult<ExplorerFile> {
    let root = workspace_root(&root)?;
    let file_path = safe_target(&root, Some(&path))?;
    if !file_path.is_file() {
        return Err(AppError::Pty("path is not a file".into()));
    }

    let size = fs::metadata(&file_path)?.len();
    let mut file = fs::File::open(&file_path)?;
    let mut sample = [0_u8; 8192];
    let sample_len = file.read(&mut sample)?;
    if sample[..sample_len].contains(&0) {
        return Err(AppError::Pty("binary files cannot be previewed".into()));
    }

    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(MAX_PREVIEW_BYTES).read_to_end(&mut bytes)?;
    let content = String::from_utf8_lossy(&bytes).to_string();

    Ok(ExplorerFile {
        path,
        content,
        size,
        truncated: size > MAX_PREVIEW_BYTES,
    })
}
