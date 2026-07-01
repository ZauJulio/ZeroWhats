//! App-lock state and the lock / unlock / auto-lock flow.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::config::{config_path, Config};
use crate::{password, window};

/// Whether the app is currently locked. Shared across the app.
static LOCKED: AtomicBool = AtomicBool::new(false);

pub fn is_locked() -> bool {
    LOCKED.load(Ordering::Relaxed)
}

fn set_locked(value: bool) {
    LOCKED.store(value, Ordering::Relaxed);
}

/// Locks the app: closes the secondary windows and hides the main one. The lock
/// window is shown lazily the next time the app is revealed (see `window`).
pub fn lock(app: &AppHandle) {
    set_locked(true);

    for label in ["settings", "about", "shortcuts"] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.close();
        }
    }

    if let Some(main) = app.get_webview_window(window::MAIN_LABEL) {
        let _ = main.hide();
    }
}

/// Verifies `input` against the stored hash (an empty/absent hash always
/// unlocks) and, on success, reveals the main window. Returns whether it
/// unlocked.
pub fn unlock(app: &AppHandle, input: &str) -> bool {
    let cfg = Config::load(&config_path(app));

    let ok = match &cfg.password_hash {
        Some(hash) => password::verify(input, hash),
        None => true,
    };

    if ok {
        set_locked(false);

        if let Some(lock_win) = app.get_webview_window("lock") {
            let _ = lock_win.close();
        }

        window::show_main(app);
    }

    ok
}

/// Reveals the unlock screen (used when reopening a locked app from the tray).
pub fn show_lock_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("lock") {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }

    // Hidden until the React Lock screen paints and reveals itself (no flash).
    let _ = WebviewWindowBuilder::new(app, "lock", WebviewUrl::App("index.html".into()))
        .title("ZeroWhats")
        .inner_size(400.0, 520.0)
        .resizable(false)
        .maximizable(false)
        .center()
        .always_on_top(true)
        .decorations(false)
        .transparent(true)
        // No `.shadow(true)`: see the comment in `window::build_main` — the
        // compositor's shadow is a plain rectangle and shows up as a square
        // edge around the CSS-rounded `.lock`, which already has its own
        // `box-shadow`.
        .visible(false)
        .background_color(window::transparent_bg())
        .build();
}

/// Re-arms the in-page inactivity timer (`window.__zwArmAutoLock`) from the saved
/// config. Auto-lock is only effective with a password set, so it resolves to 0
/// (disabled) otherwise. Called after config/password changes so the setting
/// applies live without a reload.
pub fn apply_auto_lock(app: &AppHandle) {
    let cfg = Config::load(&config_path(app));
    let minutes = effective_auto_lock_minutes(&cfg);

    if let Some(main) = app.get_webview_window(window::MAIN_LABEL) {
        let _ = main.eval(format!(
            "window.__zwArmAutoLock && window.__zwArmAutoLock({minutes})"
        ));
    }
}

/// Auto-lock minutes that actually apply: 0 (disabled) unless a password is set.
pub fn effective_auto_lock_minutes(cfg: &Config) -> u32 {
    if cfg.password_hash.is_some() {
        cfg.auto_lock_minutes.unwrap_or(0)
    } else {
        0
    }
}
