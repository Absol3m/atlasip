// ── macOS launchd service management (spec §7) ─────────────────────────────
// Functions below are only called from cli.rs when compiling for macOS.
// The `allow(dead_code)` suppresses false positives on Windows / Linux CI.
#![allow(dead_code)]
//
// Two installation targets are supported:
//
// • LaunchDaemon  — /Library/LaunchDaemons/com.atlasip.service.plist
//                   Runs as root at boot, before any user logs in.
//                   Requires root / sudo.
//
// • LaunchAgent   — ~/Library/LaunchAgents/com.atlasip.service.plist
//                   Runs as the current user at login.
//                   No elevated privileges required.
//
// CLI commands:
//   atlasip install-service-macos   [--user]
//   atlasip uninstall-service-macos [--user]

use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::current_exe_path;

const LABEL: &str = "com.atlasip.service";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Install AtlasIP as a launchd service.
///
/// * `user_mode = false` → writes to `/Library/LaunchDaemons/` (system daemon,
///   requires root).
/// * `user_mode = true`  → writes to `~/Library/LaunchAgents/` (user agent, no
///   elevated privileges needed).
pub fn install(user_mode: bool) -> Result<()> {
    let exe = current_exe_path()?;
    let plist_path = plist_path(user_mode)?;

    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let plist = generate_plist(&exe.display().to_string(), user_mode);
    fs::write(&plist_path, &plist)
        .with_context(|| format!("failed to write plist to {}", plist_path.display()))?;

    // Load the service immediately.
    launchctl(&["load", "-w", plist_path.to_str().unwrap()])?;

    let mode_label = if user_mode { "user agent" } else { "system daemon" };
    println!("{}", crate::i18n::t("service.macos.installed").replace("{mode}", mode_label));
    println!("{}", crate::i18n::t("service.macos.hint.status"));
    println!("{}", crate::i18n::t("service.macos.hint.plist").replace("{path}", &plist_path.display().to_string()));
    Ok(())
}

/// Uninstall the AtlasIP launchd service.
pub fn uninstall(user_mode: bool) -> Result<()> {
    let plist_path = plist_path(user_mode)?;

    if plist_path.exists() {
        // Unload first — ignore error (may not be loaded).
        let _ = launchctl(&["unload", "-w", plist_path.to_str().unwrap()]);

        fs::remove_file(&plist_path)
            .with_context(|| format!("failed to remove {}", plist_path.display()))?;
    } else {
        println!("{}", crate::i18n::t("service.macos.not_found").replace("{path}", &plist_path.display().to_string()));
    }

    println!("{}", crate::i18n::t("service.macos.uninstalled"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Plist template
// ---------------------------------------------------------------------------

/// Generate the launchd plist XML for AtlasIP.
pub fn generate_plist(exe_path: &str, user_mode: bool) -> String {
    // For a system daemon we run as nobody; for a user agent we let launchd
    // inherit the session user.
    let user_key = if user_mode {
        String::new()
    } else {
        "    <key>UserName</key>\n    <string>nobody</string>\n".to_string()
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>

    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>serve</string>
    </array>

    <!-- Keep the service alive if it crashes. -->
    <key>KeepAlive</key>
    <true/>

    <!-- Start at boot / login. -->
    <key>RunAtLoad</key>
    <true/>

    <!-- Logging -->
    <key>StandardOutPath</key>
    <string>/var/log/atlasip.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/atlasip.error.log</string>

    <!-- Environment -->
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>atlasip=info</string>
    </dict>
{user_key}
    <!-- Throttle rapid restarts (seconds). -->
    <key>ThrottleInterval</key>
    <integer>10</integer>
</dict>
</plist>
"#
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn plist_path(user_mode: bool) -> Result<PathBuf> {
    if user_mode {
        let home = std::env::var("HOME")
            .context("$HOME is not set — cannot locate LaunchAgents directory")?;
        Ok(PathBuf::from(home)
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist")))
    } else {
        Ok(Path::new("/Library/LaunchDaemons").join(format!("{LABEL}.plist")))
    }
}

fn launchctl(args: &[&str]) -> Result<()> {
    let status = Command::new("launchctl")
        .args(args)
        .status()
        .with_context(|| format!("failed to run launchctl {}", args.join(" ")))?;

    if !status.success() {
        anyhow::bail!(
            "launchctl {} exited with status {}",
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
    fn test_plist_contains_required_keys() {
        let plist = generate_plist("/usr/local/bin/atlasip", false);
        assert!(plist.contains("<string>com.atlasip.service</string>"));
        assert!(plist.contains("<string>/usr/local/bin/atlasip</string>"));
        assert!(plist.contains("<string>serve</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
    }

    #[test]
    fn test_plist_user_mode_omits_username_key() {
        let plist_user   = generate_plist("/bin/atlasip", true);
        let plist_system = generate_plist("/bin/atlasip", false);
        assert!(!plist_user.contains("<key>UserName</key>"));
        assert!(plist_system.contains("<key>UserName</key>"));
    }

    #[test]
    fn test_plist_path_user_mode() {
        // Should contain ~/Library/LaunchAgents
        // SAFETY: single-threaded test environment; no concurrent reads of HOME.
        unsafe { std::env::set_var("HOME", "/tmp/fake_home") };
        let path = plist_path(true).unwrap();
        assert!(path.to_str().unwrap().contains("LaunchAgents"));
        assert!(path.to_str().unwrap().contains("com.atlasip.service.plist"));
    }

    #[test]
    fn test_plist_path_system_mode() {
        let path = plist_path(false).unwrap();
        assert!(path.to_str().unwrap().contains("LaunchDaemons"));
    }
}
