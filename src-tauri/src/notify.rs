use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

const SOUND_NAME: &str = "Mainbound";

/// macOS resolves custom notification sounds from ~/Library/Sounds —
/// install our bundled brand chime there once.
fn ensure_sound_installed(app: &AppHandle) {
    use tauri::Manager;
    let Some(home) = dirs::home_dir() else { return };
    let dest = home.join("Library/Sounds").join(format!("{SOUND_NAME}.aiff"));
    if dest.exists() {
        return;
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(src) = app.path().resolve(
        "sounds/Mainbound.aiff",
        tauri::path::BaseDirectory::Resource,
    ) {
        let _ = std::fs::copy(src, dest);
    }
}

/// Sends a native notification; falls back to osascript so something
/// always shows even when the notification plugin path fails.
pub fn send(app: &AppHandle, title: &str, body: &str) {
    ensure_sound_installed(app);
    let shown = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .sound(SOUND_NAME)
        .show()
        .is_ok();
    if !shown {
        let esc = |s: &str| s.replace('\\', " ").replace('"', "'");
        let script = format!(
            r#"display notification "{}" with title "{}" sound name "{SOUND_NAME}""#,
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
