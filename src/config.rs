use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs,
    path::{Path, PathBuf},
};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    ValidationError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(s) => write!(f, "I/O error: {s}"),
            ConfigError::ParseError(s) => write!(f, "Parse error: {s}"),
            ConfigError::ValidationError(s) => write!(f, "Validation error: {s}"),
        }
    }
}

impl std::error::Error for ConfigError {}

// Allow `?` to propagate ConfigError into anyhow::Error via the blanket impl.
// (anyhow covers all `Error + Send + Sync + 'static` automatically.)

// ---------------------------------------------------------------------------
// ProxyConfig
// ---------------------------------------------------------------------------

/// Per-protocol proxy URLs (spec §7 — Configuration).
///
/// Each field, when `Some`, must start with the matching scheme:
/// `http://`, `https://`, `socks4://`, `socks5://`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyConfig {
    pub http: Option<String>,
    pub https: Option<String>,
    pub socks4: Option<String>,
    pub socks5: Option<String>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            http: None,
            https: None,
            socks4: None,
            socks5: None,
        }
    }
}

// ---------------------------------------------------------------------------
// AppConfig
// ---------------------------------------------------------------------------

/// Runtime configuration for AtlasIP (spec §7 — Configuration).
///
/// Persisted as TOML at [`AppConfig::default_path`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // ── Network ────────────────────────────────────────────────────────────
    /// Address and port the HTTP API listens on.
    pub listen_addr: String,

    /// Default network timeout in milliseconds (used when a sub-timeout is
    /// not overridden).
    pub default_timeout_ms: u64,

    // ── Per-pipeline timeouts (kept for http.rs / cli.rs compatibility) ───
    pub dns_timeout_ms: u64,
    pub whois_timeout_ms: u64,
    pub rdap_timeout_ms: u64,

    // ── Lookup behaviour ───────────────────────────────────────────────────
    /// Silently skip private / reserved IP addresses.
    pub ignore_private_ips: bool,

    /// Maximum number of concurrent lookup tasks.
    pub max_concurrent_lookups: u32,

    /// Automatically retry failed lookups once.
    pub auto_retry_failed: bool,

    /// After this many lookups, pause for `pause_duration_ms` ms.
    /// `0` means never pause.
    pub pause_every: u32,

    /// Duration of the rate-limit pause in milliseconds.
    pub pause_duration_ms: u64,

    /// Override the WHOIS server used for all queries (empty = auto).
    pub whois_server_override: String,

    // ── Export / UI (kept for http.rs / models.rs compatibility) ──────────
    pub language: String,
    pub default_export_format: String,
    pub csv_with_header: bool,

    // ── Legacy flat proxy fields (kept for ConfigUpdateRequest compat) ─────
    pub proxy_type: String,
    pub proxy_host: String,
    pub proxy_port: u16,

    // ── Structured proxy block (spec §7) ───────────────────────────────────
    pub proxy: ProxyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".to_string(),
            default_timeout_ms: 5000,
            dns_timeout_ms: 3000,
            whois_timeout_ms: 5000,
            rdap_timeout_ms: 5000,
            ignore_private_ips: true,
            max_concurrent_lookups: 32,
            auto_retry_failed: false,
            pause_every: 0,
            pause_duration_ms: 0,
            whois_server_override: String::new(),
            language: "fr".to_string(),
            default_export_format: "csv".to_string(),
            csv_with_header: true,
            proxy_type: "none".to_string(),
            proxy_host: String::new(),
            proxy_port: 0,
            proxy: ProxyConfig::default(),
        }
    }
}

impl AppConfig {
    // ── Construction ────────────────────────────────────────────────────────

