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
    pub mute_notifications: bool,
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
            mute_notifications: false,
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
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
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
    pub mute_notifications: bool,
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
            mute_notifications: c.mute_notifications,
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
    pub mute_notifications: bool,
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
        cfg.mute_notifications = self.mute_notifications;
        cfg.cache_enabled = self.cache_enabled;
        cfg.auto_start = self.auto_start;
        cfg.hardware_acceleration = self.hardware_acceleration;
        cfg.lock_on_close = self.lock_on_close;
        cfg.auto_lock_minutes = self.auto_lock_minutes;
    }
}
