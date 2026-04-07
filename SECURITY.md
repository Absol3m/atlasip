# Security Policy

## Supported Versions

AtlasIP is currently in active development.  
Only the latest released version receives security updates.

| Version | Supported |
|--------|-----------|
| 0.3.x  | ✔️ Active |
| < 0.3  | ❌ No     |

---

## Reporting a Vulnerability

If you discover a security vulnerability, please **do not open a public issue**.

Instead, report it privately:

- Email: **security@absol3m.dev** (example — replace with your real address)
- Or contact the maintainer directly via GitHub private message

Please include:

- A clear description of the issue  
- Steps to reproduce  
- Affected version(s)  
- Your environment (OS, architecture)  
- Any proof‑of‑concept or logs (if available)

We will acknowledge your report within **72 hours** and provide a timeline for resolution.

---

## Disclosure Policy

- Valid reports will be investigated promptly  
- A fix will be prepared and tested before public disclosure  
- You will be credited in the release notes unless you prefer anonymity  
- Critical vulnerabilities may trigger an out‑of‑band hotfix release  

---

## Scope

This policy applies to:

- The AtlasIP desktop application (Tauri)  
- The Rust backend  
- The React frontend  
- Configuration handling  
- Build and packaging scripts  

Out of scope:

- Issues caused by modified or unofficial builds  
- Vulnerabilities in third‑party dependencies (report upstream instead)  
- Social engineering or phishing attempts  
- Physical access attacks  

---

## Best Practices for Contributors

To help maintain security:

- Avoid introducing new dependencies without discussion  
- Never commit secrets, tokens, or credentials  
- Validate all user input  
- Prefer Rust’s safe APIs over unsafe blocks  
- Follow the project’s coding guidelines  

---

## Thank You

We appreciate responsible disclosure and contributions that help keep AtlasIP secure.
