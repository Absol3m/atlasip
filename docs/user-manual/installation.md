# Installation

## Overview
AtlasIP provides native installers for macOS and Windows. No external dependencies are required, and the application runs fully locally without network‑based activation or cloud services. This document describes how to install, update, and uninstall AtlasIP on supported platforms.

---

## System Requirements

### macOS
- macOS 12 or later
- Apple Silicon (M1, M2, M3)
- 150 MB of free disk space

### Windows
- Windows 10 or Windows 11 (64‑bit)
- 150 MB of free disk space

---

## Downloading AtlasIP
The latest release packages are available on the project’s release page:

- macOS: `.dmg` installer
- Windows: `.msi` installer

Each release includes versioned artifacts and a changelog entry.

---

## macOS Installation

### 1. Open the Disk Image
Double‑click the downloaded `.dmg` file to mount the disk image.

### 2. Install the Application
Drag **AtlasIP.app** into the **Applications** folder.

### 3. First Launch
macOS may display a security prompt because the application is unsigned or self‑signed.  
To proceed:

1. Open **System Settings**
2. Navigate to **Privacy & Security**
3. Scroll to the bottom and allow the application to run

After approval, AtlasIP can be launched normally from Launchpad or Finder.

---

## Windows Installation

### 1. Run the Installer
Double‑click the downloaded `.msi` file.

### 2. Follow the Setup Wizard
The installer guides the user through:

- Installation directory selection
- Shortcut creation
- Optional confirmation prompts

### 3. Launch the Application
After installation, AtlasIP is available from the Start Menu.

---

## Updating AtlasIP

### macOS
1. Download the latest `.dmg`
2. Replace the existing **AtlasIP.app** in the Applications folder

### Windows
1. Download the latest `.msi`
2. Run the installer; it will update the existing installation

Configuration files are preserved automatically on both platforms.

---

## Uninstallation

### macOS
Delete the application from:

/Applications/AtlasIP.app


Configuration files remain in:

~/Library/Application Support/atlasip/

These can be removed manually if desired.

### Windows
Use **Add or Remove Programs** to uninstall AtlasIP.  
Configuration files remain in:

%APPDATA%\atlasip\

They can be deleted manually if a full cleanup is required.

---

## Verifying Installation
After installation, AtlasIP should:

- Launch without errors
- Create a configuration file on first run
- Display the main interface with the IP analysis panel

If issues occur, refer to the **Troubleshooting** section.

---

## Next Steps
- Review the **Getting Started** guide for first‑time usage
- Explore the **Configuration** section to customize behavior
- Read the **UI Overview** to understand the interface layout