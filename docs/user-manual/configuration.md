# Configuration

## Overview
AtlasIP uses a local, strongly typed TOML configuration file to store user preferences and network settings. The configuration is created automatically on first launch and updated whenever settings are changed through the application interface. This document describes the structure, location, and behavior of the configuration system.

---

## Configuration File Location

### macOS

~/Library/Application Support/atlasip/config.toml

### Windows

%APPDATA%\atlasip\config.toml

The file is generated automatically if it does not exist.

---

## Editing the Configuration
AtlasIP provides a graphical Settings panel that updates the configuration file automatically.  
Manual editing is possible but not required. When edited manually:

- Invalid values may be corrected or reset by the application
- Missing fields are restored with default values
- Unknown fields are ignored

The application validates the configuration on startup.

---

## Configuration Structure
The configuration file is organized into logical sections.  
A typical structure is shown below:

[network]
timeout = 5000
retry_count = 2
proxy_enabled = false
proxy_address = ""
proxy_port = 0

[ui]
theme = "system"
animations = true

[logging]
level = "info"

The exact schema may evolve between versions.

---

## Network Settings

### `timeout`
Defines the maximum duration (in milliseconds) for network operations.

Example:

timeout = 5000

### `retry_count`
Number of retry attempts for failed requests.

Example:

retry_count = 2

### `proxy_enabled`
Enables or disables proxy usage.

Example:

proxy_enabled = true

### `proxy_address` and `proxy_port`
Specifies the proxy server address and port.

Example:

proxy_address = "127.0.0.1"
proxy_port = 8080


If `proxy_enabled` is false, these values are ignored.

---

## UI Settings

### `theme`
Controls the application theme.

Possible values:
- `"light"`
- `"dark"`
- `"system"`

### `animations`
Enables or disables UI animations.

Example:

animations = true

---

## Logging Settings

### `level`
Controls the verbosity of internal logs.

Possible values:
- `"error"`
- `"warn"`
- `"info"`
- `"debug"`

Example:

level = "info"

Logs are stored locally and are not transmitted externally.

---

## Default Values
If the configuration file is missing or corrupted, AtlasIP restores default values:

timeout = 5000
retry_count = 2
proxy_enabled = false
theme = "system"
animations = true
level = "info"

Defaults may vary depending on the version.

---

## Resetting the Configuration
To reset AtlasIP to its default settings:

1. Close the application.
2. Delete the configuration file from its platform‑specific location.
3. Relaunch AtlasIP.

A fresh configuration file will be created automatically.

---

## Version Compatibility
AtlasIP ensures backward compatibility between minor versions.  
When upgrading:

- New fields are added automatically
- Deprecated fields are ignored
- Existing values are preserved whenever possible

Major version changes may introduce schema updates.

---

## Next Steps
- Review the **UI Overview** to understand the interface layout
- Consult **Troubleshooting** for common configuration issues