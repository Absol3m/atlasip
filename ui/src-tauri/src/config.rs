use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Config schema ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetworkConfig {
    pub dns_resolver: String,
    pub ip_mode: String,
    pub pooling_enabled: bool,
    pub max_connections: u32,
    pub keep_alive: bool,
    pub retry_strategy: String,
    pub retry_delay_ms: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            dns_resolver: "system".into(),
            ip_mode: "dual".into(),
            pooling_enabled: true,
            max_connections: 10,
            keep_alive: true,
            retry_strategy: "none".into(),
            retry_delay_ms: 1_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProxyConfig {
    pub enabled: bool,
    pub proxy_type: String,
    pub url: String,
    pub no_proxy: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            proxy_type: "HTTP".into(),
            url: String::new(),
            no_proxy: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TimeoutsConfig {
    pub global_ms: u64,
    pub request_ms: u64,
    pub dns_ms: u64,
    pub geoip_ms: u64,
}

impl Default for TimeoutsConfig {
    fn default() -> Self {
        Self {
            global_ms: 5_000,
            request_ms: 10_000,
            dns_ms: 2_000,
            geoip_ms: 3_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RetryConfig {
    pub enabled: bool,
    pub count: u32,
    pub delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            count: 3,
            delay_ms: 1_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub timeouts: TimeoutsConfig,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub autostart: bool,
}

fn default_locale() -> String { "en-US".into() }
fn default_theme()  -> String { "system".into() }

// ── File path ─────────────────────────────────────────────────────────────────

fn config_path() -> PathBuf {
    // <data-dir>/atlasip/config.toml  (falls back to current dir)
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("atlasip")
        .join("config.toml")
}

// ── Internal I/O (path-taking; used by tests) ─────────────────────────────────

/// Load a config from an arbitrary path.
/// - File missing → silent default.
/// - Parse error  → logged default.
pub(crate) fn load_config_from(path: &Path) -> AppConfig {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return AppConfig::default(),
        Err(e) => {
            eprintln!("[atlasip] could not read {}: {e}", path.display());
            return AppConfig::default();
        }
    };

    match toml::from_str::<AppConfig>(&raw) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("[atlasip] config parse error at {}: {e}", path.display());
            AppConfig::default()
        }
    }
}

/// Write a config to an arbitrary path (atomic rename).
pub(crate) fn save_config_to(config: &AppConfig, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }

    let raw = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, &raw)
        .map_err(|e| format!("Failed to write temporary config: {e}"))?;

    std::fs::rename(&tmp, path)
        .map_err(|e| format!("Failed to finalise config.toml: {e}"))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load `config.toml` from the platform data directory.
pub fn load_config() -> AppConfig {
    load_config_from(&config_path())
}

/// Persist `config` to `config.toml` in the platform data directory.
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    save_config_to(config, &config_path())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Return a unique temp path for each test (no external crate needed).
    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "atlasip_cfg_{}_{}.toml",
            tag,
            std::process::id(),
        ))
    }

    fn sample() -> AppConfig {
        AppConfig {
            network: NetworkConfig {
                dns_resolver:    "cloudflare".into(),
                ip_mode:         "ipv4".into(),
                pooling_enabled: false,
                max_connections: 25,
                keep_alive:      false,
                retry_strategy:  "linear".into(),
                retry_delay_ms:  2_000,
            },
            proxy: ProxyConfig {
                enabled:    true,
                proxy_type: "SOCKS5".into(),
                url:        "socks5://proxy.example.com:1080".into(),
                no_proxy:   "localhost,127.0.0.1".into(),
            },
            timeouts: TimeoutsConfig {
                global_ms:  8_000,
                request_ms: 4_000,
                dns_ms:     1_500,
                geoip_ms:   2_500,
            },
            retry: RetryConfig {
                enabled:  true,
                count:    5,
                delay_ms: 500,
            },
            locale: "fr".into(),
            theme:  "dark".into(),
        }
    }

    // ── get_config equivalent ─────────────────────────────────────────────────

    #[test]
    fn test_load_missing_file_returns_default() {
        let path = PathBuf::from("/nonexistent/atlasip/config.toml");
        let cfg = load_config_from(&path);
        let def = AppConfig::default();

        assert_eq!(cfg.network.dns_resolver, def.network.dns_resolver);
        assert_eq!(cfg.network.ip_mode,      def.network.ip_mode);
        assert_eq!(cfg.locale,               def.locale);
        assert_eq!(cfg.theme,                def.theme);
    }

    #[test]
    fn test_load_invalid_toml_returns_default() {
        let path = temp_path("invalid");
        std::fs::write(&path, b"not valid toml {{{{ [[[").unwrap();

        let cfg = load_config_from(&path);
        assert_eq!(cfg.network.dns_resolver, AppConfig::default().network.dns_resolver);

        std::fs::remove_file(&path).ok();
    }

    // ── set_config equivalent ─────────────────────────────────────────────────

    #[test]
    fn test_save_creates_toml_file() {
        let path = temp_path("save");
        let cfg  = sample();

        save_config_to(&cfg, &path).expect("save should succeed");

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("dns_resolver = \"cloudflare\""), "TOML missing dns_resolver");
        assert!(contents.contains("proxy_type = \"SOCKS5\""),       "TOML missing proxy_type");
        assert!(contents.contains("locale = \"fr\""),               "TOML missing locale");
        assert!(contents.contains("theme = \"dark\""),              "TOML missing theme");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_creates_parent_dirs() {
        let path = temp_path("dirs");
        let nested = PathBuf::from(path.to_str().unwrap().replace(".toml", "/nested/config.toml"));
        save_config_to(&AppConfig::default(), &nested).expect("should create parent dirs");
        assert!(nested.exists());
        std::fs::remove_dir_all(nested.parent().unwrap().parent().unwrap()).ok();
    }

    // ── Round-trip ────────────────────────────────────────────────────────────

    #[test]
    fn test_round_trip_preserves_all_fields() {
        let path     = temp_path("roundtrip");
        let original = sample();

        save_config_to(&original, &path).unwrap();
        let restored = load_config_from(&path);

        // Network
        assert_eq!(restored.network.dns_resolver,    "cloudflare");
        assert_eq!(restored.network.ip_mode,         "ipv4");
        assert!(!restored.network.pooling_enabled);
        assert_eq!(restored.network.max_connections, 25);
        assert!(!restored.network.keep_alive);
        assert_eq!(restored.network.retry_strategy,  "linear");
        assert_eq!(restored.network.retry_delay_ms,  2_000);

        // Proxy
        assert!(restored.proxy.enabled);
        assert_eq!(restored.proxy.proxy_type, "SOCKS5");
        assert_eq!(restored.proxy.url,        "socks5://proxy.example.com:1080");
        assert_eq!(restored.proxy.no_proxy,   "localhost,127.0.0.1");

        // Timeouts
        assert_eq!(restored.timeouts.global_ms,  8_000);
        assert_eq!(restored.timeouts.request_ms, 4_000);
        assert_eq!(restored.timeouts.dns_ms,     1_500);
        assert_eq!(restored.timeouts.geoip_ms,   2_500);

        // Retry
        assert!(restored.retry.enabled);
        assert_eq!(restored.retry.count,    5);
        assert_eq!(restored.retry.delay_ms, 500);

        // Top-level
        assert_eq!(restored.locale, "fr");
        assert_eq!(restored.theme,  "dark");

        std::fs::remove_file(&path).ok();
    }
}
