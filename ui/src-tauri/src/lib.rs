pub mod config;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Manager,
};

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn get_config() -> Result<config::AppConfig, String> {
    Ok(config::load_config())
}

/// Persist config and sync OS-level autostart registration.
#[tauri::command]
fn set_config(config: config::AppConfig, app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let mgr = app.autolaunch();
    if config.autostart {
        mgr.enable().map_err(|e: tauri_plugin_autostart::Error| e.to_string())?;
    } else {
        mgr.disable().map_err(|e: tauri_plugin_autostart::Error| e.to_string())?;
    }
    config::save_config(&config)
}

#[tauri::command]
fn translate(key: String) -> String {
    atlasip::i18n::t(&key)
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let backend_cfg = atlasip::config::AppConfig::load(
        atlasip::config::AppConfig::default_path(),
    )
    .unwrap_or_default();

    if backend_cfg.headless {
        tracing_init();
        tracing::info!("AtlasIP starting in headless mode (no UI)");
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("AtlasIP: failed to create tokio runtime")
            .block_on(atlasip::start_server_with_config(backend_cfg));
        return;
    }

    std::thread::spawn(|| {
        tokio::runtime::Runtime::new()
            .expect("AtlasIP: failed to create tokio runtime")
            .block_on(atlasip::start_server());
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            build_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide to tray instead of quitting when the user closes the window.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![get_config, set_config, translate])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── Tray ──────────────────────────────────────────────────────────────────────

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show AtlasIP", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit",         true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("no default window icon — check bundle.icon in tauri.conf.json");

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("AtlasIP")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click toggles the window; right-click opens the menu (default).
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        show_window(app);
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tracing_init() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "atlasip=info".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init();
}
