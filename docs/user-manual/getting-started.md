# Getting Started

## Overview
AtlasIP is a cross‑platform OSINT analysis tool designed to simplify IP investigation workflows. It provides a fast, lightweight, and privacy‑respecting environment powered by a modern React + Tauri + Rust architecture. This guide introduces the basic concepts, installation process, and first steps to begin using AtlasIP.

AtlasIP runs on macOS and Windows and requires no external dependencies or cloud services.

---

## Key Features
- Cross‑platform desktop application (macOS and Windows)
- Fast native performance powered by Rust and Tauri
- Modern and accessible UI built with React and design tokens
- Configurable network timeouts and proxy settings
- Local configuration stored in a typed TOML file
- No telemetry, no external tracking, no remote API usage
- Fully open‑source

---

## Supported Platforms
AtlasIP provides native builds for:

- macOS (Apple Silicon)
- Windows 10/11 (x64)

Linux support may be added in future releases.

---

## Installation

### macOS
1. Download the `.dmg` file from the latest release.
2. Open the disk image.
3. Drag **AtlasIP.app** into the **Applications** folder.
4. Launch the application from Launchpad or Finder.
5. On first launch, macOS may require confirmation because the app is unsigned or self‑signed.

### Windows
1. Download the `.msi` installer from the latest release.
2. Run the installer and follow the setup steps.
3. Launch AtlasIP from the Start Menu.

---

## First Launch
When AtlasIP starts for the first time:

1. A default configuration file is created automatically.
2. The application loads the default settings for timeouts, proxy behavior, and UI preferences.
3. The main interface displays the IP analysis panel.

No internet connection is required to run the application itself.

---

## Basic Workflow

### 1. Enter an IP Address
Use the main input field to enter a valid IPv4 or IPv6 address.

### 2. Trigger Analysis
Press **Analyze** to start the enrichment process.  
AtlasIP performs local validation and applies the configured network settings.

### 3. Review Results
Results are displayed in a structured, readable format.  
Depending on the configuration, AtlasIP may perform:

- Reverse DNS lookups  
- WHOIS queries  
- ASN extraction  
- Geolocation lookups (local database if available)

### 4. Adjust Settings (Optional)
Open the **Settings** panel to configure:

- Network timeouts  
- Proxy settings  
- Retry behavior  
- UI preferences  

Changes are applied immediately and persisted to the configuration file.

---

## Configuration File Location

AtlasIP stores its configuration in a TOML file:

### macOS

~/Library/Application Support/atlasip/config.toml

### Windows

%APPDATA%\atlasip\config.toml

The file is generated automatically and updated whenever settings are changed through the UI.

---

## Updating AtlasIP
To update the application:

1. Download the latest release for your platform.
2. Replace the existing installation (macOS) or run the new installer (Windows).
3. The configuration file is preserved automatically.

Automatic updates may be added in future versions.

---

## Getting Help
For documentation, troubleshooting, and development information, refer to:

docs/user-manual/
docs/developer/

Issues and feature requests can be submitted through the project’s issue tracker.

---

## Next Steps
- Review the **Installation** guide for platform‑specific details.
- Explore the **Configuration** section to customize behavior.
- Read the **UI Overview** to understand the interface layout.
- Consult **Troubleshooting** for common issues and solutions.