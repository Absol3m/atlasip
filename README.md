<p align="center">
  <img src="img/logo.png" alt="AtlasIP Logo" width="180">
</p>

<h1 align="center">AtlasIP</h1>
<p align="center">Modern, fast, and open‑source IP OSINT toolkit</p>

---

## 🔍 Overview

AtlasIP is a cross‑platform OSINT analysis tool designed for fast, reliable, and privacy‑respecting IP investigations.  
It provides a modern desktop application powered by **React**, **Tauri**, and **Rust**, with a clean UI and a fully local enrichment pipeline.

AtlasIP performs structured IP lookups using:
- Reverse DNS  
- WHOIS  
- ASN extraction  
- Geolocation (local database support planned)  

No telemetry, no cloud services, no external tracking.

---

## ✨ Features

### 🖥️ Desktop Application
- Native macOS and Windows builds  
- Modern UI with design tokens  
- Responsive layout  
- Accessible and keyboard‑friendly  

### 🔧 Enrichment Engine
- Reverse DNS  
- WHOIS (port 43)  
- ASN extraction  
- IPv4 & IPv6 support  
- Configurable timeouts and retries  
- Optional proxy support  

### ⚙️ Configuration
- Strongly typed TOML configuration  
- Automatic validation and default restoration  
- UI settings panel with:
  - Network settings  
  - UI preferences  
  - Logging level  

### 🔐 Privacy & Security
- Fully local processing  
- No telemetry  
- No remote API calls  
- No data collection  

---

## 📦 Installation

AtlasIP provides native installers for macOS and Windows.

### macOS
1. Download the `.dmg` from the latest release  
2. Drag **AtlasIP.app** into **Applications**  
3. Approve the app in **System Settings → Privacy & Security** if required  

### Windows
1. Download the `.msi` installer  
2. Run the setup wizard  
3. Launch AtlasIP from the Start Menu  

Full installation details are available in:

docs/user-manual/installation.md

---

## 🚀 Getting Started

1. Launch AtlasIP  
2. Enter an IPv4 or IPv6 address  
3. Click **Analyze**  
4. Review structured results in the results panel  
5. Adjust settings if needed (timeouts, proxy, theme)

Full guide:

docs/user-manual/getting-started.md

---

## 🧩 Architecture

AtlasIP uses a hybrid architecture:

React (UI)
↓
Tauri (Desktop Shell)
↓
Rust (Backend Enrichment Engine)

### Frontend
- React + Vite  
- Design tokens  
- Modern component architecture  

### Backend
- Rust  
- Async networking  
- WHOIS + DNS + ASN modules  
- Strong configuration schema  

### Packaging
- macOS: `.dmg`  
- Windows: `.msi`  
- Reproducible builds  

More details:

docs/developer/build-instructions.md
docs/developer/config-schema.md

---

## 📚 Documentation

AtlasIP includes complete documentation:

### User Manual
- Getting Started  
- Installation  
- Configuration  
- UI Overview  
- Troubleshooting  

### Developer Documentation
- Build instructions  
- Release process  
- Configuration schema  

Documentation root:

docs/

---

## 🛠️ Development

### Prerequisites
- Node.js  
- Rust (stable)  
- Tauri CLI  

### Development build

cd ui
npm install
npm run tauri dev

### Production build

cd ui
npm run build
npm run tauri build

Full instructions:

docs/developer/build-instructions.md

---

## 🗺️ Roadmap

Planned features include:

- Local geolocation database  
- Advanced enrichment modules  
- Plugin system  
- Automated updates  
- Browser extension integration  
- Linux support  

Roadmap evolves with each release.

---

## 📜 Changelog

All release notes are available in:

docs/changelog/

Latest major release:

docs/changelog/0.3.0.md

---

## 🤝 Contributing

Contributions are welcome.  
Please follow the guidelines in:

CONTRIBUTING.md

Bug reports and feature requests can be submitted through the issue tracker.

---

## 🔒 License

AtlasIP is licensed under the Apache License 2.0.  
See the `LICENSE` and `NOTICE` files for details.

---

<p align="center">
  Built with ❤️ for OSINT analysts, SOC teams, CERTs, and researchers.
</p>
