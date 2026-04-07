# AtlasIP Architecture Overview

## 1. Introduction
AtlasIP is a cross-platform OSINT analysis tool built with a modern, lightweight, and maintainable architecture. The application combines a React + Vite frontend with a Rust + Tauri backend, providing a fast, secure, and native desktop experience on macOS and Windows.

This document provides a high-level overview of the project structure, technologies, and internal communication patterns.

## 2. High-Level Architecture

Frontend (React + Vite + TypeScript)
↓ Tauri IPC
Backend (Rust + Tauri)
↓
Operating System (macOS / Windows)

## 3. Frontend (React + Vite)

### 3.1 Technologies
- React (UI framework)
- Vite (bundler and dev server)
- TypeScript (type safety)
- Shadcn/UI + Radix Primitives (UI components)
- Design Tokens (colors, spacing, radius, shadows, transitions)

### 3.2 Responsibilities
The frontend is responsible for:
- Rendering the user interface
- Managing user interactions
- Displaying results and logs
- Providing the Settings panel
- Communicating with the backend through Tauri commands
- Applying consistent styling through design tokens

### 3.3 Directory Structure (simplified)

ui/
  src/
    components/
    pages/
    hooks/
    lib/
    styles/
    tauri/
  public/
  index.html
  vite.config.ts

## 4. Backend (Rust + Tauri)

### 4.1 Technologies
- Rust (core logic)
- Tauri (desktop runtime, IPC, bundling)
- Serde (serialization)
- TOML (configuration format)

### 4.2 Responsibilities
The backend handles:
- Configuration loading, validation, and persistence
- Network and proxy logic
- Timeout and retry strategies
- File system access
- Secure communication with the frontend
- Packaging into DMG (macOS) and MSI (Windows)

### 4.3 Directory Structure (simplified)

ui/src-tauri/
  src/
    config.rs
    network.rs
    proxy.rs
    timeouts.rs
    commands.rs
    main.rs
  tauri.conf.json
  Cargo.toml

## 5. Configuration System (TOML)

AtlasIP uses a strongly typed TOML configuration file stored in:

- macOS: ~/Library/Application Support/atlasip/config.toml
- Windows: %APPDATA%\\atlasip\\config.toml

Features:
- Default values
- Schema validation
- Atomic read/write
- Strong typing via Rust structs
- Synchronized with the Settings UI

## 6. Frontend ↔ Backend Communication

Communication uses Tauri Commands, which expose Rust functions to the frontend.

Example flow:
1. User updates a setting in the UI
2. React triggers a Tauri command
3. Rust updates the TOML config
4. Rust returns a success/error response
5. UI updates accordingly

This ensures a clean separation of concerns.

## 7. Build & Packaging

### macOS
- Bundled as a signed or unsigned DMG
- Appears in Launchpad after first launch from /Applications

### Windows
- Bundled as an MSI installer
- Uses Tauri’s WiX-based bundler

Build commands are handled through scripts or `npm run tauri build`.

## 8. Testing

### Frontend
- Vitest
- Testing Library
- happy-dom
- Component tests
- Debounce & validation tests

### Backend
- Rust unit tests
- Config round-trip tests
- Error handling tests

## 9. Project Goals
- Lightweight, fast, and secure OSINT tool
- Professional UI/UX
- Cross-platform parity
- Clean architecture and maintainable codebase
- Open-source friendly

## 10. Future Improvements
- Internationalization (EN/FR)
- Advanced Settings panel
- Documentation integration inside the app
- Automated release scripts
- Plugin system (long-term)
