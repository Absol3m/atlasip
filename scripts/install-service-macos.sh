#!/usr/bin/env bash
# ── AtlasIP macOS service installer (spec §7) ────────────────────────────────
# Installs or uninstalls AtlasIP as a macOS launchd service.
#
# Usage:
#   ./install-service-macos.sh [install | uninstall] [--daemon]
#
# Options:
#   --daemon   Install as a system LaunchDaemon (requires sudo).
#              Default: install as a user LaunchAgent (no sudo needed).

set -euo pipefail

# ── Colours ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
ok()   { echo -e "${GREEN}[OK]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
err()  { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

# ── Defaults ──────────────────────────────────────────────────────────────────
ACTION="${1:-install}"
DAEMON_MODE=false
for arg in "$@"; do [[ "$arg" == "--daemon" ]] && DAEMON_MODE=true; done

LABEL="com.atlasip.service"

# ── Locate the atlasip binary ─────────────────────────────────────────────────
ATLASIP_BIN="${ATLASIP_BIN:-}"
if [[ -z "$ATLASIP_BIN" ]]; then
    if command -v atlasip &>/dev/null; then
        ATLASIP_BIN="$(command -v atlasip)"
    elif [[ -f "$(dirname "$0")/../target/release/atlasip" ]]; then
        ATLASIP_BIN="$(realpath "$(dirname "$0")/../target/release/atlasip")"
    elif [[ -f "/Applications/AtlasIP.app/Contents/MacOS/atlasip" ]]; then
        ATLASIP_BIN="/Applications/AtlasIP.app/Contents/MacOS/atlasip"
    else
        err "atlasip binary not found. Set ATLASIP_BIN=/path/to/atlasip or add it to PATH."
    fi
fi

ok "Using binary: $ATLASIP_BIN"

# ── Determine plist path ──────────────────────────────────────────────────────
if $DAEMON_MODE; then
    [[ $EUID -ne 0 ]] && err "Daemon mode requires root (sudo $0 $*)"
    PLIST_DIR="/Library/LaunchDaemons"
else
    PLIST_DIR="${HOME}/Library/LaunchAgents"
    mkdir -p "$PLIST_DIR"
fi

PLIST_PATH="${PLIST_DIR}/${LABEL}.plist"

case "$ACTION" in
# ── Install ───────────────────────────────────────────────────────────────────
install)
    # Build the plist.
    USER_KEY=""
    if ! $DAEMON_MODE; then
        USER_KEY=""
    else
        USER_KEY="    <key>UserName</key>
    <string>nobody</string>"
    fi

    cat > "$PLIST_PATH" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${LABEL}</string>

    <key>ProgramArguments</key>
    <array>
        <string>${ATLASIP_BIN}</string>
        <string>serve</string>
    </array>

    <key>KeepAlive</key>
    <true/>

    <key>RunAtLoad</key>
    <true/>

    <key>StandardOutPath</key>
    <string>/var/log/atlasip.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/atlasip.error.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>atlasip=info</string>
    </dict>
${USER_KEY}
    <key>ThrottleInterval</key>
    <integer>10</integer>
</dict>
</plist>
PLIST

    launchctl load -w "$PLIST_PATH"

    if $DAEMON_MODE; then
        ok "AtlasIP system daemon installed and loaded."
        echo "   Plist  : $PLIST_PATH"
        echo "   Status : launchctl list $LABEL"
    else
        ok "AtlasIP user agent installed and loaded."
        echo "   Plist  : $PLIST_PATH"
        echo "   Status : launchctl list $LABEL"
    fi
    ;;

# ── Uninstall ─────────────────────────────────────────────────────────────────
uninstall)
    if [[ -f "$PLIST_PATH" ]]; then
        launchctl unload -w "$PLIST_PATH" 2>/dev/null || true
        rm -f "$PLIST_PATH"
        ok "AtlasIP service uninstalled."
    else
        warn "No plist found at $PLIST_PATH — nothing to remove."
    fi
    ;;

*)
    err "Unknown action '$ACTION'. Use: install | uninstall"
    ;;
esac
