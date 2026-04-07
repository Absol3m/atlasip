# Contributing to AtlasIP

Thank you for your interest in contributing to AtlasIP!  
This project welcomes contributions of all kinds — code, documentation, design, testing, and feedback.

The goal is to maintain a clean, stable, and professional codebase while fostering a friendly and productive open‑source environment.

---

## 🧭 How to Contribute

There are several ways to contribute:

- Report bugs  
- Suggest new features  
- Improve documentation  
- Submit pull requests  
- Help with testing  
- Participate in discussions  

All contributions are appreciated.

---

## 🐛 Reporting Issues

Before opening an issue:

1. Check if the problem already exists in the issue tracker  
2. Provide clear steps to reproduce the issue  
3. Include platform details (macOS/Windows)  
4. Include AtlasIP version  
5. Attach logs if relevant  

Good bug reports help maintainers fix problems quickly.

---

## 💡 Suggesting Features

Feature requests are welcome.  
Please include:

- A clear description of the feature  
- The problem it solves  
- Possible alternatives  
- Any relevant examples or references  

Feature discussions help shape the roadmap.

---

## 🔧 Development Setup

### Prerequisites
- Node.js (LTS recommended)  
- Rust (stable)  
- Tauri CLI  
- Git  

### Install dependencies

cd ui
npm install

### Start development environment

npm run tauri dev

### Build production artifacts

npm run build
npm run tauri build

More details are available in:

docs/developer/build-instructions.md

---

## 🧹 Code Style & Guidelines

### General Principles
- Keep the code clean and readable  
- Follow existing patterns and architecture  
- Avoid unnecessary dependencies  
- Write small, focused commits  
- Document changes when needed  

### Rust
- Use idiomatic Rust  
- Prefer explicit types when clarity improves  
- Keep modules small and cohesive  

### React / TypeScript
- Use functional components  
- Keep components small and composable  
- Use design tokens for styling  
- Avoid inline styles unless necessary  

### Tauri
- Do not introduce blocking operations in the main thread  
- Keep commands minimal and well‑defined  

---

## 🧪 Testing

Testing contributions are welcome.  
Areas that benefit from tests:

- Rust backend modules  
- Configuration schema  
- Network timeout and retry logic  
- UI behavior (manual testing)  

Automated UI testing may be added in future versions.

---

## 📦 Pull Requests

### Before submitting a PR:
1. Ensure the code builds successfully  
2. Run the application and test your changes  
3. Update documentation if needed  
4. Keep commits focused and meaningful  
5. Reference related issues  

### PR Guidelines
- Use clear titles and descriptions  
- Keep changes scoped and minimal  
- Avoid mixing refactors with feature changes  
- Be open to feedback and iteration  

Maintainers review PRs to ensure quality and consistency.

---

## 🗺️ Roadmap & Vision

AtlasIP aims to provide:

- A modern, reliable IP OSINT toolkit  
- A clean and accessible UI  
- A stable and extensible architecture  
- A fully local, privacy‑respecting workflow  

Contributions aligned with this vision are especially welcome.

---

## 🤝 Code of Conduct

All contributors are expected to follow the project’s Code of Conduct:

CODE_OF_CONDUCT.md

Respectful and constructive communication is essential.

---

## 📄 License

By contributing, you agree that your contributions will be licensed under:

LICENSE

---

Thank you for helping improve AtlasIP!
