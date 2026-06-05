mod ai;
mod error;
mod git;
mod github;
mod pty;
mod state;
mod store;
mod workspace;

use state::AppState;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::Emitter;

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
  let app_menu = SubmenuBuilder::new(app, "tide")
    .about(None)
    .separator()
    .services()
    .separator()
    .hide()
    .hide_others()
    .show_all()
    .separator()
    .quit()
    .build()?;

  let edit = SubmenuBuilder::new(app, "Edit")
    .undo()
    .redo()
    .separator()
    .cut()
    .copy()
    .paste()
    .select_all()
    .build()?;

  let shell = SubmenuBuilder::new(app, "Shell")
    .item(
      &MenuItemBuilder::with_id("new-session", "New Session")
        .accelerator("CmdOrCtrl+T")
        .build(app)?,
    )
    .separator()
    .item(
      &MenuItemBuilder::with_id("split-right", "Split Right")
        .accelerator("CmdOrCtrl+D")
        .build(app)?,
    )
    .item(
      &MenuItemBuilder::with_id("split-down", "Split Down")
        .accelerator("Shift+CmdOrCtrl+D")
        .build(app)?,
    )
    .separator()
    .item(
      &MenuItemBuilder::with_id("close-session", "Close Pane")
        .accelerator("CmdOrCtrl+W")
        .build(app)?,
    )
    .build()?;

  let view = SubmenuBuilder::new(app, "View")
    .item(
      &MenuItemBuilder::with_id("view-terminal", "Terminal")
        .accelerator("CmdOrCtrl+1")
        .build(app)?,
    )
    .item(
      &MenuItemBuilder::with_id("view-changes", "File Changes")
        .accelerator("CmdOrCtrl+2")
        .build(app)?,
    )
    .separator()
    .fullscreen()
    .build()?;

  let window = SubmenuBuilder::new(app, "Window").minimize().build()?;

  let menu = MenuBuilder::new(app)
    .items(&[&app_menu, &edit, &shell, &view, &window])
    .build()?;
  app.set_menu(menu)?;
  app.on_menu_event(|app, event| {
    let _ = app.emit("menu://action", event.id().0.clone());
  });
  Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .manage(AppState::new())
    .invoke_handler(tauri::generate_handler![
      ai::ai_commit_message,
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
      github::gh_status,
      github::gh_set_pat,
      github::gh_logout,
      github::gh_device_start,
      github::gh_device_poll,
      github::gh_remote_info,
      github::gh_push,
      github::gh_pull,
      github::gh_list_prs,
      github::gh_create_pr,
      github::gh_pr_checks,
      workspace::workspace_list,
      workspace::workspace_add,
      workspace::workspace_remove,
      workspace::workspace_set_last,
      workspace::repo_scan,
    ])
    .setup(|app| {
      build_menu(app.handle())?;
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
