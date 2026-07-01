// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod lock;
mod notification;
mod password;
mod scripts;
mod tray;
mod window;

use config::{config_path, Config};
use tauri::{Listener, Manager, WindowEvent};

fn main() {
    let mut builder = tauri::Builder::default();

    // Single-instance must be the FIRST plugin: a second launch re-focuses the
    // existing window instead of starting a new process.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show_main(app);
        }));
    }

    builder
        .plugin(
            // Registered early so every later plugin/setup step can log. Writes to
            // both stdout (dev) and the OS log dir (release builds have no
            // terminal attached, so this is the only way to get diagnostics back
            // from a user-reported bug).
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: None,
                    }),
                ])
                .build(),
        )
        .plugin(
            tauri_plugin_window_state::Builder::default()
                // Only the main window's geometry is worth remembering; the modal
                // screens stay centered at their fixed sizes.
                .with_denylist(&["settings", "about", "shortcuts", "lock"])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::set_password,
            commands::reset_password,
            commands::forget_password_wipe,
            commands::get_platform,
            commands::set_theme,
            commands::unlock,
            commands::open_url,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let cfg = Config::load(&config_path(&handle));

            apply_environment(&cfg);
            commands::apply_autostart(&handle, cfg.auto_start);

            window::build_main(&handle, &cfg)?;
            register_web_events(&handle);

            if cfg.password_hash.is_some() {
                lock::lock(&handle);
                lock::show_lock_window(&handle);
            }

            tray::build(&handle)?;
            reassert_tray_menu(&handle);
            Ok(())
        })
        .on_window_event(|win, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if win.label() == window::MAIN_LABEL {
                    let app = win.app_handle();
                    let cfg = Config::load(&config_path(app));
                    if cfg.lock_on_close && cfg.password_hash.is_some() {
                        lock::lock(app);
                    } else {
                        let _ = win.hide();
                    }
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Re-asserts the tray menu shortly after the initial build. GNOME's
/// AppIndicator extension reads the icon + DBusMenu layout off the session bus;
/// if that registration races the extension's own startup (login, or right
/// after waking from sleep), the first `set_menu` can be missed with no error
/// surfaced back to us. A second assert a moment later is the standard
/// mitigation. Linux-only: macOS/Windows use native menu APIs and don't have
/// this race.
#[cfg(target_os = "linux")]
fn reassert_tray_menu(app: &tauri::AppHandle) {
    let handle = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));

        let main_handle = handle.clone();
        if let Err(e) = handle.run_on_main_thread(move || tray::refresh(&main_handle)) {
            log::warn!("tray re-assert: run_on_main_thread failed: {e}");
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn reassert_tray_menu(_app: &tauri::AppHandle) {}

/// Applies config that has to be set as process environment before the webview
/// starts (proxy, hardware acceleration, Linux WebKit rendering).
fn apply_environment(cfg: &Config) {
    if cfg.proxy_enabled && !cfg.proxy_url.is_empty() {
        std::env::set_var("http_proxy", &cfg.proxy_url);
        std::env::set_var("https_proxy", &cfg.proxy_url);
    }

    if !cfg.hardware_acceleration {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    #[cfg(target_os = "linux")]
    apply_linux_rendering();
}

/// WebKitGTK's DMABUF renderer leaves the window blank on many Linux/Wayland +
/// GPU-driver combinations (the page loads and runs, but nothing ever paints) —
/// disabling it is the standard, reliable fix. An explicit user override of the
/// env var is honoured.
///
/// Note: forcing the integrated GPU (Mesa EGL) was tried to recover GPU
/// compositing on hybrid Intel+NVIDIA laptops, but it blanked the window on this
/// setup (cross-GPU buffer sharing with the compositor), so we keep the simple,
/// always-renders path. On such laptops the trade-off is real: the renderer that
/// paints is the slower one.
#[cfg(target_os = "linux")]
fn apply_linux_rendering() {
    // Escape hatch: ZW_FORCE_SOFTWARE=1 keeps the slow-but-always-safe software
    // path (disable WebKit's DMABUF renderer), for setups where GPU acceleration
    // leaves the window blank.
    if std::env::var_os("ZW_FORCE_SOFTWARE").is_some() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        return;
    }

    // On NVIDIA + integrated-GPU laptops, WebKitGTK's DMABUF renderer blanks on
    // the NVIDIA proprietary path, so it's commonly disabled system-wide — which
    // drops WebKit to slow software compositing. Instead, pin THIS process's whole
    // GL/EGL/GBM stack to the integrated GPU (Mesa), where DMABUF works, and enable
    // it: hardware-accelerated WebKit, no blank. The override must be consistent —
    // a half-switch (EGL=Mesa but GBM/GLX still NVIDIA) is what blanks.
    const MESA_EGL: &str = "/usr/share/glvnd/egl_vendor.d/50_mesa.json";

    if std::path::Path::new(MESA_EGL).exists() {
        std::env::set_var("__EGL_VENDOR_LIBRARY_FILENAMES", MESA_EGL);
        std::env::set_var("__GLX_VENDOR_LIBRARY_NAME", "mesa");
        std::env::set_var("LIBVA_DRIVER_NAME", "iHD");
        std::env::remove_var("GBM_BACKEND");
        std::env::remove_var("__NV_PRIME_RENDER_OFFLOAD");
        std::env::remove_var("WEBKIT_DISABLE_DMABUF_RENDERER");
    } else if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

#[derive(serde::Deserialize)]
struct ActionPayload {
    action: String,
}

#[derive(serde::Deserialize)]
struct UnreadPayload {
    count: u32,
}

#[derive(serde::Deserialize)]
struct NotifyPayload {
    title: Option<String>,
    body: Option<String>,
}

#[derive(serde::Deserialize)]
struct UrlPayload {
    url: String,
}

/// Bridges the page-injected scripts to the backend. App commands can't be
/// invoked from the remote WhatsApp origin (only core commands can be granted to
/// it), so the scripts emit events — `event emit` is a core command — and we
/// dispatch them here. Window/menu work is hopped onto the main thread because
/// GTK window creation must run there.
fn register_web_events(app: &tauri::AppHandle) {
    let handle = app.clone();
    app.listen("zw://action", move |event| {
        if let Ok(payload) = serde_json::from_str::<ActionPayload>(event.payload()) {
            let handle = handle.clone();
            let _ = handle
                .clone()
                .run_on_main_thread(move || dispatch_action(&handle, &payload.action));
        }
    });

    let handle = app.clone();
    app.listen("zw://unread", move |event| {
        if let Ok(payload) = serde_json::from_str::<UnreadPayload>(event.payload()) {
            let handle = handle.clone();
            let _ = handle
                .clone()
                .run_on_main_thread(move || tray::set_unread(&handle, payload.count));
        }
    });

    let handle = app.clone();
    app.listen("zw://notify", move |event| {
        if let Ok(payload) = serde_json::from_str::<NotifyPayload>(event.payload()) {
            let handle = handle.clone();
            let _ = handle.clone().run_on_main_thread(move || {
                notification::notify(&handle, payload.title, payload.body)
            });
        }
    });

    let handle = app.clone();
    app.listen("zw://open-external", move |event| {
        if let Ok(payload) = serde_json::from_str::<UrlPayload>(event.payload()) {
            commands::open_external(&handle, &payload.url);
        }
    });
}

/// Routes a titlebar/menu action (or the auto-lock timer) to its handler.
fn dispatch_action(app: &tauri::AppHandle, action: &str) {
    match action {
        "lock" => lock::lock(app),
        "settings" => window::open_settings(app),
        "shortcuts" => window::open_shortcuts(app),
        "about" => window::open_about(app),
        other => log::warn!("unknown menu action: {other}"),
    }
}
