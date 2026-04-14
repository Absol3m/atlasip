use clap::{Parser, Subcommand};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::info;

use crate::{
    config::{AppConfig, DnsMode},
    export::{self, ExportFormat},
    http::{self, AppState},
    i18n::I18n,
    models::IpRecord,
    service,
    utils,
};

// ---------------------------------------------------------------------------
// CLI structure (spec §2.5)
// ---------------------------------------------------------------------------

/// AtlasIP — Modern IP OSINT tool.
#[derive(Parser, Debug)]
#[command(
    name = "atlasip",
    version,
    about = "Modern OSINT tool for IP analysis",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Look up one IP address or hostname.
    Lookup {
        /// IP address or hostname to analyse.
        target: String,

        /// Output format: json | csv | tsv | txt | txt-h | html | html-v | xml | whois.
        #[arg(long, default_value = "json")]
        format: String,

        /// Write output to this file instead of stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,

        /// Use RDAP only; skip WHOIS.
        #[arg(long)]
        rdap_only: bool,

        /// Use WHOIS only; skip RDAP.
        #[arg(long)]
        whois_only: bool,
    },

    /// Export previously cached lookup results.
    Export {
        /// Output format: csv | tsv | txt | txt-h | html | html-v | xml | whois.
        #[arg(long)]
        format: String,

        /// Destination file path.
        #[arg(long, short)]
        output: PathBuf,

        /// Comma-separated record UUIDs to include (omit = all).
        #[arg(long)]
        ids: Option<String>,
    },

    /// Start the local HTTP API server.
    Serve {
        /// Address to bind (overrides config listen_addr).
        #[arg(long)]
        bind: Option<String>,
    },

    /// Read or update persistent configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    // ── Service management (spec §6/7/8) ────────────────────────────────────

    /// Install AtlasIP as a Windows service (Administrator required).
    ///
    /// The service starts automatically with Windows and exposes the local
    /// HTTP API on 127.0.0.1:<port>.
    #[cfg(windows)]
    InstallService,

    /// Uninstall the AtlasIP Windows service (Administrator required).
    #[cfg(windows)]
    UninstallService,

    /// Install AtlasIP as a macOS LaunchAgent or LaunchDaemon.
    ///
    /// Pass `--daemon` to install as a system daemon under
    /// /Library/LaunchDaemons (requires root).  The default installs a user
    /// agent under ~/Library/LaunchAgents.
    #[cfg(target_os = "macos")]
    InstallServiceMacos {
        /// Install as a system daemon (requires root).
        #[arg(long)]
        daemon: bool,
    },

    /// Uninstall the AtlasIP macOS launchd service.
    #[cfg(target_os = "macos")]
    UninstallServiceMacos {
        /// Remove the system daemon (requires root).
        #[arg(long)]
        daemon: bool,
    },

    /// Install AtlasIP as a systemd service (requires root / sudo).
    ///
    /// Writes /etc/systemd/system/atlasip.service and enables it.
    #[cfg(target_os = "linux")]
    InstallServiceLinux,

    /// Uninstall the AtlasIP systemd service (requires root / sudo).
    #[cfg(target_os = "linux")]
    UninstallServiceLinux,

    /// Internal: invoked by the Windows SCM to run AtlasIP as a service.
    ///
    /// Do NOT call this manually.
    #[cfg(windows)]
    #[command(hide = true)]
    RunService,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Print current configuration as JSON.
    Show,

    /// Set a single configuration value (format: key=value).
    Set {
        /// Key-value pair, e.g. `language=en` or `proxy.socks5=socks5://host:1080`.
        kv: String,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the CLI command selected by the user.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Lookup {
            target,
            format,
            output,
            ..
        } => cmd_lookup(target, format, output).await,

        Commands::Export { format, output, ids } => {
            cmd_export(format, output, ids).await
        }

        Commands::Serve { bind } => cmd_serve(bind).await,

        Commands::Config { action } => match action {
            ConfigAction::Show => cmd_config_show(),
            ConfigAction::Set { kv } => cmd_config_set(kv),
        },

        // ── Service commands ─────────────────────────────────────────────────
        #[cfg(windows)]
        Commands::InstallService => service::windows::install(),

        #[cfg(windows)]
        Commands::UninstallService => service::windows::uninstall(),

        #[cfg(windows)]
        Commands::RunService => service::windows::run_dispatcher(),

        #[cfg(target_os = "macos")]
        Commands::InstallServiceMacos { daemon } => {
            service::macos::install(!daemon) // user_mode = !daemon
        }

        #[cfg(target_os = "macos")]
        Commands::UninstallServiceMacos { daemon } => {
            service::macos::uninstall(!daemon)
        }

        #[cfg(target_os = "linux")]
        Commands::InstallServiceLinux => service::linux::install(),

        #[cfg(target_os = "linux")]
        Commands::UninstallServiceLinux => service::linux::uninstall(),
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

/// `atlasip lookup <target>`
async fn cmd_lookup(
    target: String,
    format: String,
    output: Option<PathBuf>,
) -> anyhow::Result<()> {
    let config = load_config();
    let i18n = try_i18n();

    // Validate that the target is not obviously private (best-effort).
    if let Ok(ip) = target.parse::<std::net::IpAddr>() {
        if !utils::is_public_ip(&ip) {
            let msg = translate(&i18n, "error.private_ip");
            anyhow::bail!("{msg}");
        }
    }

    info!("Looking up: {target}");
    let cache   = crate::cache::LookupCache::new(std::time::Duration::from_secs(config.cache_ttl_secs));
    let metrics = crate::metrics::RequestMetrics::new();
    let record  = http::perform_lookup(1, &target, &config, &cache, &metrics).await;

    // Save to session file so `export` can reuse results.
    save_session(&[record.clone()])?;

    let content = render_record(&record, &format, config.csv_with_header)?;
    write_output(&content, output.as_deref())
}

/// `atlasip export --format <fmt> --output <path> [--ids <id1,id2,...>]`
async fn cmd_export(
    format: String,
    output: PathBuf,
    ids: Option<String>,
) -> anyhow::Result<()> {
    let config = load_config();

    let mut records = load_session()?;

    // Filter by IDs if provided.
    if let Some(ids_str) = ids {
        let wanted: Vec<&str> = ids_str.split(',').map(str::trim).collect();
        records.retain(|r| wanted.contains(&r.id.to_string().as_str()));
    }

    let fmt = ExportFormat::from_str(&format)?;
    let content = export::export(&records, fmt, config.csv_with_header)?;
    write_output(&content, Some(&output))
}

/// `atlasip serve [--bind <addr>]`
async fn cmd_serve(bind_override: Option<String>) -> anyhow::Result<()> {
    let config = load_config();
    let bind_addr = bind_override.unwrap_or_else(|| config.listen_addr.clone());

    let state = AppState::with_config(config);

    let router = http::build_router(state);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("AtlasIP API listening on http://{bind_addr}");
    axum::serve(listener, router).await?;
    Ok(())
}

/// `atlasip config show`
fn cmd_config_show() -> anyhow::Result<()> {
    let config = load_config();
    let json = serde_json::to_string_pretty(&config)?;
    println!("{json}");
    Ok(())
}

/// `atlasip config set <key=value>`
fn cmd_config_set(kv: String) -> anyhow::Result<()> {
    let mut config = load_config();
    apply_key_value(&mut config, &kv)?;
    config.validate()?;
    let path = AppConfig::default_path();
    config.save(&path)?;
    println!("Saved: {kv}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers — config
// ---------------------------------------------------------------------------

/// Load config from the default path, falling back to defaults on any error.
fn load_config() -> AppConfig {
    AppConfig::load(AppConfig::default_path()).unwrap_or_default()
}

/// Apply a `key=value` pair to `config`.
fn apply_key_value(config: &mut AppConfig, kv: &str) -> anyhow::Result<()> {
    let (key, value) = kv
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("Expected key=value, got: {kv}"))?;

    match key.trim() {
        // String fields
        "language"               => config.language               = value.to_owned(),
        "listen_addr"            => config.listen_addr            = value.to_owned(),
        "whois_server_override"  => config.whois_server_override  = value.to_owned(),
        "default_export_format"  => config.default_export_format  = value.to_owned(),
        "proxy_type"             => config.proxy_type             = value.to_owned(),
        "proxy_host"             => config.proxy_host             = value.to_owned(),

        // u64 fields
        "default_timeout_ms"  => config.default_timeout_ms  = parse_field(value, key)?,
        "dns_timeout_ms"      => config.dns_timeout_ms      = parse_field(value, key)?,
        "whois_timeout_ms"    => config.whois_timeout_ms    = parse_field(value, key)?,
        "rdap_timeout_ms"     => config.rdap_timeout_ms     = parse_field(value, key)?,
        "pause_duration_ms"   => config.pause_duration_ms   = parse_field(value, key)?,

        // u32 fields
        "max_concurrent_lookups" => config.max_concurrent_lookups = parse_field(value, key)?,
        "pause_every"            => config.pause_every            = parse_field(value, key)?,

        // u16 fields
        "proxy_port" => config.proxy_port = parse_field(value, key)?,

        // bool fields
        "ignore_private_ips" => config.ignore_private_ips = parse_bool(value, key)?,
        "auto_retry_failed"  => config.auto_retry_failed  = parse_bool(value, key)?,
        "csv_with_header"    => config.csv_with_header    = parse_bool(value, key)?,

        // Proxy URLs
        "proxy.http"   => config.proxy.http   = opt_str(value),
        "proxy.https"  => config.proxy.https  = opt_str(value),
        "proxy.socks4" => config.proxy.socks4 = opt_str(value),
        "proxy.socks5" => config.proxy.socks5 = opt_str(value),

        // Headless / service mode
        "headless" => config.headless = parse_bool(value, key)?,

        // DNS mode
        "dns_mode" => {
            config.dns_mode = match value.to_ascii_lowercase().as_str() {
                "system_only"  | "system"   => DnsMode::SystemOnly,
                "doh_only"     | "doh"      => DnsMode::DohOnly,
                "automatic"    | "auto"     => DnsMode::Automatic,
                "disabled"     | "off"      => DnsMode::Disabled,
                _ => anyhow::bail!("Invalid dns_mode '{value}': expected system_only | doh_only | automatic | disabled"),
            };
        }

        "doh_endpoint"          => config.doh_endpoint          = value.to_owned(),
        "dns_system_timeout_ms" => config.dns_system_timeout_ms = parse_field(value, key)?,

        _ => anyhow::bail!("Unknown config key: {key}"),
    }

    Ok(())
}

fn parse_field<T>(value: &str, key: &str) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|e| anyhow::anyhow!("Invalid value for '{key}': {e}"))
}

