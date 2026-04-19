// ── Linux systemd service management (spec §8) ─────────────────────────────
// Functions below are only called from cli.rs when compiling for Linux.
// The `allow(dead_code)` suppresses false positives on macOS / Windows CI.
#![allow(dead_code)]
//
// Commands exposed via the CLI:
//   atlasip install-service-linux    (requires root / sudo)
//   atlasip uninstall-service-linux  (requires root / sudo)
//
// The generated unit file is written to /etc/systemd/system/atlasip.service.

use anyhow::{Context, Result};
use std::{
    fs,
    path::Path,
    process::Command,
};

use super::current_exe_path;

const UNIT_PATH: &str = "/etc/systemd/system/atlasip.service";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Install AtlasIP as a systemd service.
///
/// 1. Writes the unit file to `/etc/systemd/system/atlasip.service`.
/// 2. Runs `systemctl daemon-reload`.
/// 3. Runs `systemctl enable --now atlasip`.
///
/// Requires root privileges (or `sudo`).
pub fn install() -> Result<()> {
    let exe = current_exe_path()?;
    let unit = generate_unit_file(&exe.display().to_string());

    // Write unit file (requires root).
    fs::write(UNIT_PATH, &unit)
        .with_context(|| format!("failed to write unit file to {UNIT_PATH} (run as root)"))?;

    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", "--now", "atlasip"])?;

    println!("{}", crate::i18n::t("service.linux.installed"));
    println!("{}", crate::i18n::t("service.linux.hint.status"));
    println!("{}", crate::i18n::t("service.linux.hint.logs"));
    Ok(())
}

/// Uninstall (stop + disable + remove) the AtlasIP systemd service.
///
/// Requires root privileges.
pub fn uninstall() -> Result<()> {
    // Stop and disable — ignore errors (service may not be running/enabled).
    let _ = systemctl(&["disable", "--now", "atlasip"]);

    if Path::new(UNIT_PATH).exists() {
        fs::remove_file(UNIT_PATH)
            .with_context(|| format!("failed to remove {UNIT_PATH} (run as root)"))?;
    }

    systemctl(&["daemon-reload"])?;
    println!("{}", crate::i18n::t("service.linux.uninstalled"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit file template
// ---------------------------------------------------------------------------

/// Generate the content of the systemd unit file for AtlasIP.
pub fn generate_unit_file(exe_path: &str) -> String {
    format!(
        r#"[Unit]
Description=AtlasIP headless backend service
Documentation=https://github.com/Absol3m/atlasip
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={exe_path} serve
Restart=always
RestartSec=5
# Never expose the API beyond localhost.
# The --headless flag is handled via config; `serve` binds 127.0.0.1 only.
Environment=RUST_LOG=atlasip=info
StandardOutput=journal
StandardError=journal
SyslogIdentifier=atlasip

# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/etc/atlasip /var/lib/atlasip

[Install]
WantedBy=multi-user.target
"#
    )
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn systemctl(args: &[&str]) -> Result<()> {
    let status = Command::new("systemctl")
        .args(args)
        .status()
        .with_context(|| format!("failed to run systemctl {}", args.join(" ")))?;

    if !status.success() {
        anyhow::bail!(
            "systemctl {} exited with status {}",
            args.join(" "),
            status
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_file_contains_required_fields() {
        let unit = generate_unit_file("/usr/local/bin/atlasip");
        assert!(unit.contains("ExecStart=/usr/local/bin/atlasip serve"));
        assert!(unit.contains("Restart=always"));
        assert!(unit.contains("WantedBy=multi-user.target"));
        assert!(unit.contains("After=network-online.target"));
    }

    #[test]
    fn test_unit_file_exe_path_interpolated() {
        let unit = generate_unit_file("/opt/atlasip/bin/atlasip");
        assert!(unit.contains("ExecStart=/opt/atlasip/bin/atlasip serve"));
    }
}
