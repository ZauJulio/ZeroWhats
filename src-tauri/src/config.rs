//! Persisted app settings: the on-disk [`Config`], its frontend-facing views
//! ([`ConfigView`]/[`ConfigPatch`]), and the `Theme` enum.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

/// Absolute path of the persisted config file (`config.json` in the app config
/// dir). See the README for the per-OS location.
pub fn config_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .expect("app config dir resolvable")
        .join("config.json")
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    /// The value WhatsApp Web persists in localStorage["theme"].
    pub fn wa_value(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}

/// How much of a notification's content is shown natively.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPrivacy {
    /// Show the real sender and message (the historical behaviour).
    #[default]
    Full,
    /// Show a generic banner ("WhatsApp" / "New message"), hiding the preview.
    Generic,
    /// Suppress the notification entirely (equivalent to the old "mute").
    Hidden,
}

impl NotificationPrivacy {
    pub fn is_hidden(self) -> bool {
        matches!(self, NotificationPrivacy::Hidden)
    }

    /// Whether the message preview (and, with it, the sender's avatar) may be
    /// shown. Only `Full` reveals who wrote and what they said.
    pub fn shows_preview(self) -> bool {
        matches!(self, NotificationPrivacy::Full)
    }

    /// Applies the privacy level to a notification's `(title, body)`. `None`
    /// means the notification should be suppressed entirely.
    pub fn apply(
        self,
        title: Option<String>,
        body: Option<String>,
    ) -> Option<(Option<String>, Option<String>)> {
        match self {
            // Suppress entirely.
            NotificationPrivacy::Hidden => None,
            // Keep the real sender (`title`); replace the message with a
            // neutral preview.
            NotificationPrivacy::Generic => Some((title, Some("New message".to_string()))),
            NotificationPrivacy::Full => Some((title, body)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
    /// "en" / "pt-br", or None to follow the system locale.
    pub locale: Option<String>,
    pub proxy_enabled: bool,
    pub proxy_url: String,
    pub auto_download: bool,
    pub download_path: Option<String>,
    /// How much of a notification's content is shown natively.
    pub notification_privacy: NotificationPrivacy,
    /// Blur the WhatsApp page whenever the window loses focus (protects
    /// screenshots / task-switcher thumbnails / screen-sharing).
    pub hide_content_on_unfocus: bool,
    /// Legacy toggle (pre-`notification_privacy`). Only read to migrate old
    /// configs on load, then dropped. Never written back.
    #[serde(default, skip_serializing)]
    mute_notifications: Option<bool>,
    pub cache_enabled: bool,
    pub password_hash: Option<String>,
    pub auto_start: bool,
    pub hardware_acceleration: bool,
    /// When a password is set, lock whenever the window is closed.
    pub lock_on_close: bool,
    /// Auto-lock the app after this many minutes of inactivity *within the
    /// window* (cross-platform, no system idle API). `None`/0 disables it; only
    /// effective when a password is set.
    pub auto_lock_minutes: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: Theme::System,
            locale: None,
            proxy_enabled: false,
            proxy_url: String::new(),
            auto_download: true,
            download_path: None,
            notification_privacy: NotificationPrivacy::Full,
            hide_content_on_unfocus: false,
            mute_notifications: None,
            cache_enabled: true,
            password_hash: None,
            auto_start: false,
            hardware_acceleration: true,
            lock_on_close: false,
            auto_lock_minutes: None,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let mut cfg: Config = std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();

        // Migrate the pre-`notification_privacy` toggle: a muted old config maps
        // to `Hidden`. Cleared so it's never carried forward.
        if let Some(true) = cfg.mute_notifications.take() {
            cfg.notification_privacy = NotificationPrivacy::Hidden;
        }

        cfg
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self).expect("Config is always serializable");
        std::fs::write(path, json)
    }
}

/// The config as exposed to the frontend (Settings/Lock screens). Hides the
/// password hash, surfacing only whether a password is set.
#[derive(serde::Serialize)]
pub struct ConfigView {
    pub theme: Theme,
    pub locale: Option<String>,
    pub proxy_enabled: bool,
    pub proxy_url: String,
    pub auto_download: bool,
    pub download_path: Option<String>,
    pub notification_privacy: NotificationPrivacy,
    pub hide_content_on_unfocus: bool,
    pub cache_enabled: bool,
    pub auto_start: bool,
    pub hardware_acceleration: bool,
    pub lock_on_close: bool,
    pub auto_lock_minutes: Option<u32>,
    pub has_password: bool,
}

impl From<Config> for ConfigView {
    fn from(c: Config) -> Self {
        ConfigView {
            theme: c.theme,
            locale: c.locale,
            proxy_enabled: c.proxy_enabled,
            proxy_url: c.proxy_url,
            auto_download: c.auto_download,
            download_path: c.download_path,
            notification_privacy: c.notification_privacy,
            hide_content_on_unfocus: c.hide_content_on_unfocus,
            cache_enabled: c.cache_enabled,
            auto_start: c.auto_start,
            hardware_acceleration: c.hardware_acceleration,
            lock_on_close: c.lock_on_close,
            auto_lock_minutes: c.auto_lock_minutes,
            has_password: c.password_hash.is_some(),
        }
    }
}

/// The settings the frontend can change (everything in [`ConfigView`] except the
/// derived `has_password`). The password is changed via its own command.
#[derive(serde::Deserialize)]
pub struct ConfigPatch {
    pub theme: Theme,
    pub locale: Option<String>,
    pub proxy_enabled: bool,
    pub proxy_url: String,
    pub auto_download: bool,
    pub download_path: Option<String>,
    pub notification_privacy: NotificationPrivacy,
    pub hide_content_on_unfocus: bool,
    pub cache_enabled: bool,
    pub auto_start: bool,
    pub hardware_acceleration: bool,
    pub lock_on_close: bool,
    pub auto_lock_minutes: Option<u32>,
}

impl ConfigPatch {
    /// Applies the patch onto a loaded config, preserving fields the frontend
    /// doesn't own (e.g. the password hash).
    pub fn apply_to(self, cfg: &mut Config) {
        cfg.theme = self.theme;
        cfg.locale = self.locale;
        cfg.proxy_enabled = self.proxy_enabled;
        cfg.proxy_url = self.proxy_url;
        cfg.auto_download = self.auto_download;
        cfg.download_path = self.download_path;
        cfg.notification_privacy = self.notification_privacy;
        cfg.hide_content_on_unfocus = self.hide_content_on_unfocus;
        cfg.cache_enabled = self.cache_enabled;
        cfg.auto_start = self.auto_start;
        cfg.hardware_acceleration = self.hardware_acceleration;
        cfg.lock_on_close = self.lock_on_close;
        cfg.auto_lock_minutes = self.auto_lock_minutes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn load_from_json(json: &str) -> Config {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(json.as_bytes()).unwrap();
        Config::load(f.path())
    }

    #[test]
    fn migrates_legacy_mute_to_hidden() {
        let cfg = load_from_json(r#"{ "mute_notifications": true }"#);
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Hidden);
    }

    #[test]
    fn legacy_unmuted_keeps_full_default() {
        let cfg = load_from_json(r#"{ "mute_notifications": false }"#);
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Full);
    }

    #[test]
    fn reads_new_privacy_field() {
        let cfg = load_from_json(r#"{ "notification_privacy": "generic" }"#);
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Generic);
    }

    #[test]
    fn privacy_apply_full_passes_through() {
        let got = NotificationPrivacy::Full.apply(Some("Ana".into()), Some("hi there".into()));
        assert_eq!(got, Some((Some("Ana".into()), Some("hi there".into()))));
    }

    #[test]
    fn privacy_apply_generic_keeps_sender_hides_body() {
        let got = NotificationPrivacy::Generic.apply(Some("Ana".into()), Some("hi there".into()));
        assert_eq!(got, Some((Some("Ana".into()), Some("New message".into()))));
    }

    #[test]
    fn privacy_apply_hidden_suppresses() {
        let got = NotificationPrivacy::Hidden.apply(Some("Ana".into()), Some("hi there".into()));
        assert_eq!(got, None);
    }

    #[test]
    fn saved_config_drops_legacy_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = Config::default();
        cfg.notification_privacy = NotificationPrivacy::Generic;
        cfg.save(&path).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("mute_notifications"));
        assert!(raw.contains("notification_privacy"));
        assert!(raw.contains("hide_content_on_unfocus"));
    }
}
