# UI Overview

## Introduction
AtlasIP provides a clean, modern, and efficient user interface designed to support fast OSINT workflows. The layout is organized into functional sections that guide the user from input to analysis results with minimal friction. This document describes the structure and behavior of the main interface components.

---

## Main Layout
The application interface is divided into the following areas:

- **Header**  
  Contains navigation elements and access to global actions.

- **Input Panel**  
  Allows entering IP addresses and triggering analysis.

- **Results Panel**  
  Displays structured information returned by the analysis process.

- **Sidebar (optional)**  
  Provides access to settings, logs, and additional tools.

- **Footer**  
  Displays version information and optional status indicators.

The layout adapts to different window sizes and respects system theme preferences.

---

## Header

### Components
- Application title
- Navigation buttons (if enabled)
- Access to the Settings panel
- Theme toggle (optional)
- About dialog (optional)

The header remains visible at all times to ensure quick access to global actions.

---

## Input Panel

### IP Input Field
The main input field accepts IPv4 and IPv6 addresses.  
Features include:

- Real‑time validation
- Error highlighting for invalid formats
- Keyboard shortcuts for quick submission

### Analyze Button
Triggers the enrichment process.  
The button is disabled when the input is invalid.

### Input Behavior
- Pressing **Enter** starts the analysis
- Invalid input displays a contextual error message
- Previous input is preserved between sessions

---

## Results Panel

### Structure
Results are displayed in a structured, readable format.  
Typical sections include:

- Reverse DNS
- WHOIS information
- ASN details
- Geolocation (if available)
- Additional metadata

Each section may include:

- Key‑value pairs
- Collapsible groups
- Status indicators
- Error messages when data is unavailable

### Loading States
During analysis:

- A loading indicator is displayed
- The panel is temporarily disabled
- Partial results may appear progressively depending on the backend behavior

### Error Handling
If an error occurs:

- A clear message is displayed
- The user is informed of the cause (timeout, invalid IP, network issue)
- Suggested actions may be provided

---

## Settings Panel

### Access
The Settings panel is accessible from the header.

### Sections
The panel includes:

- **Network Settings**  
  Timeout, retry count, proxy configuration

- **UI Settings**  
  Theme selection, animation preferences

- **Logging Settings**  
  Log level configuration

### Behavior
- Changes are applied immediately
- All settings are persisted to the configuration file
- Invalid values are corrected automatically

---

## Sidebar (Optional)
Depending on the version, the sidebar may include:

- Quick access to logs
- Navigation to additional tools
- Links to documentation

The sidebar can be collapsed to maximize workspace.

---

## Footer
The footer displays:

- Application version
- Build information
- Optional status indicators (proxy enabled, debug mode, etc.)

---

## Keyboard Shortcuts
AtlasIP supports basic shortcuts:

- **Enter** — Start analysis
- **Ctrl/Cmd + ,** — Open Settings
- **Ctrl/Cmd + R** — Refresh results (if enabled)

Additional shortcuts may be added in future versions.

---

## Theme and Appearance
AtlasIP supports:

- Light mode
- Dark mode
- System theme detection

UI elements follow a consistent design system based on:

- Design tokens
- Accessible color contrast
- Responsive layout rules

---

## Responsiveness
The interface adapts to:

- Small window sizes
- High‑DPI displays
- System font scaling

Panels reorganize automatically to maintain readability.

---

## Next Steps
- Review the **Configuration** guide to customize behavior
- Consult **Troubleshooting** for common UI issues
