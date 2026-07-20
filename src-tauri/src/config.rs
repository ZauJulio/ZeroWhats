//! Persisted app settings: the on-disk [`Config`], its frontend-facing views
//! ([`ConfigView`]/[`ConfigPatch`]), and the `Theme` enum.

use schemars::JsonSchema;
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
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
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    /// Spell-check the message composer (WebKitGTK/enchant on Linux). When on
    /// with no languages listed, WebKit auto-detects from the system locale.
    pub spellcheck_enabled: bool,
    /// enchant dictionary codes to spell-check against, e.g. ["en_US","pt_BR"].
    pub spellcheck_languages: Vec<String>,
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
            spellcheck_enabled: true,
            spellcheck_languages: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let raw = match std::fs::read_to_string(path) {
            Ok(r) => r,
            Err(_) => return Config::default(),
        };

        let value: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("config.json parse error, using defaults: {e}");
                return Config::default();
            }
        };

        let schema = schemars::schema_for!(Config);
        let schema_value = serde_json::to_value(&schema).expect("schema serializable");
        let validator = jsonschema::validator_for(&schema_value).expect("valid JSON Schema");

        let errors: Vec<_> = validator.iter_errors(&value).collect();
        if !errors.is_empty() {
            for err in &errors {
                log::warn!(
                    "config.json schema violation: {err} at {}",
                    err.instance_path
                );
            }
        }

        let mut cfg: Config = serde_json::from_value(value).unwrap_or_default();

        if let Some(true) = cfg.mute_notifications.take() {
            cfg.notification_privacy = NotificationPrivacy::Hidden;
        }

        cfg
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut value = serde_json::to_value(self).expect("Config is always serializable");
        if let serde_json::Value::Object(ref mut map) = value {
            map.insert(
                "$schema".to_string(),
                serde_json::Value::String("./config.schema.json".to_string()),
            );
        }
        let json = serde_json::to_string_pretty(&value).expect("JSON serializable");
        std::fs::write(path, &json)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        let schema = schemars::schema_for!(Config);
        let schema_json =
            serde_json::to_string_pretty(&schema).expect("schema is always serializable");
        let schema_path = path.with_file_name("config.schema.json");
        std::fs::write(&schema_path, &schema_json)?;

        Ok(())
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
    pub spellcheck_enabled: bool,
    pub spellcheck_languages: Vec<String>,
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
            spellcheck_enabled: c.spellcheck_enabled,
            spellcheck_languages: c.spellcheck_languages,
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
    pub spellcheck_enabled: bool,
    pub spellcheck_languages: Vec<String>,
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
        cfg.spellcheck_enabled = self.spellcheck_enabled;
        cfg.spellcheck_languages = self.spellcheck_languages;
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

    // --- Legacy migration ---

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
    fn legacy_mute_absent_keeps_full_default() {
        let cfg = load_from_json(r#"{}"#);
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Full);
    }

    #[test]
    fn explicit_privacy_wins_over_absent_legacy() {
        let cfg = load_from_json(r#"{ "notification_privacy": "generic" }"#);
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Generic);
    }

    #[test]
    fn legacy_mute_true_overrides_explicit_privacy() {
        let cfg =
            load_from_json(r#"{ "notification_privacy": "full", "mute_notifications": true }"#);
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Hidden);
    }

    // --- NotificationPrivacy ---

    #[test]
    fn privacy_is_hidden() {
        assert!(NotificationPrivacy::Hidden.is_hidden());
        assert!(!NotificationPrivacy::Full.is_hidden());
        assert!(!NotificationPrivacy::Generic.is_hidden());
    }

    #[test]
    fn privacy_shows_preview() {
        assert!(NotificationPrivacy::Full.shows_preview());
        assert!(!NotificationPrivacy::Generic.shows_preview());
        assert!(!NotificationPrivacy::Hidden.shows_preview());
    }

    #[test]
    fn privacy_apply_full_passes_through() {
        let got = NotificationPrivacy::Full.apply(Some("Ana".into()), Some("hi there".into()));
        assert_eq!(got, Some((Some("Ana".into()), Some("hi there".into()))));
    }

    #[test]
    fn privacy_apply_full_with_none_body() {
        let got = NotificationPrivacy::Full.apply(Some("Ana".into()), None);
        assert_eq!(got, Some((Some("Ana".into()), None)));
    }

    #[test]
    fn privacy_apply_full_with_none_title() {
        let got = NotificationPrivacy::Full.apply(None, Some("msg".into()));
        assert_eq!(got, Some((None, Some("msg".into()))));
    }

    #[test]
    fn privacy_apply_generic_keeps_sender_hides_body() {
        let got = NotificationPrivacy::Generic.apply(Some("Ana".into()), Some("hi there".into()));
        assert_eq!(got, Some((Some("Ana".into()), Some("New message".into()))));
    }

    #[test]
    fn privacy_apply_generic_with_none_title() {
        let got = NotificationPrivacy::Generic.apply(None, Some("secret".into()));
        assert_eq!(got, Some((None, Some("New message".into()))));
    }

    #[test]
    fn privacy_apply_hidden_suppresses() {
        let got = NotificationPrivacy::Hidden.apply(Some("Ana".into()), Some("hi there".into()));
        assert_eq!(got, None);
    }

    #[test]
    fn privacy_apply_hidden_suppresses_even_nones() {
        let got = NotificationPrivacy::Hidden.apply(None, None);
        assert_eq!(got, None);
    }

    // --- Theme ---

    #[test]
    fn theme_wa_values() {
        assert_eq!(Theme::System.wa_value(), "system");
        assert_eq!(Theme::Light.wa_value(), "light");
        assert_eq!(Theme::Dark.wa_value(), "dark");
    }

    #[test]
    fn theme_default_is_system() {
        assert_eq!(Theme::default(), Theme::System);
    }

    #[test]
    fn theme_serde_roundtrip() {
        for theme in [Theme::System, Theme::Light, Theme::Dark] {
            let json = serde_json::to_string(&theme).unwrap();
            let back: Theme = serde_json::from_str(&json).unwrap();
            assert_eq!(theme, back);
        }
    }

    #[test]
    fn theme_deserializes_lowercase() {
        let t: Theme = serde_json::from_str(r#""dark""#).unwrap();
        assert_eq!(t, Theme::Dark);
    }

    #[test]
    fn theme_rejects_uppercase() {
        assert!(serde_json::from_str::<Theme>(r#""Dark""#).is_err());
    }

    // --- Config defaults ---

    #[test]
    fn default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.theme, Theme::System);
        assert!(cfg.locale.is_none());
        assert!(!cfg.proxy_enabled);
        assert!(cfg.proxy_url.is_empty());
        assert!(cfg.auto_download);
        assert!(cfg.download_path.is_none());
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Full);
        assert!(!cfg.hide_content_on_unfocus);
        assert!(cfg.cache_enabled);
        assert!(cfg.password_hash.is_none());
        assert!(!cfg.auto_start);
        assert!(cfg.hardware_acceleration);
        assert!(!cfg.lock_on_close);
        assert!(cfg.auto_lock_minutes.is_none());
        assert!(cfg.spellcheck_enabled);
        assert!(cfg.spellcheck_languages.is_empty());
    }

    // --- Load ---

    #[test]
    fn load_missing_file_returns_defaults() {
        let cfg = Config::load(Path::new("/tmp/zw-test-nonexistent-config.json"));
        assert_eq!(cfg.theme, Theme::System);
        assert!(cfg.auto_download);
    }

    #[test]
    fn load_invalid_json_returns_defaults() {
        let cfg = load_from_json("NOT JSON {{{");
        assert_eq!(cfg.theme, Theme::System);
    }

    #[test]
    fn load_empty_object_returns_defaults() {
        let cfg = load_from_json("{}");
        assert_eq!(cfg.theme, Theme::System);
        assert!(cfg.auto_download);
        assert!(cfg.spellcheck_enabled);
    }

    #[test]
    fn load_partial_config_fills_defaults() {
        let cfg = load_from_json(r#"{ "theme": "dark", "auto_start": true }"#);
        assert_eq!(cfg.theme, Theme::Dark);
        assert!(cfg.auto_start);
        assert!(cfg.auto_download);
        assert!(cfg.spellcheck_enabled);
    }

    #[test]
    fn load_all_fields() {
        let cfg = load_from_json(
            r#"{
                "theme": "light",
                "locale": "pt-br",
                "proxy_enabled": true,
                "proxy_url": "socks5://localhost:1080",
                "auto_download": false,
                "download_path": "/tmp/dl",
                "notification_privacy": "hidden",
                "hide_content_on_unfocus": true,
                "cache_enabled": false,
                "password_hash": "$argon2id$...",
                "auto_start": true,
                "hardware_acceleration": false,
                "lock_on_close": true,
                "auto_lock_minutes": 5,
                "spellcheck_enabled": false,
                "spellcheck_languages": ["en_US", "pt_BR"]
            }"#,
        );
        assert_eq!(cfg.theme, Theme::Light);
        assert_eq!(cfg.locale.as_deref(), Some("pt-br"));
        assert!(cfg.proxy_enabled);
        assert_eq!(cfg.proxy_url, "socks5://localhost:1080");
        assert!(!cfg.auto_download);
        assert_eq!(cfg.download_path.as_deref(), Some("/tmp/dl"));
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Hidden);
        assert!(cfg.hide_content_on_unfocus);
        assert!(!cfg.cache_enabled);
        assert!(cfg.password_hash.is_some());
        assert!(cfg.auto_start);
        assert!(!cfg.hardware_acceleration);
        assert!(cfg.lock_on_close);
        assert_eq!(cfg.auto_lock_minutes, Some(5));
        assert!(!cfg.spellcheck_enabled);
        assert_eq!(cfg.spellcheck_languages, vec!["en_US", "pt_BR"]);
    }

    #[test]
    fn load_unknown_fields_ignored() {
        let cfg = load_from_json(r#"{ "theme": "dark", "future_field": 42 }"#);
        assert_eq!(cfg.theme, Theme::Dark);
    }

    #[test]
    fn load_schema_violation_still_loads_with_serde_defaults() {
        let cfg = load_from_json(r#"{ "auto_download": "not a bool" }"#);
        assert!(cfg.auto_download);
    }

    // --- Save ---

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

    #[test]
    fn save_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deep").join("config.json");
        Config::default().save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn save_includes_schema_ref() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        Config::default().save(&path).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains(r#""$schema": "./config.schema.json""#));
    }

    #[test]
    fn save_generates_schema_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        Config::default().save(&path).unwrap();

        let schema_path = dir.path().join("config.schema.json");
        assert!(schema_path.exists());

        let schema_raw = std::fs::read_to_string(&schema_path).unwrap();
        let schema: serde_json::Value = serde_json::from_str(&schema_raw).unwrap();
        assert!(schema.get("properties").is_some());
        assert!(schema["properties"].get("theme").is_some());
        assert!(schema["properties"].get("notification_privacy").is_some());
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let mut original = Config::default();
        original.theme = Theme::Dark;
        original.locale = Some("en".into());
        original.proxy_enabled = true;
        original.proxy_url = "http://proxy:8080".into();
        original.auto_download = false;
        original.notification_privacy = NotificationPrivacy::Generic;
        original.hide_content_on_unfocus = true;
        original.lock_on_close = true;
        original.auto_lock_minutes = Some(10);
        original.spellcheck_languages = vec!["en_US".into()];
        original.password_hash = Some("hash123".into());
        original.save(&path).unwrap();

        let loaded = Config::load(&path);
        assert_eq!(loaded.theme, Theme::Dark);
        assert_eq!(loaded.locale.as_deref(), Some("en"));
        assert!(loaded.proxy_enabled);
        assert_eq!(loaded.proxy_url, "http://proxy:8080");
        assert!(!loaded.auto_download);
        assert_eq!(loaded.notification_privacy, NotificationPrivacy::Generic);
        assert!(loaded.hide_content_on_unfocus);
        assert!(loaded.lock_on_close);
        assert_eq!(loaded.auto_lock_minutes, Some(10));
        assert_eq!(loaded.spellcheck_languages, vec!["en_US"]);
        assert_eq!(loaded.password_hash.as_deref(), Some("hash123"));
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        Config::default().save(&path).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    // --- Schema validation ---

    #[test]
    fn generated_schema_is_valid() {
        let schema = schemars::schema_for!(Config);
        let schema_value = serde_json::to_value(&schema).unwrap();
        assert!(jsonschema::validator_for(&schema_value).is_ok());
    }

    #[test]
    fn schema_validates_correct_config() {
        let schema = schemars::schema_for!(Config);
        let schema_value = serde_json::to_value(&schema).unwrap();
        let validator = jsonschema::validator_for(&schema_value).unwrap();

        let cfg = Config::default();
        let value = serde_json::to_value(&cfg).unwrap();
        assert!(validator.validate(&value).is_ok());
    }

    #[test]
    fn schema_rejects_wrong_theme_value() {
        let schema = schemars::schema_for!(Config);
        let schema_value = serde_json::to_value(&schema).unwrap();
        let validator = jsonschema::validator_for(&schema_value).unwrap();

        let bad: serde_json::Value = serde_json::from_str(r#"{ "theme": "rainbow" }"#).unwrap();
        assert!(validator.iter_errors(&bad).next().is_some());
    }

    #[test]
    fn schema_rejects_wrong_type() {
        let schema = schemars::schema_for!(Config);
        let schema_value = serde_json::to_value(&schema).unwrap();
        let validator = jsonschema::validator_for(&schema_value).unwrap();

        let bad: serde_json::Value = serde_json::from_str(r#"{ "auto_download": "yes" }"#).unwrap();
        assert!(validator.iter_errors(&bad).next().is_some());
    }

    // --- ConfigView ---

    #[test]
    fn config_view_hides_password_hash() {
        let mut cfg = Config::default();
        cfg.password_hash = Some("$argon2id$secret".into());
        let view: ConfigView = cfg.into();
        assert!(view.has_password);

        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains("argon2id"));
        assert!(!json.contains("password_hash"));
        assert!(json.contains("has_password"));
    }

    #[test]
    fn config_view_no_password() {
        let cfg = Config::default();
        let view: ConfigView = cfg.into();
        assert!(!view.has_password);
    }

    #[test]
    fn config_view_preserves_all_fields() {
        let mut cfg = Config::default();
        cfg.theme = Theme::Dark;
        cfg.locale = Some("pt-br".into());
        cfg.proxy_enabled = true;
        cfg.proxy_url = "http://p".into();
        cfg.hide_content_on_unfocus = true;
        cfg.auto_lock_minutes = Some(15);
        cfg.spellcheck_languages = vec!["en_US".into()];

        let view: ConfigView = cfg.into();
        assert_eq!(view.theme, Theme::Dark);
        assert_eq!(view.locale.as_deref(), Some("pt-br"));
        assert!(view.proxy_enabled);
        assert_eq!(view.proxy_url, "http://p");
        assert!(view.hide_content_on_unfocus);
        assert_eq!(view.auto_lock_minutes, Some(15));
        assert_eq!(view.spellcheck_languages, vec!["en_US"]);
    }

    // --- ConfigPatch ---

    #[test]
    fn patch_preserves_password_hash() {
        let mut cfg = Config::default();
        cfg.password_hash = Some("secret_hash".into());

        let patch: ConfigPatch = serde_json::from_str(
            r#"{
                "theme": "dark",
                "locale": null,
                "proxy_enabled": false,
                "proxy_url": "",
                "auto_download": true,
                "download_path": null,
                "notification_privacy": "generic",
                "hide_content_on_unfocus": false,
                "cache_enabled": true,
                "auto_start": false,
                "hardware_acceleration": true,
                "lock_on_close": true,
                "auto_lock_minutes": 5,
                "spellcheck_enabled": true,
                "spellcheck_languages": []
            }"#,
        )
        .unwrap();

        patch.apply_to(&mut cfg);
        assert_eq!(cfg.theme, Theme::Dark);
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Generic);
        assert!(cfg.lock_on_close);
        assert_eq!(cfg.auto_lock_minutes, Some(5));
        assert_eq!(cfg.password_hash.as_deref(), Some("secret_hash"));
    }

    #[test]
    fn patch_applies_all_fields() {
        let mut cfg = Config::default();

        let patch: ConfigPatch = serde_json::from_str(
            r#"{
                "theme": "light",
                "locale": "en",
                "proxy_enabled": true,
                "proxy_url": "http://proxy",
                "auto_download": false,
                "download_path": "/downloads",
                "notification_privacy": "hidden",
                "hide_content_on_unfocus": true,
                "cache_enabled": false,
                "auto_start": true,
                "hardware_acceleration": false,
                "lock_on_close": true,
                "auto_lock_minutes": 30,
                "spellcheck_enabled": false,
                "spellcheck_languages": ["pt_BR"]
            }"#,
        )
        .unwrap();

        patch.apply_to(&mut cfg);
        assert_eq!(cfg.theme, Theme::Light);
        assert_eq!(cfg.locale.as_deref(), Some("en"));
        assert!(cfg.proxy_enabled);
        assert_eq!(cfg.proxy_url, "http://proxy");
        assert!(!cfg.auto_download);
        assert_eq!(cfg.download_path.as_deref(), Some("/downloads"));
        assert_eq!(cfg.notification_privacy, NotificationPrivacy::Hidden);
        assert!(cfg.hide_content_on_unfocus);
        assert!(!cfg.cache_enabled);
        assert!(cfg.auto_start);
        assert!(!cfg.hardware_acceleration);
        assert!(cfg.lock_on_close);
        assert_eq!(cfg.auto_lock_minutes, Some(30));
        assert!(!cfg.spellcheck_enabled);
        assert_eq!(cfg.spellcheck_languages, vec!["pt_BR"]);
    }
}
