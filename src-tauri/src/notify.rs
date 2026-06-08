use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

const SOUND_NAME: &str = "Mainbound";

/// macOS only: install the branded chime to ~/Library/Sounds once.
#[cfg(target_os = "macos")]
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

#[cfg(not(target_os = "macos"))]
fn ensure_sound_installed(_app: &AppHandle) {}

/// Sends a native notification. On macOS falls back to osascript when the
/// plugin path fails. On Windows/Linux the plugin handles it directly.
pub fn send(app: &AppHandle, title: &str, body: &str) {
    ensure_sound_installed(app);

    #[cfg(target_os = "macos")]
    let sound = SOUND_NAME;
    #[cfg(not(target_os = "macos"))]
    let sound = "Default";

    let shown = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .sound(sound)
        .show()
        .is_ok();

    // osascript fallback — macOS only
    #[cfg(target_os = "macos")]
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

    // suppress unused warning on non-macOS
    #[cfg(not(target_os = "macos"))]
    let _ = shown;
}

#[tauri::command]
pub fn notify(app: AppHandle, title: String, body: String) {
    send(&app, &title, &body);
}
