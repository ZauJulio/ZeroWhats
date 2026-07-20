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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_semver_plain() {
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn parse_semver_with_v_prefix() {
        assert_eq!(parse_semver("v1.5.2"), Some((1, 5, 2)));
    }

    #[test]
    fn parse_semver_zero() {
        assert_eq!(parse_semver("0.0.0"), Some((0, 0, 0)));
    }

    #[test]
    fn parse_semver_large_numbers() {
        assert_eq!(parse_semver("v100.200.300"), Some((100, 200, 300)));
    }

    #[test]
    fn parse_semver_invalid_missing_patch() {
        assert_eq!(parse_semver("1.2"), None);
    }

    #[test]
    fn parse_semver_invalid_empty() {
        assert_eq!(parse_semver(""), None);
    }

    #[test]
    fn parse_semver_invalid_text() {
        assert_eq!(parse_semver("abc"), None);
    }

    #[test]
    fn parse_semver_invalid_non_numeric() {
        assert_eq!(parse_semver("v1.x.3"), None);
    }

    #[test]
    fn semver_comparison_newer() {
        let remote = parse_semver("v2.0.0").unwrap();
        let local = parse_semver("v1.5.2").unwrap();
        assert!(remote > local);
    }

    #[test]
    fn semver_comparison_same() {
        let remote = parse_semver("v1.5.2").unwrap();
        let local = parse_semver("v1.5.2").unwrap();
        assert!(!(remote > local));
    }

    #[test]
    fn semver_comparison_older() {
        let remote = parse_semver("v1.4.0").unwrap();
        let local = parse_semver("v1.5.2").unwrap();
        assert!(!(remote > local));
    }

    #[test]
    fn semver_comparison_patch_bump() {
        let remote = parse_semver("v1.5.3").unwrap();
        let local = parse_semver("v1.5.2").unwrap();
        assert!(remote > local);
    }

    #[test]
    fn semver_comparison_minor_bump() {
        let remote = parse_semver("v1.6.0").unwrap();
        let local = parse_semver("v1.5.9").unwrap();
        assert!(remote > local);
    }
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
