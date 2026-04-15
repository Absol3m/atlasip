// ── Windows Service Manager integration (spec §6) ──────────────────────────
//
// This module compiles only on Windows (`#[cfg(windows)]` is applied at the
// module declaration in service/mod.rs).
//
// CLI commands:
//   atlasip.exe install-service    (requires Administrator)
//   atlasip.exe uninstall-service  (requires Administrator)
//
// When Windows SCM starts the service it calls:
//   atlasip.exe run-service
//
// The `define_windows_service!` macro that wires up the SCM dispatcher is
// declared in `src/main.rs` (must live at crate root level).

use anyhow::{Context, Result};
use std::{
    ffi::OsString,
    sync::{Arc, Mutex},
    time::Duration,
};
use crate::config::AppConfig;
use crate::http::{self, AppState};
use windows_service::{
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl,
        ServiceExitCode, ServiceInfo, ServiceStartType, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

pub const SERVICE_NAME: &str = "AtlasIPService";
const SERVICE_DISPLAY_NAME: &str = "AtlasIP Service";
const SERVICE_DESCRIPTION: &str =
    "AtlasIP headless backend — local HTTP API for IP analysis (127.0.0.1)";

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

/// Register AtlasIP as a Windows service that starts automatically.
///
/// Requires the process to be running with Administrator privileges.
pub fn install() -> Result<()> {
    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
            .context("failed to open Service Control Manager (run as Administrator)")?;

    let exe = super::current_exe_path()?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe,
        // The SCM will pass `run-service` so main() knows to enter the
        // service dispatcher instead of the normal CLI flow.
        launch_arguments: vec![OsString::from("run-service")],
        dependencies: vec![],
        account_name: None,  // LocalSystem
        account_password: None,
    };

    let service = manager
        .create_service(&service_info, ServiceAccess::CHANGE_CONFIG)
        .context("failed to create Windows service")?;

    service
        .set_description(SERVICE_DESCRIPTION)
        .context("failed to set service description")?;

    println!("AtlasIP Service installed successfully.");
    println!("  Start  : sc start {SERVICE_NAME}");
    println!("  Stop   : sc stop  {SERVICE_NAME}");
    println!("  Remove : atlasip.exe uninstall-service");
    Ok(())
}

// ---------------------------------------------------------------------------
// Uninstall
// ---------------------------------------------------------------------------

/// Stop (if running) and delete the AtlasIP Windows service.
///
/// Requires Administrator privileges.
pub fn uninstall() -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT,
    )
    .context("failed to open Service Control Manager")?;

    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
        )
        .context("failed to open AtlasIP service (is it installed?)")?;

    // Stop the service if it is currently running.
    let status = service.query_status().context("failed to query service status")?;
    if status.current_state != ServiceState::Stopped {
        service.stop().context("failed to stop the service")?;
        // Brief busy-wait — Windows needs a moment before DELETE is accepted.
        let mut attempts = 0u32;
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let s = service.query_status().context("failed to query service status")?;
            if s.current_state == ServiceState::Stopped || attempts > 10 {
                break;
            }
            attempts += 1;
        }
    }

    service.delete().context("failed to delete service")?;
    println!("AtlasIP Service uninstalled.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Service main — called by define_windows_service! in main.rs
// ---------------------------------------------------------------------------

/// Entry point invoked by the Windows SCM after `service_dispatcher::start`.
///
/// This function:
/// 1. Registers the service control handler.
/// 2. Sets the service status to Running.
/// 3. Starts the AtlasIP Axum HTTP server.
/// 4. Waits for a Stop/Shutdown signal.
/// 5. Sets the service status to Stopped.
///
/// **Must not be called directly** — use `run_dispatcher()` from `main()`.
pub fn service_main(_args: Vec<OsString>) {
    if let Err(e) = run_service_inner() {
        // Log to Windows Event Log as best-effort.
        eprintln!("AtlasIP service error: {e}");
    }
}

fn run_service_inner() -> Result<()> {
    // Channel used to signal the HTTP server to shut down when SCM sends Stop.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));

    let event_handler = {
        let shutdown_tx = Arc::clone(&shutdown_tx);
        move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    if let Some(tx) = shutdown_tx.lock().unwrap().take() {
                        let _ = tx.send(());
                    }
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)
        .context("failed to register service control handler")?;

    // Notify SCM: we are now Running.
    status_handle
        .set_service_status(running_status())
        .context("failed to set service status to Running")?;

    // Build and run the tokio runtime + HTTP server.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?
        .block_on(async move {
            let cfg = AppConfig::load(
                AppConfig::default_path(),
            )
            .unwrap_or_default();

            let addr = cfg.listen_addr.clone();

            let state  = AppState::with_config(cfg);
            let router = http::build_router(state);

            let listener = tokio::net::TcpListener::bind(addr.clone())
                .await
                .expect("AtlasIP: failed to bind address");

            tracing::info!("AtlasIP service listening on http://{addr}");

            tokio::select! {
                result = axum::serve(listener, router) => {
                    if let Err(e) = result {
                        tracing::error!("HTTP server error: {e}");
                    }
                }
                _ = async move {
                    let _ = shutdown_rx.await;
                    tracing::info!("AtlasIP service received stop signal");
                } => {}
            }
        });

    // Notify SCM: we have stopped cleanly.
    status_handle
        .set_service_status(stopped_status())
        .context("failed to set service status to Stopped")?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Dispatcher entry point
// ---------------------------------------------------------------------------

/// Invoke the Windows service dispatcher.
///
/// Call this from `main()` when `run-service` is the first CLI argument.
/// Blocks until the SCM sends a Stop signal.
pub fn run_dispatcher() -> Result<()> {
    // The `ffi_service_main` symbol is defined by `define_windows_service!`
    // in `src/main.rs`.  We reference it by name here via a type alias so the
    // linker can resolve it.  The actual function is generated at the call site.
    windows_service::service_dispatcher::start(SERVICE_NAME, service_main_wrapper)
        .context("service dispatcher failed — was the binary invoked by the SCM?")?;
    Ok(())
}

// Forward declaration so this module can compile; the real symbol is emitted
// by `define_windows_service!` in main.rs.
unsafe extern "system" {
    fn ffi_service_main(num_service_args: u32, service_arg_vectors: *mut *mut u16);
}

// Safe wrapper required by service_dispatcher::start — Rust 2024 marks
// extern "system" symbols as unsafe, but the dispatcher expects a safe fn.
extern "system" fn service_main_wrapper(
    num_service_args: u32,
    service_arg_vectors: *mut *mut u16,
) {
    unsafe { ffi_service_main(num_service_args, service_arg_vectors); }
}

// ---------------------------------------------------------------------------
// Status helpers
// ---------------------------------------------------------------------------

fn running_status() -> ServiceStatus {
    ServiceStatus {
        service_type:     ServiceType::OWN_PROCESS,
        current_state:    ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code:        ServiceExitCode::Win32(0),
        checkpoint:       0,
        wait_hint:        Duration::default(),
        process_id:       None,
    }
}

fn stopped_status() -> ServiceStatus {
    ServiceStatus {
        service_type:     ServiceType::OWN_PROCESS,
        current_state:    ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code:        ServiceExitCode::Win32(0),
        checkpoint:       0,
        wait_hint:        Duration::default(),
        process_id:       None,
    }
}
