//! Native OS notifications on behalf of the WhatsApp page, plus the mute toggle.

use tauri::AppHandle;
#[cfg(not(target_os = "linux"))]
use tauri::Manager;
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

use crate::config::{config_path, Config, NotificationPrivacy};

/// Shows a native OS notification for the WhatsApp page (its web `Notification`
/// / service-worker `showNotification` calls are redirected here via the
/// `zw://notify` event — see `web/notifications.js`). Native notifications don't
/// register an MPRIS media session, keeping the system media controls clean.
/// The notification-privacy level is enforced here so it holds regardless of
/// which web path fired.
pub fn notify(
    app: &AppHandle,
    title: Option<String>,
    body: Option<String>,
    icon: Option<String>,
) {
    let cfg = Config::load(&config_path(app));

    // Apply the privacy level; `None` means suppress the notification entirely.
    let privacy = cfg.notification_privacy;
    let Some((title, body)) = privacy.apply(title, body) else {
        return;
    };

    let title = title.unwrap_or_else(|| "WhatsApp".to_string());
    let body = body.unwrap_or_default();

    // The sender avatar identifies who wrote — only surface it when previews are
    // allowed (`Full`); `Generic` deliberately hides the sender's identity.
    let avatar = if privacy.shows_preview() {
        icon.as_deref().and_then(save_avatar)
    } else {
        None
    };

    #[cfg(target_os = "linux")]
    {
        show_clickable_linux(app, title, body, avatar);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = &avatar; // silence unused on the plugin path below
        let mut builder = app.notification().builder().title(title).body(body);
        if let Some(icon) = avatar.or_else(|| notification_icon(app)) {
            builder = builder.icon(icon);
        }
        let _ = builder.show();
    }
}

/// Decodes a `data:image/...;base64,...` avatar URL into a file and returns its
/// path — notification daemons take an icon by path/name, not by a data URL.
///
/// The file must live somewhere the *host's* notification daemon can read. Under
/// Flatpak that rules out both the sandbox-private temp dir and a plain
/// `$XDG_RUNTIME_DIR` write: the path advertised inside the sandbox doesn't
/// resolve on the host (the same trap the tray icon hit). We therefore write into
/// a subdir the manifest bind-mounts through at an *identical* host path via
/// `--filesystem=xdg-run/zerowhats:create` — [`avatar_dir`] resolves it. A fixed
/// name means each notification overwrites the previous file instead of
/// accumulating. Returns `None` for anything that isn't a base64 data URL.
fn save_avatar(data_url: &str) -> Option<String> {
    use base64::Engine;
    use std::io::Write;

    let rest = data_url.strip_prefix("data:")?;
    let (mime, b64) = rest.split_once(";base64,")?;
    let ext = match mime {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    };

    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;

    let dir = avatar_dir()?;
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("zerowhats-notify-avatar.{ext}"));

    let mut f = std::fs::File::create(&path).ok()?;
    f.write_all(&bytes).ok()?;

    path.to_str().map(str::to_string)
}

/// Directory for the notification avatar file. Prefers `$XDG_RUNTIME_DIR/zerowhats`,
/// which the Flatpak manifest shares with the host at the same absolute path
/// (`--filesystem=xdg-run/zerowhats:create`); falls back to the temp dir off
/// Flatpak, where the daemon runs unsandboxed and can read it directly.
fn avatar_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|r| std::path::PathBuf::from(r).join("zerowhats"))
        .or_else(|| Some(std::env::temp_dir()))
}

/// Linux: driven through `notify-rust` directly (not the notification plugin)
/// so that clicking the notification focuses the app. The plugin calls
/// `notify_rust::Notification::show()` and throws away the returned handle, but
/// action/close callbacks are only delivered *through* that handle — so with
/// the plugin a click can never reach us. We keep the handle, register a
/// `default` action (fired when the notification body itself is clicked), and
/// wait for it on a detached thread, then bring the main window forward.
#[cfg(target_os = "linux")]
fn show_clickable_linux(app: &AppHandle, title: String, body: String, avatar: Option<String>) {
    let app = app.clone();

    // `wait_for_action` blocks until the notification is actioned or dismissed,
    // so it must run off the main thread.
    std::thread::spawn(move || {
        let mut notification = notify_rust::Notification::new();
        notification
            .summary(&title)
            .body(&body)
            // The app icon (resolved by name against the XDG icon theme, which
            // works under Flatpak). Stays as the notification's app icon; the
            // sender avatar is layered on top via `image-data` below.
            .icon("com.zaujulio.zerowhats")
            // GNOME Shell only routes a notification's `default` click back to
            // the app when it can tie the notification to a desktop entry; the
            // `desktop-entry` hint (the app-id, matching our .desktop file) makes
            // that link, and without it clicking the popup just dismisses it.
            .hint(notify_rust::Hint::DesktopEntry(
                "com.zaujulio.zerowhats".to_string(),
            ))
            // The "default" action has no button; it fires when the user clicks
            // the notification popup itself.
            .action("default", "Open");

        // Sender avatar. GNOME Shell ignores a per-notification icon *path*, so
        // we decode the avatar file and attach it as inline `image-data`, which
        // it honours — this is what makes the photo actually show. Best-effort:
        // if decoding fails the notification still goes out without a photo.
        if let Some(path) = avatar.as_deref() {
            match notify_rust::Image::open(path) {
                Ok(image) => {
                    notification.hint(notify_rust::Hint::ImageData(image));
                }
                Err(e) => log::warn!("failed to load notification avatar: {e}"),
            }
        }

        match notification.show() {
            Ok(handle) => handle.wait_for_action(|action| {
                if action == "default" {
                    let app = app.clone();
                    let _ = app
                        .clone()
                        .run_on_main_thread(move || crate::window::show_main(&app));
                }
            }),
            Err(e) => log::warn!("failed to show notification: {e}"),
        }
    });
}

/// The icon to attach to a notification.
///
/// On Linux the notification daemon resolves an icon by *name* against the XDG
/// icon theme, so we pass the app-id (`com.zaujulio.zerowhats`) — this works
/// under Flatpak, where the themed icon is installed at
/// `/app/share/icons/hicolor/*/apps/` but the binary has no `resource_dir()`
/// copy of the PNG. On other platforms we fall back to the bundled resource
/// path (`bundle.resources` in tauri.conf.json), which those packagers ship.
/// Linux drives notifications through `notify-rust` directly (see
/// [`show_clickable_linux`]) and sets the app-id icon there, so this is only
/// used on the plugin path.
#[cfg(not(target_os = "linux"))]
fn notification_icon(app: &AppHandle) -> Option<String> {
    app.path()
        .resource_dir()
        .ok()
        .map(|d| d.join("icons/128x128.png"))
        .filter(|p| p.exists())
        .and_then(|p| p.to_str().map(str::to_string))
}

/// Tray "Mute" toggle: flips between fully suppressing notifications
/// (`Hidden`) and showing them in full (`Full`). The `Generic` middle level is
/// only reachable from the Settings dropdown. Returns whether notifications are
/// now muted so the tray check item can reflect it.
pub fn toggle_muted(app: &AppHandle) -> bool {
    let path = config_path(app);
    let mut cfg = Config::load(&path);

    cfg.notification_privacy = if cfg.notification_privacy.is_hidden() {
        NotificationPrivacy::Full
    } else {
        NotificationPrivacy::Hidden
    };

    let _ = cfg.save(&path);
    cfg.notification_privacy.is_hidden()
}
