// Public library surface used by the Tauri shell and external consumers.
// CLI-specific code (cli) stays bin-only.
pub mod bgp;
pub mod cache;
pub mod config;
pub mod geoip;
pub mod dns;
pub mod export;
pub mod http;
pub mod i18n;
pub mod metrics;
pub mod models;
pub mod rdap;
pub mod retry;
pub mod service;
pub mod utils;
pub mod whois;

// ---------------------------------------------------------------------------
// Convenience entry points
// ---------------------------------------------------------------------------

/// Bind the Axum HTTP server on the address specified in `AppConfig.listen_addr`
/// and serve forever.
///
/// The address is always validated to be a loopback address (`127.x.x.x` or
/// `::1`) before binding, so the API is never accidentally exposed externally.
///
/// Intended to be called from the Tauri shell inside a dedicated OS thread
/// with its own tokio runtime:
/// ```ignore
/// std::thread::spawn(|| {
///     tokio::runtime::Runtime::new().unwrap().block_on(atlasip::start_server());
/// });
/// ```
pub async fn start_server() {
    let cfg = config::AppConfig::load(config::AppConfig::default_path())
        .unwrap_or_default();

    // Safety: never expose the API beyond localhost.
    let addr = sanitize_listen_addr(&cfg.listen_addr);

    let state  = http::AppState::with_config(cfg);
    let router = http::build_router(state);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("AtlasIP: failed to bind {addr} — {e}"));

    tracing::info!("AtlasIP API listening on http://{addr}");

    axum::serve(listener, router)
        .await
        .expect("AtlasIP: HTTP server exited unexpectedly");
}

/// Like [`start_server`] but accepts a pre-loaded config.
///
/// Useful when the caller (e.g. the Windows service) already loaded the config
/// and wants to avoid reading the file twice.
pub async fn start_server_with_config(cfg: config::AppConfig) {
    let addr   = sanitize_listen_addr(&cfg.listen_addr);
    let state  = http::AppState::with_config(cfg);
    let router = http::build_router(state);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("AtlasIP: failed to bind {addr} — {e}"));

    tracing::info!("AtlasIP API listening on http://{addr}");

    axum::serve(listener, router)
        .await
        .expect("AtlasIP: HTTP server exited unexpectedly");
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Ensure the listen address binds to 127.x.x.x or ::1 only.
///
/// If the configured host is anything other than a loopback address, this
/// function overrides the host part with `127.0.0.1` while preserving the
/// port.  This prevents accidental external exposure when `listen_addr` has
/// been misconfigured.
fn sanitize_listen_addr(addr: &str) -> String {
    if let Some(pos) = addr.rfind(':') {
        let host = &addr[..pos];
        let port = &addr[pos + 1..];
        let is_loopback = host == "127.0.0.1"
            || host == "::1"
            || host.starts_with("127.");
        if is_loopback {
            addr.to_owned()
        } else {
            tracing::warn!(
                "AtlasIP: listen_addr '{addr}' is not loopback — overriding host to 127.0.0.1"
            );
            format!("127.0.0.1:{port}")
        }
    } else {
        // Malformed — fall back to default.
        tracing::warn!("AtlasIP: malformed listen_addr '{addr}' — using 127.0.0.1:8080");
        "127.0.0.1:8080".to_owned()
    }
}
