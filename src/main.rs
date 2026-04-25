mod bgp;
mod cache;
mod cli;
mod config;
mod dns;
mod export;
mod geoip;
mod http;
mod i18n;
mod metrics;
mod models;
mod rdap;
mod retry;
mod service;
mod utils;
mod whois;

use clap::Parser;

// ---------------------------------------------------------------------------
// Windows service dispatcher (spec §6)
// ---------------------------------------------------------------------------
//
// `define_windows_service!` must be called at crate-root level.  It generates
// an `extern "system"` C-compatible function (`ffi_service_main`) that the
// Windows Service Control Manager calls when it starts this process as a
// service.  The second argument (`windows_service_entry`) is the Rust function
// that the generated FFI trampoline delegates to.
//
// On non-Windows targets this entire block is compiled away.

#[cfg(windows)]
windows_service::define_windows_service!(ffi_service_main, windows_service_entry);

/// Rust-level service entry point called from the SCM trampoline.
#[cfg(windows)]
fn windows_service_entry(args: Vec<std::ffi::OsString>) {
    service::windows::service_main(args);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Structured logging (P3-PERF-015) ─────────────────────────────────────
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "atlasip=info".into());

    if std::env::var("RUST_LOG_FORMAT").as_deref() == Ok("json") {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .init();
    }

    // ── Windows SCM dispatch (spec §6) ───────────────────────────────────────
    // When Windows starts AtlasIP as a service it passes "run-service" as the
    // first argument (set in ServiceInfo::launch_arguments during install).
    // We hand off to the service dispatcher before Clap even touches argv so
    // that the SCM handshake happens as early as possible.
    #[cfg(windows)]
    if std::env::args().nth(1).as_deref() == Some("run-service") {
        return service::windows::run_dispatcher();
    }

    let cli = cli::Cli::parse();
    cli::run(cli).await
}
