// Public library surface used by the Tauri shell.
// Only modules required by the embedded HTTP server are exported here;
// CLI-specific code (cli, i18n) stays bin-only.
pub mod cache;
pub mod config;
pub mod dns;
pub mod export;
pub mod http;
pub mod metrics;
pub mod models;
pub mod rdap;
pub mod retry;
pub mod utils;
pub mod whois;

/// Bind the Axum HTTP server on `127.0.0.1:8080` and serve forever.
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
    let state  = http::AppState::with_config(cfg);
    let router = http::build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("AtlasIP: failed to bind port 8080 — is another instance running?");

    tracing::info!("AtlasIP API listening on http://127.0.0.1:8080");

    axum::serve(listener, router)
        .await
        .expect("AtlasIP: HTTP server exited unexpectedly");
}