    /// Return the canonical path for the config file.
    ///
    /// Resolution order:
    /// 1. `$XDG_CONFIG_HOME/atlasip/config.toml`
    /// 2. `$HOME/.config/atlasip/config.toml`
    /// 3. `.config/atlasip/config.toml` (relative fallback)
    pub fn default_path() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
                    .unwrap_or_else(|_| PathBuf::from(".config"))
            });
        base.join("atlasip").join("config.toml")
    }

    // ── Persistence ─────────────────────────────────────────────────────────

    /// Load config from `path`.
    ///
    /// If the file does not exist, returns [`AppConfig::default`].
    /// Parse errors are returned as [`ConfigError::ParseError`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        let contents = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => {
                return Err(ConfigError::IoError(e.to_string()));
            }
        };

        toml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Serialize the config as TOML and write it to `path`.
    ///
    /// Parent directories are created automatically.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;
        }

        let contents =
            toml::to_string_pretty(self).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        fs::write(path, contents).map_err(|e| ConfigError::IoError(e.to_string()))
    }

    // ── Validation ──────────────────────────────────────────────────────────

    /// Validate the configuration, returning the first violation found.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.listen_addr.contains(':') {
            return Err(ConfigError::ValidationError(
                "listen_addr must be in the form \"host:port\"".to_string(),
            ));
        }

        if self.default_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "default_timeout_ms must be > 0".to_string(),
            ));
        }

        if self.max_concurrent_lookups == 0 {
            return Err(ConfigError::ValidationError(
                "max_concurrent_lookups must be > 0".to_string(),
            ));
        }

        // Proxy URL scheme validation.
        let proxy_checks: &[(&Option<String>, &str)] = &[
            (&self.proxy.http, "http://"),
            (&self.proxy.https, "https://"),
            (&self.proxy.socks4, "socks4://"),
            (&self.proxy.socks5, "socks5://"),
        ];

        for (field, expected_scheme) in proxy_checks {
            if let Some(url) = field {
                if !url.starts_with(expected_scheme) {
                    return Err(ConfigError::ValidationError(format!(
                        "proxy URL \"{url}\" must start with \"{expected_scheme}\""
                    )));
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("atlasip_test_{name}.toml"))
    }

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.listen_addr, "127.0.0.1:8080");
        assert_eq!(cfg.default_timeout_ms, 5000);
        assert!(cfg.ignore_private_ips);
        assert_eq!(cfg.max_concurrent_lookups, 32);
        assert!(!cfg.auto_retry_failed);
        assert_eq!(cfg.pause_every, 0);
        assert_eq!(cfg.pause_duration_ms, 0);
        assert_eq!(cfg.whois_server_override, "");
        assert!(cfg.proxy.http.is_none());
        assert!(cfg.proxy.https.is_none());
        assert!(cfg.proxy.socks4.is_none());
        assert!(cfg.proxy.socks5.is_none());
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let path = temp_path("missing_file_that_does_not_exist_xyz");
        // Make sure it really does not exist.
        let _ = fs::remove_file(&path);

        let cfg = AppConfig::load(&path).expect("should return default");
        assert_eq!(cfg.listen_addr, AppConfig::default().listen_addr);
    }

    #[test]
    fn test_load_valid_config() {
        let path = temp_path("load_valid");
        let toml = r#"
            listen_addr = "0.0.0.0:9090"
            default_timeout_ms = 8000
            dns_timeout_ms = 2000
            whois_timeout_ms = 6000
            rdap_timeout_ms = 6000
            ignore_private_ips = false
            max_concurrent_lookups = 16
            auto_retry_failed = true
            pause_every = 10
            pause_duration_ms = 500
            whois_server_override = "whois.example.com"
            language = "en"
            default_export_format = "tsv"
            csv_with_header = false
            proxy_type = "none"
            proxy_host = ""
            proxy_port = 0

            [proxy]
            http = "http://proxy.example.com:3128"
            socks5 = "socks5://proxy.example.com:1080"
        "#;
        fs::write(&path, toml).unwrap();

        let cfg = AppConfig::load(&path).expect("should parse");
        assert_eq!(cfg.listen_addr, "0.0.0.0:9090");
        assert_eq!(cfg.default_timeout_ms, 8000);
        assert!(!cfg.ignore_private_ips);
        assert_eq!(cfg.max_concurrent_lookups, 16);
        assert!(cfg.auto_retry_failed);
        assert_eq!(cfg.pause_every, 10);
        assert_eq!(cfg.pause_duration_ms, 500);
        assert_eq!(cfg.whois_server_override, "whois.example.com");
        assert_eq!(cfg.proxy.http.as_deref(), Some("http://proxy.example.com:3128"));
        assert_eq!(cfg.proxy.socks5.as_deref(), Some("socks5://proxy.example.com:1080"));
        assert!(cfg.proxy.https.is_none());
        assert!(cfg.proxy.socks4.is_none());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_invalid_config() {
        let path = temp_path("load_invalid");
        // Invalid TOML: duplicate key.
        fs::write(&path, "listen_addr = \"bad\"\nlisten_addr = \"double\"").unwrap();

        let result = AppConfig::load(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ParseError(_) => {}
            other => panic!("expected ParseError, got {other:?}"),
        }

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_save_and_reload() {
        let path = temp_path("save_reload");
        let mut cfg = AppConfig::default();
        cfg.listen_addr = "0.0.0.0:7777".to_string();
        cfg.max_concurrent_lookups = 64;
        cfg.whois_server_override = "whois.ripe.net".to_string();
        cfg.proxy.socks5 = Some("socks5://127.0.0.1:9050".to_string());

        cfg.save(&path).expect("save should succeed");
        assert!(path.exists());

        let reloaded = AppConfig::load(&path).expect("reload should succeed");
        assert_eq!(reloaded.listen_addr, "0.0.0.0:7777");
        assert_eq!(reloaded.max_concurrent_lookups, 64);
        assert_eq!(reloaded.whois_server_override, "whois.ripe.net");
        assert_eq!(
            reloaded.proxy.socks5.as_deref(),
            Some("socks5://127.0.0.1:9050")
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_proxy_validation() {
        let mut cfg = AppConfig::default();

        // Valid proxy URLs.
        cfg.proxy.http = Some("http://proxy:3128".to_string());
        cfg.proxy.https = Some("https://proxy:3129".to_string());
        cfg.proxy.socks4 = Some("socks4://proxy:1080".to_string());
        cfg.proxy.socks5 = Some("socks5://proxy:1081".to_string());
        assert!(cfg.validate().is_ok());

        // Wrong scheme for http field.
        cfg.proxy.http = Some("socks5://proxy:3128".to_string());
        assert!(cfg.validate().is_err());

        // Wrong scheme for socks5 field.
        cfg.proxy.http = None;
        cfg.proxy.socks5 = Some("http://proxy:1081".to_string());
        assert!(cfg.validate().is_err());

        // Clearing invalid fields restores validity.
        cfg.proxy = ProxyConfig::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validation_listen_addr() {
        let mut cfg = AppConfig::default();
        cfg.listen_addr = "no-colon".to_string();
        assert!(cfg.validate().is_err());

        cfg.listen_addr = "127.0.0.1:8080".to_string();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validation_timeout_zero() {
        let mut cfg = AppConfig::default();
        cfg.default_timeout_ms = 0;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
    }

    #[test]
    fn test_validation_concurrency_zero() {
        let mut cfg = AppConfig::default();
        cfg.max_concurrent_lookups = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_whois_override() {
        let path = temp_path("whois_override");
        let mut cfg = AppConfig::default();
        cfg.whois_server_override = "whois.arin.net".to_string();
        cfg.save(&path).unwrap();

        let reloaded = AppConfig::load(&path).unwrap();
        assert_eq!(reloaded.whois_server_override, "whois.arin.net");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_save_creates_parent_dirs() {
        let dir = env::temp_dir().join("atlasip_test_nested_dir_xyz");
        let path = dir.join("sub").join("config.toml");
        let _ = fs::remove_dir_all(&dir);

        AppConfig::default().save(&path).expect("save should create parent dirs");
        assert!(path.exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
