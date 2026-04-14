# ── AtlasIP Windows service installer (spec §6) ──────────────────────────────
# Installs or uninstalls AtlasIP as a Windows service.
#
# Usage (run PowerShell as Administrator):
#   .\install-service-windows.ps1 -Action install   [-BinPath "C:\path\to\atlasip.exe"]
#   .\install-service-windows.ps1 -Action uninstall
#
# The script wraps the native `atlasip.exe install-service` /
# `atlasip.exe uninstall-service` commands but provides a friendlier interface
# for MSI installers and automation.

param(
    [Parameter(Mandatory = $false)]
    [ValidateSet("install", "uninstall")]
    [string]$Action = "install",

    [Parameter(Mandatory = $false)]
    [string]$BinPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Colour helpers ────────────────────────────────────────────────────────────
function Write-Ok   { param($msg) Write-Host "[OK]    $msg" -ForegroundColor Green  }
function Write-Warn { param($msg) Write-Host "[WARN]  $msg" -ForegroundColor Yellow }
function Write-Err  { param($msg) Write-Host "[ERROR] $msg" -ForegroundColor Red    }

# ── Admin check ───────────────────────────────────────────────────────────────
$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Err "This script must be run as Administrator."
    exit 1
}

# ── Locate atlasip.exe ────────────────────────────────────────────────────────
if ([string]::IsNullOrEmpty($BinPath)) {
    # 1. Environment variable
    if ($env:ATLASIP_BIN -and (Test-Path $env:ATLASIP_BIN)) {
        $BinPath = $env:ATLASIP_BIN
    }
    # 2. Cargo release build (dev workflow)
    elseif (Test-Path "$PSScriptRoot\..\target\release\atlasip.exe") {
        $BinPath = Resolve-Path "$PSScriptRoot\..\target\release\atlasip.exe"
    }
    # 3. Program Files
    elseif (Test-Path "$env:ProgramFiles\AtlasIP\atlasip.exe") {
        $BinPath = "$env:ProgramFiles\AtlasIP\atlasip.exe"
    }
    else {
        Write-Err "atlasip.exe not found. Pass -BinPath or set `$env:ATLASIP_BIN."
        exit 1
    }
}

Write-Ok "Using binary: $BinPath"

$ServiceName    = "AtlasIPService"
$ServiceDisplay = "AtlasIP Service"
$ServiceDesc    = "AtlasIP headless backend — local HTTP API for IP analysis."

switch ($Action) {
    # ── Install ───────────────────────────────────────────────────────────────
    "install" {
        # Check if already installed.
        $existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
        if ($existing) {
            Write-Warn "Service '$ServiceName' already exists. Reinstalling..."
            Stop-Service  -Name $ServiceName -Force -ErrorAction SilentlyContinue
            sc.exe delete $ServiceName | Out-Null
            Start-Sleep -Seconds 2
        }

        # Register the service with the Windows SCM.
        # `run-service` tells the AtlasIP binary to enter the service dispatcher.
        $binPathWithArg = "`"$BinPath`" run-service"
        New-Service `
            -Name        $ServiceName `
            -DisplayName $ServiceDisplay `
            -BinaryPathName $binPathWithArg `
            -StartupType Automatic `
            -Description $ServiceDesc | Out-Null

        # Start the service immediately.
        Start-Service -Name $ServiceName

        Write-Ok "AtlasIP Service installed and started."
        Write-Host "   Query  : Get-Service $ServiceName"
        Write-Host "   Stop   : Stop-Service $ServiceName"
        Write-Host "   Logs   : Get-EventLog -LogName Application -Source AtlasIP -Newest 20"
    }

    # ── Uninstall ─────────────────────────────────────────────────────────────
    "uninstall" {
        $existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
        if (-not $existing) {
            Write-Warn "Service '$ServiceName' is not installed."
            exit 0
        }

        Stop-Service  -Name $ServiceName -Force -ErrorAction SilentlyContinue
        sc.exe delete $ServiceName | Out-Null

        Write-Ok "AtlasIP Service uninstalled."
    }
}
