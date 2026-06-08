mod ai;
mod error;
mod notify;
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
  // macOS has a named app menu; Windows/Linux use a simpler top-level
  #[cfg(target_os = "macos")]
  let app_menu = SubmenuBuilder::new(app, "Mainbound")
    .about(None)
    .separator()
    .item(&MenuItemBuilder::with_id("check-updates", "Check for Updates…").build(app)?)
    .item(&MenuItemBuilder::with_id("notify-test", "Test Notification").build(app)?)
    .separator()
    .services()
    .separator()
    .hide()
    .hide_others()
    .show_all()
    .separator()
    .quit()
    .build()?;

  #[cfg(not(target_os = "macos"))]
  let app_menu = SubmenuBuilder::new(app, "Mainbound")
    .item(&MenuItemBuilder::with_id("check-updates", "Check for Updates…").build(app)?)
    .item(&MenuItemBuilder::with_id("notify-test", "Test Notification").build(app)?)
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
    .separator()
    .item(
      &MenuItemBuilder::with_id("find", "Find")
        .accelerator("CmdOrCtrl+F")
        .build(app)?,
    )
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
    .item(
      &MenuItemBuilder::with_id("command-palette", "Command Palette")
        .accelerator("CmdOrCtrl+K")
        .build(app)?,
    )
    .separator()
    .item(
      &MenuItemBuilder::with_id("zoom-in", "Zoom In")
        .accelerator("CmdOrCtrl+=")
        .build(app)?,
    )
    .item(
      &MenuItemBuilder::with_id("zoom-out", "Zoom Out")
        .accelerator("CmdOrCtrl+-")
        .build(app)?,
    )
    .item(
      &MenuItemBuilder::with_id("zoom-reset", "Reset Zoom")
        .accelerator("CmdOrCtrl+0")
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
    let id = event.id().0.as_str();
    match id {
      "notify-test" => {
        notify::send(app, "Mainbound", "Notifications are working 🎉");
      }
      // zoom is handled natively — no webview round-trip needed
      "zoom-in" | "zoom-out" | "zoom-reset" => {
        use tauri::Manager;
        if let Some(state) = app.try_state::<AppState>() {
          let mut zoom = state.zoom.lock();
          *zoom = match id {
            "zoom-in" => (*zoom + 0.1).min(3.0),
            "zoom-out" => (*zoom - 0.1).max(0.5),
            _ => 1.0,
          };
          if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_zoom(*zoom);
          }
          // persist across restarts
          let mut persisted = state.store.lock();
          persisted.zoom = *zoom;
          let _ = store::save(&persisted);
        }
      }
      _ => {
        let _ = app.emit("menu://action", id.to_string());
      }
    }
  });
  Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .manage(AppState::new())
    .invoke_handler(tauri::generate_handler![
      ai::ai_commit_message,
      ai::ai_pr_message,
      ai::ai_branch_name,
      notify::notify,
      pty::pty_spawn,
      pty::pty_write,
      pty::pty_resize,
      pty::pty_kill,
      pty::pty_list,
      git::git_branch,
      git::git_repo_root,
      git::git_branches,
      git::git_checkout,
      git::git_discard,
      git::git_status,
      git::git_diff,
      git::git_stage,
      git::git_stage_all,
      git::git_unstage,
      git::git_commit,
      git::git_log,
      git::git_pr_template,
      git::git_stash_list,
      git::git_stash_push,
      git::git_stash_apply,
      git::git_stash_drop,
      git::git_commit_amend,
      git::git_cherry_pick,
      git::git_blame,
      git::git_stage_hunk,
      git::git_conflict_resolve,
      github::gh_status,
      github::gh_set_pat,
      github::gh_switch,
      github::gh_logout,
      github::gh_device_start,
      github::gh_device_poll,
      github::gh_remote_info,
      github::gh_push,
      github::gh_pull,
      github::gh_list_prs,
      github::gh_create_pr,
      github::gh_pr_checks,
      github::gh_pr_for_branch,
      github::gh_pr_reviews,
      github::gh_pr_detail,
      github::gh_pr_files,
      github::gh_pr_comment,
      github::gh_pr_reply_thread,
      github::gh_pr_resolve_thread,
      github::gh_pr_merge,
      github::gh_issues_list,
      github::gh_pr_mark_ready,
      github::gh_workflow_runs,
      github::gh_workflow_jobs,
      github::gh_job_log,
      workspace::workspace_list,
      workspace::workspace_add,
      workspace::workspace_remove,
      workspace::workspace_set_last,
      workspace::repo_scan,
      workspace::sessions_save,
      workspace::sessions_load,
      workspace::settings_save,
      workspace::settings_load,
    ])
    .setup(|app| {
      build_menu(app.handle())?;

      use tauri::Manager;
      let window = app.get_webview_window("main");

      // macOS: overlay title bar + hidden title (traffic lights over our rail)
      #[cfg(target_os = "macos")]
      if let Some(w) = &window {
        use tauri::TitleBarStyle;
        let _ = w.set_title_bar_style(TitleBarStyle::Overlay);
        let _ = w.set_title(""); // hides the text next to traffic lights
      }

      // Windows: make sure the window is visible and properly sized
      #[cfg(target_os = "windows")]
      if let Some(w) = &window {
        let _ = w.show();
        let _ = w.set_focus();
      }

      // re-apply persisted zoom
      {
        let state = app.state::<AppState>();
        let zoom = *state.zoom.lock();
        if (zoom - 1.0).abs() > f64::EPSILON {
          if let Some(w) = &window {
            let _ = w.set_zoom(zoom);
          }
        }
      }
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
