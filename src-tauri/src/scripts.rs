//! JavaScript injected into the WhatsApp Web page.
//!
//! Each script lives in its own `web/*.js` file (compiled in with `include_str!`)
//! so the page logic stays real, lintable JavaScript instead of giant Rust
//! string literals. They are registered as initialization scripts on the main
//! window in `main.rs`, in the order below, after [`bootstrap`].

/// Clips the page into a rounded shape so the (transparent) OS window shows
/// rounded corners. Must run before the in-page titlebar so its `html`
/// transform is in place when the titlebar's fixed bar is appended.
pub const ROUNDED_CORNERS: &str = include_str!("web/rounded-corners.js");

/// Hides WhatsApp's recurring "background sync" banner.
pub const BACKGROUND_SYNC: &str = include_str!("web/background-sync.js");

/// Redirects web notifications to native OS notifications.
pub const NOTIFICATIONS: &str = include_str!("web/notifications.js");

/// Mirrors WhatsApp's unread count onto the tray (`set_unread`).
pub const UNREAD_BADGE: &str = include_str!("web/unread-badge.js");

/// App-scoped inactivity auto-lock. Reads `window.__ZW.autoLockMinutes`.
pub const AUTO_LOCK: &str = include_str!("web/auto-lock.js");

/// Exposes `window.__ZW.setBlur` so Rust can blur the page while the window is
/// unfocused (privacy for screenshots / thumbnails / screen-sharing).
pub const PRIVACY_BLUR: &str = include_str!("web/privacy-blur.js");

/// The in-page custom titlebar (hamburger menu + window controls).
pub const TITLEBAR: &str = include_str!("web/titlebar.js");

/// Edge/corner resize grips for the frameless (undecorated) main window.
pub const RESIZE_HANDLES: &str = include_str!("web/resize-handles.js");

/// Bridges clipboard-image paste (WebKitGTK can't hand images to the page).
pub const CLIPBOARD_IMAGE: &str = include_str!("web/clipboard-image.js");

/// Routes external links (clicks / window.open) to the system browser.
pub const LINKS: &str = include_str!("web/links.js");

/// Find-in-page overlay (Ctrl/Cmd+F).
pub const FIND: &str = include_str!("web/find.js");

/// Bridges the HTML5 Fullscreen API to the native window.
pub const FULLSCREEN: &str = include_str!("web/fullscreen.js");

/// Logs WhatsApp out and wipes its storage. Eval'd on demand (not an init
/// script) by the non-Linux "forgot password" recovery.
pub const WIPE_SESSION: &str = include_str!("web/wipe-session.js");

/// Injected in minimal mode to disable WebRTC/media APIs that can inflate the
/// WebKit process memory footprint.
pub const DISABLE_MEDIA: &str = include_str!("web/disable-media.js");

/// The first script to run: seeds `window.__ZW` with the config the other
/// scripts read, and primes WhatsApp's persisted theme before the page boots.
pub fn bootstrap(
    wa_theme: &str,
    auto_lock_minutes: u32,
    has_password: bool,
    spellcheck: bool,
) -> String {
    include_str!("web/bootstrap.js")
        .replace("\"__ZW_THEME__\"", &format!("{wa_theme:?}"))
        .replace(
            "\"__ZW_AUTO_LOCK_MINUTES__\"",
            &auto_lock_minutes.to_string(),
        )
        .replace("\"__ZW_HAS_PASSWORD__\"", &has_password.to_string())
        .replace("\"__ZW_SPELLCHECK__\"", &spellcheck.to_string())
}

/// Forces `spellcheck=true` on WhatsApp's composer when enabled.
pub const SPELLCHECK: &str = include_str!("web/spellcheck.js");
