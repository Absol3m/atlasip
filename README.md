# AtlasIP

AtlasIP est un outil moderne d'analyse d'adresses IP, écrit en Rust.  
Il interroge d'abord les services **RDAP** (standard moderne), puis bascule automatiquement sur **WHOIS (port 43)** en fallback si nécessaire.

AtlasIP fournit :
- une **CLI** pour analyser des IP en local
- un **serveur HTTP local** pour être utilisé par une extension navigateur
- un **moteur RDAP** rapide et fiable
- un **fallback WHOIS** compatible avec les environnements d'entreprise

## 🚀 Fonctionnalités principales

- Analyse d'une IP ou d'un hostname
- RDAP-first (HTTPS)
- WHOIS fallback (TCP/43)
- Résolution DNS
- API HTTP locale (`127.0.0.1:8080`)
- Sortie JSON structurée
- Compatible VDI / proxys / environnements verrouillés

## 🛠️ Installation

### Prérequis
- Rust (via rustup)
- macOS, Linux ou Windows

### Compilation

```bash
cargo build --release

