//! System-tray icon: its menu, click handling, and the unread-count badge.

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::config::{config_path, Config};
use crate::{lock, notification, window};

/// Stable id used to look the tray up again (e.g. from `set_unread`).
const TRAY_ID: &str = "tray";

/// Embedded fallback for the tray icon. `default_window_icon()` is always
/// `Some` in practice (the bundle always configures one), but a missing icon
/// used to be a hard `.expect()` panic — this keeps the tray (and the rest of
/// the app) alive instead.
const FALLBACK_ICON: &[u8] = include_bytes!("../icons/32x32.png");

fn app_icon(app: &AppHandle) -> Image<'_> {
    if let Some(icon) = app.default_window_icon() {
        return icon.clone();
    }
    log::warn!("bundled window icon missing; using embedded fallback");
    Image::from_bytes(FALLBACK_ICON).expect("embedded fallback icon is valid PNG bytes")
}

/// Builds the tray menu from the current config: "Mute" reflects the saved
/// state, and "Lock" only appears once a password is configured.
fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let cfg = Config::load(&config_path(app));
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let prefs = MenuItem::with_id(app, "preferences", "Preferences", true, None::<&str>)?;

    let mute = CheckMenuItem::with_id(
        app,
        "mute",
        "Mute notifications",
        true,
        cfg.mute_notifications,
        None::<&str>,
    )?;

    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::new(app)?;
    menu.append(&show)?;
    menu.append(&prefs)?;
    menu.append(&mute)?;

    if cfg.password_hash.is_some() {
        let lock = MenuItem::with_id(app, "lock", "Lock", true, None::<&str>)?;
        menu.append(&lock)?;
    }

    menu.append(&quit)?;

    Ok(menu)
}

/// Rebuilds the tray menu — call after the mute or password state changes so the
/// check mark and the conditional "Lock" item stay in sync.
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        log::warn!("tray::refresh: tray icon not found");
        return;
    };

    match build_menu(app) {
        Ok(menu) => {
            if let Err(e) = tray.set_menu(Some(menu)) {
                log::error!("tray::refresh: set_menu failed: {e}");
            }
        }
        Err(e) => log::error!("tray::refresh: build_menu failed: {e}"),
    }
}

/// Builds the tray icon and wires its menu and click behaviour.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&build_menu(app)?)
        .tooltip("ZeroWhats")
        .icon(app_icon(app))
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => window::show_main(app),
            "preferences" => window::open_settings(app),
            "mute" => {
                notification::toggle_muted(app);
                refresh(app); // reflect the new check state
            }
            "lock" => lock::lock(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();

                if let Some(main) = app.get_webview_window(window::MAIN_LABEL) {
                    if main.is_visible().unwrap_or(false) && !lock::is_locked() {
                        let _ = main.hide();
                    } else {
                        window::show_main(app);
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// Reflects WhatsApp's unread count on the tray (fed by `web/unread-badge.js`
/// via the `zw://unread` event). Draws a numeric badge onto the icon — visible
/// on every platform, including GNOME where the SNI title isn't rendered — plus
/// a tooltip/title (macOS menubar / Windows hover). 0 restores the plain icon.
pub fn set_unread(app: &AppHandle, count: u32) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let base = app.default_window_icon().cloned();

    if count > 0 {
        if let Some(base) = &base {
            let _ = tray.set_icon(Some(render_badge(base, count)));
        }

        let _ = tray.set_tooltip(Some(format!("ZeroWhats — {count} unread")));
    } else {
        let _ = tray.set_icon(base);
        let _ = tray.set_tooltip(Some("ZeroWhats"));
    }
}

/// A 3x5 pixel font for the digits and `+`. Each row holds the low 3 bits, MSB
/// = leftmost pixel.
#[rustfmt::skip]
const FONT_3X5: [[u8; 5]; 11] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b100, 0b100], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
    [0b000, 0b010, 0b111, 0b010, 0b000], // + (index 10)
];

fn glyph_index(c: char) -> Option<usize> {
    match c {
        '0'..='9' => Some(c as usize - '0' as usize),
        '+' => Some(10),
        _ => None,
    }
}

/// Draws a red unread badge with white digits onto the bottom-right of the base
/// tray icon and returns a new image. Painted by hand on the RGBA buffer so it
/// needs no extra image/font crate. This is what makes the counter visible on
/// Linux — GNOME's AppIndicator renders the icon, not the SNI title.
fn render_badge(base: &Image, count: u32) -> Image<'static> {
    let w = base.width() as i32;
    let h = base.height() as i32;
    let mut px = base.rgba().to_vec();

    let put = |px: &mut [u8], x: i32, y: i32, (r, g, b): (u8, u8, u8)| {
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        let i = ((y * w + x) * 4) as usize;
        px[i] = r;
        px[i + 1] = g;
        px[i + 2] = b;
        px[i + 3] = 255;
    };

    // >99 doesn't fit legibly on a small icon; collapse to "9+".
    let label = if count > 99 {
        "9+".to_string()
    } else {
        count.to_string()
    };
    let glyphs = label.chars().count() as i32;

    let scale = ((h as f32 * 0.42) / 5.0).round().max(1.0) as i32; // font pixel size
    let gap = scale;
    let text_w = glyphs * 3 * scale + (glyphs - 1) * gap;
    let text_h = 5 * scale;

    let pad = scale.max(1);
    let badge_w = text_w + pad * 2;
    let badge_h = text_h + pad * 2;
    let badge_x = w - badge_w;
    let badge_y = h - badge_h;
    let radius = pad + scale / 2;

    // Rounded-rectangle black background.
    for yy in 0..badge_h {
        for xx in 0..badge_w {
            let cx = if xx < radius {
                radius - xx
            } else if xx >= badge_w - radius {
                xx - (badge_w - radius - 1)
            } else {
                0
            };
            let cy = if yy < radius {
                radius - yy
            } else if yy >= badge_h - radius {
                yy - (badge_h - radius - 1)
            } else {
                0
            };
            if cx * cx + cy * cy <= radius * radius {
                put(&mut px, badge_x + xx, badge_y + yy, (0x00, 0x00, 0x00)); // black outline
            }
        }
    }

    // White digits, centered in the badge.
    let text_x = badge_x + (badge_w - text_w) / 2;
    let text_y = badge_y + (badge_h - text_h) / 2;
    let mut glyph_x = text_x;
    for ch in label.chars() {
        if let Some(gi) = glyph_index(ch) {
            for (row, bits) in FONT_3X5[gi].iter().enumerate() {
                for col in 0..3i32 {
                    if bits & (1 << (2 - col)) != 0 {
                        for dy in 0..scale {
                            for dx in 0..scale {
                                put(
                                    &mut px,
                                    glyph_x + col * scale + dx,
                                    text_y + row as i32 * scale + dy,
                                    (0xFF, 0xFF, 0xFF),
                                );
                            }
                        }
                    }
                }
            }
        }
        glyph_x += 3 * scale + gap;
    }

    Image::new_owned(px, w as u32, h as u32)
}