fn parse_bool(value: &str, key: &str) -> anyhow::Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => anyhow::bail!("Invalid boolean for '{key}': {value}"),
    }
}

fn opt_str(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

// ---------------------------------------------------------------------------
// Helpers — i18n
// ---------------------------------------------------------------------------

fn try_i18n() -> Option<I18n> {
    I18n::new("fr", "en").ok()
}

fn translate(i18n: &Option<I18n>, key: &str) -> String {
    i18n.as_ref().map(|i| i.t(key)).unwrap_or_else(|| key.to_owned())
}

// ---------------------------------------------------------------------------
// Helpers — rendering & output
// ---------------------------------------------------------------------------

/// Render a single record in the requested format.
fn render_record(
    record: &IpRecord,
    format: &str,
    with_header: bool,
) -> anyhow::Result<String> {
    if format == "json" {
        return Ok(serde_json::to_string_pretty(record)?);
    }
    let fmt = ExportFormat::from_str(format)?;
    export::export(std::slice::from_ref(record), fmt, with_header)
}

/// Write `content` to `path` if given, otherwise print to stdout.
fn write_output(content: &str, path: Option<&Path>) -> anyhow::Result<()> {
    match path {
        Some(p) => {
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(p, content)?;
            info!("Output written to {}", p.display());
        }
        None => {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            lock.write_all(content.as_bytes())?;
            if !content.ends_with('\n') {
                lock.write_all(b"\n")?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers — session cache
// ---------------------------------------------------------------------------

/// Path of the temporary session file where lookup results are cached.
fn session_path() -> PathBuf {
    std::env::temp_dir().join("atlasip_session.json")
}

/// Persist `records` to the session file (appends to any existing records).
fn save_session(records: &[IpRecord]) -> anyhow::Result<()> {
    let mut existing = load_session().unwrap_or_default();
    existing.extend_from_slice(records);
    let json = serde_json::to_string_pretty(&existing)?;
    fs::write(session_path(), json)?;
    Ok(())
}

/// Load records from the session file.  Returns an empty vec if missing.
fn load_session() -> anyhow::Result<Vec<IpRecord>> {
    let path = session_path();
    match fs::read_to_string(&path) {
        Ok(s) => Ok(serde_json::from_str(&s)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// Tests — clap parsing only (no network / disk I/O)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(args)
    }

    // ── lookup ───────────────────────────────────────────────────────────────

    #[test]
    fn test_cli_lookup_parses_args() {
        let cli = parse(&["atlasip", "lookup", "8.8.8.8"]).unwrap();
        match cli.command {
            Commands::Lookup { target, format, output, rdap_only, whois_only } => {
                assert_eq!(target, "8.8.8.8");
                assert_eq!(format, "json"); // default
                assert!(output.is_none());
                assert!(!rdap_only);
                assert!(!whois_only);
            }
            _ => panic!("expected Lookup"),
        }
    }

    #[test]
    fn test_cli_lookup_with_flags() {
        let cli = parse(&[
            "atlasip", "lookup", "1.1.1.1",
            "--format", "csv",
            "--output", "/tmp/out.csv",
            "--rdap-only",
        ])
        .unwrap();
        match cli.command {
            Commands::Lookup { target, format, output, rdap_only, .. } => {
                assert_eq!(target, "1.1.1.1");
                assert_eq!(format, "csv");
                assert_eq!(output.as_deref(), Some(Path::new("/tmp/out.csv")));
                assert!(rdap_only);
            }
            _ => panic!("expected Lookup"),
        }
    }

    #[test]
    fn test_cli_lookup_missing_target() {
        // target is required — parsing must fail without it.
        assert!(parse(&["atlasip", "lookup"]).is_err());
    }

    // ── export ───────────────────────────────────────────────────────────────

    #[test]
    fn test_cli_export_parses_args() {
        let cli = parse(&[
            "atlasip", "export",
            "--format", "csv",
            "--output", "/tmp/result.csv",
        ])
        .unwrap();
        match cli.command {
            Commands::Export { format, output, ids } => {
                assert_eq!(format, "csv");
                assert_eq!(output, PathBuf::from("/tmp/result.csv"));
                assert!(ids.is_none());
            }
            _ => panic!("expected Export"),
        }
    }

    #[test]
    fn test_cli_export_with_ids() {
        let cli = parse(&[
            "atlasip", "export",
            "--format", "xml",
            "--output", "/tmp/out.xml",
            "--ids", "uuid1,uuid2",
        ])
        .unwrap();
        match cli.command {
            Commands::Export { ids, format, .. } => {
                assert_eq!(ids.as_deref(), Some("uuid1,uuid2"));
                assert_eq!(format, "xml");
            }
            _ => panic!("expected Export"),
        }
    }

    #[test]
    fn test_cli_export_missing_required_args() {
        // --format and --output are both required.
        assert!(parse(&["atlasip", "export", "--format", "csv"]).is_err());
        assert!(parse(&["atlasip", "export", "--output", "/tmp/x.csv"]).is_err());
        assert!(parse(&["atlasip", "export"]).is_err());
    }

    // ── serve ────────────────────────────────────────────────────────────────

    #[test]
    fn test_cli_serve_parses_args() {
        let cli = parse(&["atlasip", "serve"]).unwrap();
        assert!(matches!(cli.command, Commands::Serve { bind: None }));
    }

    #[test]
    fn test_cli_serve_with_bind() {
        let cli = parse(&["atlasip", "serve", "--bind", "0.0.0.0:9090"]).unwrap();
        match cli.command {
            Commands::Serve { bind } => {
                assert_eq!(bind.as_deref(), Some("0.0.0.0:9090"));
            }
            _ => panic!("expected Serve"),
        }
    }

    // ── config show ──────────────────────────────────────────────────────────

    #[test]
    fn test_cli_config_show_parses_args() {
        let cli = parse(&["atlasip", "config", "show"]).unwrap();
        match cli.command {
            Commands::Config { action: ConfigAction::Show } => {}
            _ => panic!("expected Config Show"),
        }
    }

    // ── config set ───────────────────────────────────────────────────────────

    #[test]
    fn test_cli_config_set_parses_args() {
        let cli = parse(&["atlasip", "config", "set", "language=en"]).unwrap();
        match cli.command {
            Commands::Config {
                action: ConfigAction::Set { kv },
            } => {
                assert_eq!(kv, "language=en");
            }
            _ => panic!("expected Config Set"),
        }
    }

    #[test]
    fn test_cli_config_set_proxy() {
        let cli =
            parse(&["atlasip", "config", "set", "proxy.socks5=socks5://127.0.0.1:9050"])
                .unwrap();
        match cli.command {
            Commands::Config {
                action: ConfigAction::Set { kv },
            } => {
                assert_eq!(kv, "proxy.socks5=socks5://127.0.0.1:9050");
            }
            _ => panic!("expected Config Set"),
        }
    }

    // ── invalid command ──────────────────────────────────────────────────────

    #[test]
    fn test_cli_invalid_command() {
        assert!(parse(&["atlasip", "unknown"]).is_err());
        assert!(parse(&["atlasip"]).is_err());
    }

    // ── export format validation ─────────────────────────────────────────────

    #[test]
    fn test_cli_export_format_validation() {
        // Valid format strings must be accepted by ExportFormat::from_str.
        for fmt in &["csv", "tsv", "txt", "txt-v", "txt-h", "html", "html-h", "html-v", "xml", "whois"] {
            assert!(
                ExportFormat::from_str(fmt).is_ok(),
                "format '{fmt}' should be valid"
            );
        }
        // Invalid format strings must be rejected.
        for bad in &["pdf", "docx", "", "JSON", "CSV2"] {
            assert!(
                ExportFormat::from_str(bad).is_err(),
                "format '{bad}' should be invalid"
            );
        }
    }

    // ── apply_key_value ──────────────────────────────────────────────────────

    #[test]
    fn test_apply_key_value_string() {
        let mut cfg = AppConfig::default();
        apply_key_value(&mut cfg, "language=en").unwrap();
        assert_eq!(cfg.language, "en");
    }

    #[test]
    fn test_apply_key_value_u64() {
        let mut cfg = AppConfig::default();
        apply_key_value(&mut cfg, "default_timeout_ms=9000").unwrap();
        assert_eq!(cfg.default_timeout_ms, 9000);
    }

    #[test]
    fn test_apply_key_value_bool() {
        let mut cfg = AppConfig::default();
        apply_key_value(&mut cfg, "ignore_private_ips=false").unwrap();
        assert!(!cfg.ignore_private_ips);

        apply_key_value(&mut cfg, "csv_with_header=true").unwrap();
        assert!(cfg.csv_with_header);
    }

    #[test]
    fn test_apply_key_value_proxy() {
        let mut cfg = AppConfig::default();
        apply_key_value(&mut cfg, "proxy.socks5=socks5://127.0.0.1:9050").unwrap();
        assert_eq!(
            cfg.proxy.socks5.as_deref(),
            Some("socks5://127.0.0.1:9050")
        );
        // Clear a proxy field.
        apply_key_value(&mut cfg, "proxy.socks5=").unwrap();
        assert!(cfg.proxy.socks5.is_none());
    }

    #[test]
    fn test_apply_key_value_invalid_key() {
        let mut cfg = AppConfig::default();
        assert!(apply_key_value(&mut cfg, "nonexistent_key=value").is_err());
    }

    #[test]
    fn test_apply_key_value_missing_equals() {
        let mut cfg = AppConfig::default();
        assert!(apply_key_value(&mut cfg, "language").is_err());
    }
}
