pub mod config;

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Return the full application configuration loaded from `config.toml`.
#[tauri::command]
fn get_config() -> Result<config::AppConfig, String> {
    Ok(config::load_config())
}

/// Persist a new configuration to `config.toml`.
#[tauri::command]
fn set_config(config: config::AppConfig) -> Result<(), String> {
    config::save_config(&config)
}

// ── App entry point ───────────────────────────────────────────────────────────

/// Tauri shell for AtlasIP.
///
/// Behaviour depends on `AppConfig.headless`:
///
/// * `headless = false` (default) — normal desktop mode: the embedded Axum
///   backend is started on a background OS thread, then the Tauri WebView
///   window opens.
///
/// * `headless = true` — service mode: only the HTTP backend is started.
///   No Tauri event loop, no window, no WebView.  The process blocks until
///   the server stops (i.e. forever under normal service operation).
///   This allows the Tauri-packaged binary to serve as the system service
///   executable on all three platforms without a separate CLI binary.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load the AtlasIP backend config to check for headless mode.
    // We read it here (before Tauri initialises) so we can branch early.
    let backend_cfg = atlasip::config::AppConfig::load(
        atlasip::config::AppConfig::default_path(),
    )
    .unwrap_or_default();

    if backend_cfg.headless {
        // ── Headless mode ────────────────────────────────────────────────────
        // Skip the Tauri event loop entirely.  Run the HTTP server directly on
        // the current thread with its own tokio runtime.  This is the code path
        // used by all three system-service configurations.
        tracing_init();
        tracing::info!("AtlasIP starting in headless mode (no UI)");

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("AtlasIP: failed to create tokio runtime")
            .block_on(atlasip::start_server_with_config(backend_cfg));

        // start_server_with_config only returns if the server exits.
        return;
    }

    // ── Normal desktop mode ───────────────────────────────────────────────────
    // A dedicated OS thread owns the tokio runtime so it does not interfere
    // with Tauri's own async machinery.  The thread runs for the lifetime of
    // the process; when Tauri exits the process, the thread is torn down.
    std::thread::spawn(|| {
        tokio::runtime::Runtime::new()
            .expect("AtlasIP: failed to create tokio runtime")
            .block_on(atlasip::start_server());
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_config, set_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Initialise a minimal tracing subscriber (used in headless mode where Tauri
/// does not set one up).  Calling this after the Tauri builder has run is safe
/// because `try_init` is a no-op if a subscriber is already installed.
fn tracing_init() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "atlasip=info".into());

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init();
}
