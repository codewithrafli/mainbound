use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Sends a native notification; falls back to osascript so something
/// always shows even when the notification plugin path fails.
pub fn send(app: &AppHandle, title: &str, body: &str) {
    let shown = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .is_ok();
    if !shown {
        let esc = |s: &str| s.replace('\\', " ").replace('"', "'");
        let script = format!(
            r#"display notification "{}" with title "{}" sound name "default""#,
            esc(body),
            esc(title)
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn();
    }
}

#[tauri::command]
pub fn notify(app: AppHandle, title: String, body: String) {
    send(&app, &title, &body);
}
