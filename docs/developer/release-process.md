# Release Process

## Overview
This document describes the release workflow for AtlasIP, including versioning rules, build steps, artifact validation, and publication guidelines. The goal is to ensure consistent, reproducible, and verifiable releases across macOS and Windows.

---

## Versioning

AtlasIP follows semantic versioning:

MAJOR.MINOR.PATCH

- **MAJOR** — Breaking changes or significant architectural updates  
- **MINOR** — New features, UI improvements, or non‑breaking enhancements  
- **PATCH** — Bug fixes, small adjustments, or documentation updates  

Pre‑release identifiers may be used:

0.3.0-alpha
0.3.0-beta
0.3.0-rc1

---

## Release Branching Model

### `main`
- Always stable
- Contains the latest production release

### Feature branches
- Used for development work
- Merged into `main` via pull requests

### Release branches (optional)
- Used for preparing major or minor releases
- Allow final testing before merging into `main`

---

## Pre‑Release Checklist

Before building a release:

1. Ensure all tests pass  
2. Update documentation:
   - User Manual
   - Developer documentation
   - Changelog
3. Verify configuration schema changes
4. Validate UI consistency and design tokens
5. Clean the workspace:

rm -rf ui/dist/
cargo clean
6. Update version numbers:
- `src-tauri/tauri.conf.json`
- `package.json`
- Changelog entry

---

## Building Release Artifacts

### 1. Build the Frontend
From the `ui/` directory:

npm run build

### 2. Build the Desktop Application

npm run tauri build

This produces platform‑specific artifacts in:

- macOS:  

ui/src-tauri/target/release/bundle/dmg/
ui/src-tauri/target/release/bundle/macos/

- Windows:  

ui/src-tauri/target/release/bundle/msi/
ui/src-tauri/target/release/bundle/windows/

---

## Artifact Validation

### macOS
Validate:

- `.dmg` mounts correctly
- `AtlasIP.app` launches without errors
- Configuration file is created on first launch
- UI loads correctly
- Network operations behave as expected

### Windows
Validate:

- `.msi` installs without warnings (SmartScreen may appear for unsigned builds)
- Application launches from Start Menu
- Configuration file is created
- Analysis workflow functions correctly

---

## Release Notes

Each release must include:

- Summary of changes
- New features
- Fixes
- Breaking changes (if any)
- Known issues
- Upgrade notes (if required)

Release notes are stored in:

docs/changelog/

---

## Publishing a Release

1. Create a new tag:

git tag vX.Y.Z
git push origin vX.Y.Z

2. Create a release entry:
- Upload `.dmg` and `.msi` artifacts
- Attach release notes
- Mark as pre‑release if applicable

3. Verify that:
- Artifacts download correctly
- Checksums match (if provided)
- Documentation links are valid

---

## Post‑Release Tasks

- Update roadmap
- Close completed issues
- Open follow‑up tasks for next version
- Announce the release (if applicable)

---

## Hotfix Process

For urgent fixes:

1. Create a hotfix branch from `main`
2. Apply the fix
3. Update the patch version
4. Build and validate artifacts
5. Publish a patch release
6. Merge back into `main`

---

## Automation (Future Work)

Potential improvements:

- Automated version bumping
- CI‑based build pipelines
- Automated artifact signing
- Automated changelog generation