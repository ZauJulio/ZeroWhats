//! App-lock state and the lock / unlock / auto-lock flow.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
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

/// Auto-lock is driven from Rust, not from JS inside a single webview: an
/// in-page `setTimeout` only sees activity that happens in *that* webview, so
/// typing in Settings (a separate webview) or simply focusing any app window
/// never reset it, and the lock could fire out from under an actively-used
/// window. Rust sees focus changes across every window, so the timer lives
/// here instead.
///
/// `0` means auto-lock is disabled. Stored as millis-since-epoch-ish monotonic
/// ticks isn't needed — we only ever compare elapsed time, so a `Mutex<Instant>`
/// for "when was the last activity" plus the configured duration is enough. The
/// watcher thread (spawned once, see `spawn_watcher`) polls this every second.
static AUTO_LOCK_MINUTES: AtomicU64 = AtomicU64::new(0);
static LAST_ACTIVITY: OnceLock<Mutex<Instant>> = OnceLock::new();
static WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

fn last_activity() -> &'static Mutex<Instant> {
    LAST_ACTIVITY.get_or_init(|| Mutex::new(Instant::now()))
}

/// Resets the inactivity clock. Called on window focus (any app window) and on
/// in-page activity events (mouse/keyboard) forwarded from the main webview.
pub fn record_activity() {
    if let Ok(mut guard) = last_activity().lock() {
        *guard = Instant::now();
    }
}

/// Starts the background thread that checks elapsed idle time once per second
/// and locks the app when it exceeds the configured auto-lock duration.
/// Idempotent — only the first call actually spawns the thread.
pub fn spawn_watcher(app: &AppHandle) {
    if WATCHER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));

        let minutes = AUTO_LOCK_MINUTES.load(Ordering::Relaxed);
        if minutes == 0 || is_locked() {
            continue;
        }

        let elapsed = last_activity().lock().map(|g| g.elapsed()).unwrap_or_default();
        if elapsed >= Duration::from_secs(minutes * 60) {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || lock(&handle));
        }
    });
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

/// Re-arms the inactivity timer from the saved config. Auto-lock is only
/// effective with a password set, so it resolves to 0 (disabled) otherwise.
/// Called after config/password changes so the setting applies live without a
/// reload, and once at startup to start the watcher thread.
pub fn apply_auto_lock(app: &AppHandle) {
    let cfg = Config::load(&config_path(app));
    let minutes = effective_auto_lock_minutes(&cfg);

    AUTO_LOCK_MINUTES.store(minutes as u64, Ordering::Relaxed);
    record_activity();
    spawn_watcher(app);
}

/// Auto-lock minutes that actually apply: 0 (disabled) unless a password is set.
pub fn effective_auto_lock_minutes(cfg: &Config) -> u32 {
    if cfg.password_hash.is_some() {
        cfg.auto_lock_minutes.unwrap_or(0)
    } else {
        0
    }
}
