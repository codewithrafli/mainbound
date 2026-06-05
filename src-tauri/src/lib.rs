mod error;
mod git;
mod pty;
mod state;
mod store;
mod workspace;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .manage(AppState::new())
    .invoke_handler(tauri::generate_handler![
      pty::pty_spawn,
      pty::pty_write,
      pty::pty_resize,
      pty::pty_kill,
      pty::pty_list,
      git::git_branch,
      git::git_status,
      git::git_diff,
      git::git_stage,
      git::git_stage_all,
      git::git_unstage,
      git::git_commit,
      git::git_log,
      workspace::workspace_list,
      workspace::workspace_add,
      workspace::workspace_remove,
      workspace::workspace_set_last,
      workspace::repo_scan,
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
