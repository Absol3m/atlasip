# Build Instructions

## Overview
This document describes how to build AtlasIP from source on macOS and Windows. The project uses a hybrid architecture combining React (frontend), Tauri (desktop shell), and Rust (backend). All components must be built together to produce a functional application bundle.

---

## Prerequisites

### Common Requirements
- Git
- Node.js (LTS recommended)
- npm or yarn
- Rust toolchain (stable)
- Tauri CLI

### Install Rust
Rust can be installed using `rustup`:

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Verify installation:

rustc --version
cargo --version


### Install Tauri CLI

cargo install tauri-cli

### Install Node.js Dependencies
From the `ui/` directory:

npm install

or

yarn install

---

## Project Structure

atlasip/
ui/                 # React + Tauri frontend
src/              # UI source code
src-tauri/        # Rust backend and Tauri configuration
docs/               # Documentation
scripts/            # Build and tooling scripts

The build process is executed from the `ui/` directory.

---

## Development Build

### Start the Development Server
From the `ui/` directory:

npm run tauri dev

This command:
- Starts the Vite development server
- Compiles the Rust backend in debug mode
- Launches the Tauri application with hot‑reload enabled

---

## Production Build

### Step 1 — Build the Frontend
From the `ui/` directory:

npm run build

This generates optimized assets in:

ui/dist/

### Step 2 — Build the Desktop Application
Still in the `ui/` directory:

npm run tauri build

This command:
- Builds the Rust backend in release mode
- Packages the application for the current platform
- Produces platform‑specific artifacts

---

## Build Output Locations

### macOS
Artifacts are generated in:

ui/src-tauri/target/release/bundle/dmg/
ui/src-tauri/target/release/bundle/macos/

Typical outputs:
- `AtlasIP.app`
- `AtlasIP_<version>_aarch64.dmg`

### Windows
Artifacts are generated in:

ui/src-tauri/target/release/bundle/msi/
ui/src-tauri/target/release/bundle/windows/

Typical outputs:
- `AtlasIP_<version>_x64_en-US.msi`
- `AtlasIP.exe`

---

## Cleaning the Build

### Clean Frontend

rm -rf ui/dist/

### Clean Rust Backend

cargo clean

### Full Clean

rm -rf ui/dist/
cargo clean

---

## Environment Variables

### `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD`
Used for signing production builds.  
If not provided, Tauri generates unsigned builds.

### `RUST_LOG`
Controls backend logging during development.

Example:

RUST_LOG=debug npm run tauri dev

---

## Platform Notes

### macOS
- Apple Silicon is required for native builds
- Unsigned builds trigger Gatekeeper warnings
- Codesigning is optional but recommended for distribution

### Windows
- MSI installers require WiX Toolset if building from source
- SmartScreen may warn about unsigned binaries

---

## Troubleshooting

### Missing Rust Target
If building for a different architecture:

rustup target add aarch64-apple-darwin
rustup target add x86_64-pc-windows-msvc

### Node Modules Not Installed

npm install

### Tauri CLI Not Found

cargo install tauri-cli

---

## Next Steps
- Review the **Release Process** for packaging and versioning
- Consult the **Config Schema** for backend configuration details