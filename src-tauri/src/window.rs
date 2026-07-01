//! Window creation and management: the single WhatsApp window (with the page
//! scripts injected and a navigation allow-list) and the frameless React windows
//! (Settings / About / Shortcuts).

use std::path::{Path, PathBuf};
use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};

use crate::config::{config_path, Config, Theme};
use crate::{commands, lock, scripts};

/// Where downloads land: the configured `download_path`, else the OS Downloads
/// folder, else the current dir.
fn download_dir(app: &AppHandle) -> PathBuf {
    let cfg = Config::load(&config_path(app));

    if let Some(path) = cfg.download_path.filter(|p| !p.trim().is_empty()) {
        return PathBuf::from(path);
    }

    app.path()
        .download_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Avoids clobbering an existing file by appending " (1)", " (2)", …
fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let dir = path.parent().map(PathBuf::from).unwrap_or_default();
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (1..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|candidate| !candidate.exists())
        .unwrap_or(path)
}

/// Picks the save path for a download: the suggested filename (or the URL's last
/// segment) under [`download_dir`], de-duplicated.
fn download_target(app: &AppHandle, url: &Url, suggested: &Path) -> PathBuf {
    let name = suggested
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .or_else(|| {
            url.path_segments()
                .and_then(|mut s| s.next_back())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "download".to_string());

    let target = unique_path(download_dir(app).join(name));
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    target
}

/// Fully transparent: these windows are `.transparent(true)` for rounded
/// corners, and the React screen's own CSS already paints the themed
/// background the instant it loads (no flash to guard against) — a non-zero
/// alpha here would paint a square behind the CSS-rounded shape, undoing it.
pub fn transparent_bg() -> tauri::window::Color {
    tauri::window::Color(0, 0, 0, 0)
}

/// Label of the webview hosting WhatsApp Web. Its titlebar is injected into the
/// page (see `web/titlebar.js`) rather than stacking a second webview.
pub const MAIN_LABEL: &str = "main";

const WHATSAPP_URL: &str = "https://web.whatsapp.com";
// A desktop UA so WhatsApp Web serves the full desktop client.
const DESKTOP_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";

/// Builds the single frameless WhatsApp window with the page scripts injected.
/// One webview means input works on every platform (unlike a stacked second
/// webview on Linux).
pub fn build_main(app: &AppHandle, cfg: &Config) -> tauri::Result<()> {
    let start_locked = cfg.password_hash.is_some();
    let auto_lock_minutes = lock::effective_auto_lock_minutes(cfg);
    let nav_app = app.clone();
    let dl_app = app.clone();

    WebviewWindowBuilder::new(
        app,
        MAIN_LABEL,
        WebviewUrl::External(WHATSAPP_URL.parse().unwrap()),
    )
    .title("ZeroWhats")
    .inner_size(1100.0, 800.0)
    .decorations(false)
    .transparent(true)
    // No `.shadow(true)`: the compositor draws that shadow as a plain
    // rectangle around the window's bounds — it has no idea the content
    // inside is rounded — so it shows up as a square edge around the rounded
    // shape. The CSS `box-shadow` in `web/rounded-corners.js` replaces it,
    // since it's painted as part of the page's own rounded box.
    .background_color(transparent_bg())
    // Always created hidden: a transparent+rounded window shown before its
    // first composited frame can get stuck rendering opaque on some Linux
    // compositors (the alpha visual isn't picked up until a frame actually
    // paints). `web/rounded-corners.js` reveals it on the next macrotask
    // once the rounding stylesheet is in place — unless a password is set,
    // in which case the lock screen reveals it later instead.
    .visible(false)
    .user_agent(DESKTOP_USER_AGENT)
    .on_navigation(move |url| allow_navigation(&nav_app, url))
    // WhatsApp triggers its own downloads (blob/anchor); route them to the
    // configured folder so they actually save.
    .on_download(move |_webview, event| {
        if let DownloadEvent::Requested { url, destination } = event {
            let target = download_target(&dl_app, &url, destination);
            *destination = target;
        }
        true
    })
    .initialization_script(scripts::bootstrap(
        cfg.theme.wa_value(),
        auto_lock_minutes,
        start_locked,
    ))
    .initialization_script(scripts::ROUNDED_CORNERS)
    .initialization_script(scripts::BACKGROUND_SYNC)
    .initialization_script(scripts::NOTIFICATIONS)
    .initialization_script(scripts::UNREAD_BADGE)
    .initialization_script(scripts::AUTO_LOCK)
    .initialization_script(scripts::LINKS)
    .initialization_script(scripts::FIND)
    .initialization_script(scripts::FULLSCREEN)
    .initialization_script(scripts::TITLEBAR)
    .build()?;
    Ok(())
}

/// Navigation allow-list (security): only WhatsApp may load inside the app
/// window. Any other http(s) destination — a shared link, an ad — is blocked and
/// opened in the user's real browser instead.
fn allow_navigation(app: &AppHandle, url: &Url) -> bool {
    if is_whatsapp_url(url) {
        return true;
    }

    if matches!(url.scheme(), "http" | "https") {
        commands::open_external(app, url.as_str());
    }

    false
}

/// Whether `url` belongs to WhatsApp (or is an in-page pseudo-scheme). Media is
/// loaded from `*.whatsapp.net`, so those hosts are allowed too.
fn is_whatsapp_url(url: &Url) -> bool {
    if matches!(url.scheme(), "about" | "blob" | "data") {
        return true;
    }

    matches!(url.host_str(), Some(host)
        if host == "web.whatsapp.com"
            || host.ends_with(".whatsapp.com")
            || host.ends_with(".whatsapp.net"))
}

/// Updates `window.__ZW.hasPassword` live so the injected titlebar shows/hides
/// the Lock menu item without a reload (the tray is refreshed separately).
pub fn sync_has_password(app: &AppHandle, has_password: bool) {
    if let Some(main) = app.get_webview_window(MAIN_LABEL) {
        let _ = main.eval(format!(
            "if (window.__ZW) window.__ZW.hasPassword = {has_password};"
        ));
    }
}

/// Pushes the WhatsApp theme into the page and reloads so it takes effect.
pub fn apply_theme(app: &AppHandle, theme: Theme) {
    if let Some(main) = app.get_webview_window(MAIN_LABEL) {
        let wa = theme.wa_value();

        let _ = main.eval(format!(
            "(function(){{ try {{ localStorage.setItem('theme', '\"{wa}\"'); location.reload(); }} catch (e) {{}} }})();"
        ));
    }
}

/// Reveals the main window (or the lock screen if locked).
pub fn show_main(app: &AppHandle) {
    if lock::is_locked() {
        lock::show_lock_window(app);
        return;
    }

    if let Some(main) = app.get_webview_window(MAIN_LABEL) {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
}

/// Opens (or focuses) a frameless React window that renders the screen matching
/// its label.
fn open_react_window(app: &AppHandle, label: &str, title: &str, size: (f64, f64), resizable: bool) {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }

    // Created hidden; the React screen calls `show()` once it has painted, so the
    // window appears fully rendered instead of flashing white → background →
    // content (see src/lib/window.ts `revealWindow`).
    let result = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(size.0, size.1)
        .resizable(resizable)
        .maximizable(false)
        .center()
        .decorations(false)
        .transparent(true)
        // No `.shadow(true)`: see the comment on `build_main` — the compositor's
        // shadow is a plain rectangle and shows up as a square edge around the
        // CSS-rounded `.window`, which already has its own `box-shadow`.
        .visible(false)
        .background_color(transparent_bg())
        .build();
    if let Err(e) = result {
        log::error!("failed to open '{label}' window: {e}");
    }
}

pub fn open_settings(app: &AppHandle) {
    open_react_window(
        app,
        "settings",
        "ZeroWhats — Settings",
        (640.0, 680.0),
        true,
    );
}

pub fn open_about(app: &AppHandle) {
    open_react_window(app, "about", "About ZeroWhats", (400.0, 600.0), false);
}

pub fn open_shortcuts(app: &AppHandle) {
    open_react_window(
        app,
        "shortcuts",
        "Keyboard Shortcuts",
        (400.0, 360.0),
        false,
    );
}
