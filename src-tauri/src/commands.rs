//! The thin IPC layer: every `#[tauri::command]` invokable from the local React
//! windows (Settings / About / Shortcuts / Lock). Each delegates to a domain
//! module (config / lock / window / password). App commands can NOT be invoked
//! from the remote WhatsApp page, so that page talks to the backend via events
//! instead (see `register_web_events` in main.rs).

use tauri::Manager;

use crate::config::{config_path, Config, ConfigPatch, ConfigView, Theme};
use crate::{lock, password, scripts, window};

#[tauri::command]
pub fn get_config(app: tauri::AppHandle) -> ConfigView {
    Config::load(&config_path(&app)).into()
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, patch: ConfigPatch) {
    let path = config_path(&app);
    let mut cfg = Config::load(&path);

    patch.apply_to(&mut cfg);

    let _ = cfg.save(&path);
    apply_autostart(&app, cfg.auto_start);
    lock::apply_auto_lock(&app);
    window::apply_spellcheck(&app, cfg.spellcheck_enabled, cfg.spellcheck_languages.clone());
}

/// Sets (or replaces) the app-lock password. Replacing an existing password
/// requires proving ownership — `current` must verify against the stored hash,
/// otherwise the change is refused. Setting a password for the first time (no
/// stored hash) needs no proof. Returns whether the password was changed.
#[tauri::command]
pub fn set_password(app: tauri::AppHandle, plain: String, current: Option<String>) -> bool {
    let path = config_path(&app);
    let mut cfg = Config::load(&path);

    if plain.is_empty() {
        return false;
    }

    // Guard replacement: an existing password can only be overwritten by someone
    // who knows it (removing it goes through `remove_password`, which also allows
    // an admin override).
    if let Some(existing) = &cfg.password_hash {
        let ok = current.as_deref().is_some_and(|c| password::verify(c, existing));
        if !ok {
            return false;
        }
    }

    cfg.password_hash = password::hash(&plain).ok();

    let _ = cfg.save(&path);
    lock::apply_auto_lock(&app);

    // Reflect the new password state in the tray menu and the injected titlebar.
    crate::tray::refresh(&app);
    window::sync_has_password(&app, cfg.password_hash.is_some());
    true
}

/// Removes the app-lock password. Requires either the current password
/// (`current` verifies against the stored hash) or a successful system-admin
/// authentication (polkit on Linux, admin/sudo elsewhere via `reset_with_admin`).
/// Returns whether the password was removed.
#[tauri::command]
pub fn remove_password(app: tauri::AppHandle, current: Option<String>) -> bool {
    let path = config_path(&app);
    let mut cfg = Config::load(&path);

    let Some(existing) = &cfg.password_hash else {
        return true; // Nothing to remove.
    };

    let by_password = current.as_deref().is_some_and(|c| password::verify(c, existing));
    let authorized = by_password || password::reset_with_admin();
    if !authorized {
        return false;
    }

    cfg.password_hash = None;
    let _ = cfg.save(&path);
    lock::apply_auto_lock(&app);

    crate::tray::refresh(&app);
    window::sync_has_password(&app, false);
    true
}

#[tauri::command]
pub fn reset_password(app: tauri::AppHandle) -> bool {
    if password::reset_with_admin() {
        let path = config_path(&app);
        let mut cfg = Config::load(&path);

        cfg.password_hash = None;
        let _ = cfg.save(&path);

        true
    } else {
        false
    }
}

/// Non-Linux "forgot password" recovery. There's no cross-platform system-auth
/// reset (polkit is Linux-only), so we remove the lock by wiping everything: the
/// config (password included) is deleted and the WhatsApp session is logged out,
/// so a thief can't keep the session either. The user re-pairs from the QR code.
#[tauri::command]
pub fn forget_password_wipe(app: tauri::AppHandle) {
    let _ = std::fs::remove_file(config_path(&app));

    if let Some(main) = app.get_webview_window(window::MAIN_LABEL) {
        let _ = main.eval(scripts::WIPE_SESSION);
    }

    // The config (and its password) is gone, so an empty unlock now succeeds and
    // reveals the main window. The page reload from WIPE_SESSION re-seeds
    // `window.__ZW.hasPassword`; the tray needs an explicit refresh.
    crate::tray::refresh(&app);
    lock::unlock(&app, "");
}

/// The OS we're running on, so the UI can branch on Linux-only affordances such
/// as the polkit-based "forgot password" reset.
#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
pub fn set_theme(app: tauri::AppHandle, theme: Theme) {
    window::apply_theme(&app, theme);
}

#[tauri::command]
pub fn unlock(app: tauri::AppHandle, password: String) -> bool {
    lock::unlock(&app, &password)
}

/// Opens a URL (or `mailto:`) in the user's default handler.
#[tauri::command]
pub fn open_url(app: tauri::AppHandle, url: String) {
    open_external(&app, &url);
}

/// Cross-platform "open in the user's default app" via the opener plugin. Used by
/// `open_url` and the window navigation guard.
pub fn open_external(app: &tauri::AppHandle, url: &str) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url(url, None::<&str>);
}

/// Enables or disables launch-at-login to match the config. Used by `save_config`
/// and at startup.
pub fn apply_autostart(app: &tauri::AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();

    let _ = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
}
