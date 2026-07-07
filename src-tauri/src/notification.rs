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
/// The file must live somewhere the notification daemon can read — we write it
/// into `$XDG_RUNTIME_DIR/zerowhats` ([`avatar_dir`] resolves it). A fixed name
/// means each notification overwrites the previous file instead of accumulating.
/// Returns `None` for anything that isn't a base64 data URL.
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

/// Directory for the notification avatar file. Prefers `$XDG_RUNTIME_DIR/zerowhats`;
/// falls back to the temp dir when `$XDG_RUNTIME_DIR` is unset.
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
            // The app icon (resolved by name against the XDG icon theme). Stays
            // as the notification's app icon; the sender avatar is layered on
            // top via `image-data` below.
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
        //
        // With no avatar we attach the *app* icon as image-data. The named
        // `.icon(...)`/`desktop-entry` route only renders once a packaged build
        // has installed the themed icon file, so on dev / unpackaged runs the
        // notification would otherwise show no icon at all. The icon is embedded
        // in the binary (see `app_icon_image`), so a valid image is always
        // available regardless of install layout.
        let icon_image = match avatar.as_deref() {
            Some(path) => match notify_rust::Image::open(path) {
                Ok(image) => Some(image),
                Err(e) => {
                    log::warn!("failed to load notification avatar '{path}': {e}");
                    app_icon_image()
                }
            },
            None => app_icon_image(),
        };
        if let Some(image) = icon_image {
            notification.hint(notify_rust::Hint::ImageData(image));
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

/// The app icon as a `notify_rust::Image`, attached to notifications that have
/// no sender avatar. Built from the icon PNG embedded in the binary so it works
/// regardless of install layout (dev, AppImage, or packaged) — unlike named-icon
/// resolution, which needs the themed icon file installed on disk.
///
/// We decode the PNG via Tauri's image loader (already a dependency) to raw RGBA,
/// then hand that to `notify_rust::Image::from_rgba`, which the notification
/// daemon renders as inline `image-data`.
#[cfg(target_os = "linux")]
fn app_icon_image() -> Option<notify_rust::Image> {
    const ICON_PNG: &[u8] = include_bytes!("../icons/128x128.png");

    let img = tauri::image::Image::from_bytes(ICON_PNG).ok()?;
    notify_rust::Image::from_rgba(
        img.width() as i32,
        img.height() as i32,
        img.rgba().to_vec(),
    )
    .ok()
}

/// The icon to attach to a notification.
///
/// On Linux the notification daemon resolves an icon by *name* against the XDG
/// icon theme, so we pass the app-id (`com.zaujulio.zerowhats`) — the themed
/// icon is installed under `share/icons/hicolor/*/apps/` by the Linux packagers.
/// On other platforms we fall back to the bundled resource path
/// (`bundle.resources` in tauri.conf.json), which those packagers ship.
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
