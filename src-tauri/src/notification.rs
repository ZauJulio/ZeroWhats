//! Native OS notifications on behalf of the WhatsApp page, plus the mute toggle.

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::config::{config_path, Config};

/// Shows a native OS notification for the WhatsApp page (its web `Notification`
/// / service-worker `showNotification` calls are redirected here via the
/// `zw://notify` event — see `web/notifications.js`). Native notifications don't
/// register an MPRIS media session, keeping the system media controls clean.
/// Muting is enforced here so it holds regardless of which web path fired.
pub fn notify(app: &AppHandle, title: Option<String>, body: Option<String>) {
    let cfg = Config::load(&config_path(app));

    if cfg.mute_notifications {
        return;
    }

    // In dev, resource_dir() is src-tauri/; in production it's the installed lib
    // dir where the icon is copied via bundle.resources in tauri.conf.json.
    let icon = app
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("icons/128x128.png"))
        .filter(|p| p.exists())
        .and_then(|p| p.to_str().map(str::to_string));

    let mut builder = app
        .notification()
        .builder()
        .title(title.unwrap_or_else(|| "WhatsApp".to_string()))
        .body(body.unwrap_or_default());

    if let Some(icon_path) = icon {
        builder = builder.icon(icon_path);
    }

    let _ = builder.show();
}

/// Flips the muted state and persists it. The next `notify` reads the fresh
/// value, so muting takes effect immediately. Returns the new value so the
/// caller (the tray check item) can reflect it.
pub fn toggle_muted(app: &AppHandle) -> bool {
    let path = config_path(app);
    let mut cfg = Config::load(&path);
    cfg.mute_notifications = !cfg.mute_notifications;

    let _ = cfg.save(&path);
    cfg.mute_notifications
}
