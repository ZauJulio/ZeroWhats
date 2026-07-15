use serde::Serialize;
use tauri::{Emitter, Manager};

const RELEASES_URL: &str = "https://api.github.com/repos/ZauJulio/ZeroWhats/releases/latest";

#[derive(Clone, Serialize, serde::Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub html_url: String,
    pub published_at: String,
}

fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

pub fn check_update(current: &str) -> Option<ReleaseInfo> {
    let mut response: ureq::Body = ureq::get(RELEASES_URL)
        .header("User-Agent", "ZeroWhats-Updater")
        .header("Accept", "application/vnd.github+json")
        .call()
        .ok()?
        .into_body();

    let info: ReleaseInfo = serde_json::from_reader(response.as_reader()).ok()?;
    let remote = parse_semver(&info.tag_name)?;
    let local = parse_semver(current)?;

    if remote > local {
        Some(info)
    } else {
        None
    }
}

#[tauri::command]
pub fn check_for_update(app: tauri::AppHandle) -> Option<ReleaseInfo> {
    let version = app.config().version.clone().unwrap_or_default();
    check_update(&version)
}

pub fn start_background_check(app: &tauri::AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        loop {
            let version = handle.config().version.clone().unwrap_or_default();
            if let Some(info) = check_update(&version) {
                log::info!("update available: {}", info.tag_name);
                if let Some(main) = handle.get_webview_window(crate::window::MAIN_LABEL) {
                    let _ = main.emit_to(crate::window::MAIN_LABEL, "zw://update-available", &info);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(7 * 24 * 3600));
        }
    });
}
