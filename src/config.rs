use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Runtime configuration for AtlasIP.
/// Loaded from config.toml (spec section 7 — Configuration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: String,
    pub proxy_type: String,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub dns_timeout_ms: u64,
    pub whois_timeout_ms: u64,
    pub rdap_timeout_ms: u64,
    pub default_export_format: String,
    pub csv_with_header: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "fr".to_string(),
            proxy_type: "none".to_string(),
            proxy_host: String::new(),
            proxy_port: 0,
            dns_timeout_ms: 3000,
            whois_timeout_ms: 5000,
            rdap_timeout_ms: 5000,
            default_export_format: "csv".to_string(),
            csv_with_header: true,
        }
    }
}

impl AppConfig {
    /// Load config from the given path, falling back to defaults if the file
    /// does not exist.
    pub fn load(path: &PathBuf) -> Result<Self> {
        // TODO: read and parse TOML from `path`; return default if file is
        // missing; propagate parse errors.
        todo!("AppConfig::load")
    }

    /// Persist the current configuration to `path`.
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        // TODO: serialize to TOML and write to `path`.
        todo!("AppConfig::save")
    }

    /// Return the canonical path for the config file.
    pub fn default_path() -> PathBuf {
        // TODO: resolve platform-appropriate config directory
        // (e.g. XDG_CONFIG_HOME on Linux, ~/Library/Application Support on macOS).
        todo!("AppConfig::default_path")
    }
}
