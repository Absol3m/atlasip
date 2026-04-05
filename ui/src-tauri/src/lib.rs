/// Tauri shell for AtlasIP.
///
/// The embedded Axum backend is started on a background OS thread with its
/// own tokio runtime *before* Tauri's event loop begins.  The SvelteKit
/// frontend continues to call `http://127.0.0.1:8080` via fetch() — no
/// frontend changes required.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── Start the embedded HTTP backend ─────────────────────────────────────
    // A dedicated OS thread owns the tokio runtime so it does not interfere
    // with Tauri's own async machinery.  The thread runs for the lifetime of
    // the process; when Tauri exits the process, this thread is torn down
    // automatically.
    std::thread::spawn(|| {
        tokio::runtime::Runtime::new()
            .expect("AtlasIP: failed to create tokio runtime")
            .block_on(atlasip::start_server());
    });

    // ── Start the Tauri / WebView shell ─────────────────────────────────────
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
