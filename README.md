<p align="center">
  <img src="img/logo.png" alt="AtlasIP Logo" width="180">
</p>

<h1 align="center">AtlasIP</h1>
<p align="center">Modern, fast, and open‑source IP OSINT toolkit</p>

---

## 🔍 Overview

AtlasIP is a modern IP analysis tool written in Rust.  
It performs **RDAP-first lookups** (the modern standard) and automatically falls back to **WHOIS (port 43)** when needed.

It is designed as a clean, reliable, and extensible successor to legacy tools like IPNetInfo — but with a modern backend, a structured API, and a future-proof UI.

### AtlasIP provides:
- A **CLI** for local IP analysis  
- A **local HTTP server** for UI and browser extension integration  
- A fast and reliable **RDAP engine**  
- A robust **WHOIS fallback** compatible with enterprise environments  
- A modern **frontend** (in development)  
- A future **Tauri desktop app** and **browser extension**

---

## 🚀 Features

- Lookup of IPs and hostnames  
- RDAP-first (HTTPS)  
- WHOIS fallback (TCP/43)  
- DNS resolution (A/AAAA + PTR)  
- Local HTTP API (`127.0.0.1:8080`)  
- Structured JSON output  
- Proxy support (SOCKS4/5, HTTP, HTTPS)  
- Fully cross‑platform (macOS, Linux, Windows)  
- Designed for SOC, CERT, OSINT analysts, and researchers  

---

## 📦 Installation

### Prerequisites
- Rust (via `rustup`)
- macOS, Linux, or Windows

### Build from source

```bash
cargo build --release

The binary will be available in:
target/release/atlasip

---

