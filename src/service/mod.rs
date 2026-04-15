// ── Service management ──────────────────────────────────────────────────────
// Platform-specific helpers to install, start, and remove AtlasIP as a system
// service (Windows SCM, macOS launchd, Linux systemd).
//
// Each sub-module exposes at minimum:
//   • install()   → anyhow::Result<()>
//   • uninstall() → anyhow::Result<()>
//
// On Windows the sub-module additionally exposes the service-dispatcher entry
// point that must be invoked from `main()` when running under the SCM.

pub mod linux;
pub mod macos;

#[cfg(windows)]
#[cfg(not(feature = "ui"))]
pub mod windows;

// ---------------------------------------------------------------------------
// Common helpers
// ---------------------------------------------------------------------------

/// Return the absolute path of the current executable.
///
/// Used by every platform to record the correct binary path in the service
/// manifest (plist, unit file, SCM entry).
pub fn current_exe_path() -> anyhow::Result<std::path::PathBuf> {
    std::env::current_exe().map_err(|e| anyhow::anyhow!("cannot locate current executable: {e}"))
}
