#!/usr/bin/env bash
# ── AtlasIP Linux service installer (spec §8) ────────────────────────────────
# Installs or uninstalls AtlasIP as a systemd service.
#
# Usage:
#   sudo ./install-service-linux.sh [install | uninstall]
#
# The script wraps `atlasip install-service-linux` / `atlasip uninstall-service-linux`
# but can also be used standalone if the atlasip binary is in PATH or is
# extracted from an AppImage next to this script.

set -euo pipefail

# ── Colours ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
ok()   { echo -e "${GREEN}[OK]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
err()  { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

# ── Root check ────────────────────────────────────────────────────────────────
[[ $EUID -ne 0 ]] && err "This script must be run as root (sudo $0 $*)"

# ── Locate the atlasip binary ─────────────────────────────────────────────────
ATLASIP_BIN="${ATLASIP_BIN:-}"
if [[ -z "$ATLASIP_BIN" ]]; then
    if command -v atlasip &>/dev/null; then
        ATLASIP_BIN="$(command -v atlasip)"
    elif [[ -f "$(dirname "$0")/../target/release/atlasip" ]]; then
        ATLASIP_BIN="$(realpath "$(dirname "$0")/../target/release/atlasip")"
    else
        err "atlasip binary not found. Set ATLASIP_BIN=/path/to/atlasip or add it to PATH."
    fi
fi

ok "Using binary: $ATLASIP_BIN"

ACTION="${1:-install}"

case "$ACTION" in
# ── Install ───────────────────────────────────────────────────────────────────
install)
    UNIT=/etc/systemd/system/atlasip.service
    cat > "$UNIT" <<UNIT
[Unit]
Description=AtlasIP headless backend service
Documentation=https://github.com/Absol3m/atlasip
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${ATLASIP_BIN} serve
Restart=always
RestartSec=5
Environment=RUST_LOG=atlasip=info
StandardOutput=journal
StandardError=journal
SyslogIdentifier=atlasip
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only

[Install]
WantedBy=multi-user.target
UNIT

    systemctl daemon-reload
    systemctl enable --now atlasip

    ok "AtlasIP service installed and started."
    echo "   Status  : systemctl status atlasip"
    echo "   Logs    : journalctl -u atlasip -f"
    echo "   Stop    : systemctl stop atlasip"
    ;;

# ── Uninstall ─────────────────────────────────────────────────────────────────
uninstall)
    systemctl disable --now atlasip 2>/dev/null || true
    rm -f /etc/systemd/system/atlasip.service
    systemctl daemon-reload
    ok "AtlasIP service uninstalled."
    ;;

*)
    err "Unknown action '$ACTION'. Use: install | uninstall"
    ;;
esac
