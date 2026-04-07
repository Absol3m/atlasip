# Configuration Schema

## Overview
This document defines the internal configuration schema used by AtlasIP.  
The configuration is stored in a TOML file and loaded at application startup.  
The schema is strongly typed and validated by the Rust backend to ensure stability and predictable behavior.

The configuration file is created automatically on first launch and updated whenever settings are changed through the UI.

---

## File Location

### macOS

~/Library/Application Support/atlasip/config.toml

### Windows

%APPDATA%\atlasip\config.toml

---

## Schema Structure

The configuration is divided into three main sections:

- `[network]` — network behavior and timeouts  
- `[ui]` — user interface preferences  
- `[logging]` — log verbosity and debug settings  

A typical configuration file:

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

---

## Section: `[network]`

### `timeout`  
**Type:** integer  
**Unit:** milliseconds  
**Description:** Maximum duration allowed for network operations.  
**Default:** `5000`

### `retry_count`  
**Type:** integer  
**Description:** Number of retry attempts when a request fails.  
**Default:** `2`

### `proxy_enabled`  
**Type:** boolean  
**Description:** Enables or disables proxy routing.  
**Default:** `false`

### `proxy_address`  
**Type:** string  
**Description:** Proxy server hostname or IP address.  
**Default:** `""`

### `proxy_port`  
**Type:** integer  
**Description:** Proxy server port.  
**Default:** `0`

**Validation Rules:**
- If `proxy_enabled = false`, proxy fields are ignored.
- If `proxy_enabled = true`, both `proxy_address` and `proxy_port` must be valid.

---

## Section: `[ui]`

### `theme`  
**Type:** string  
**Allowed values:**  
- `"light"`  
- `"dark"`  
- `"system"`  
**Description:** Controls the application theme.  
**Default:** `"system"`

### `animations`  
**Type:** boolean  
**Description:** Enables or disables UI animations.  
**Default:** `true`

---

## Section: `[logging]`

### `level`  
**Type:** string  
**Allowed values:**  
- `"error"`  
- `"warn"`  
- `"info"`  
- `"debug"`  
**Description:** Controls the verbosity of internal logs.  
**Default:** `"info"`

---

## Validation Rules

### Missing Fields
If a field is missing, the default value is inserted automatically.

### Unknown Fields
Unknown fields are ignored to maintain forward compatibility.

### Invalid Values
If a field contains an invalid value:
- The application logs a warning
- The field is replaced with its default value

### Type Enforcement
Each field is strictly typed:
- Strings must be valid UTF‑8
- Integers must be non‑negative
- Booleans must be `true` or `false`

---

## Schema Evolution

### Backward Compatibility
Minor versions maintain backward compatibility:
- New fields are added with defaults
- Deprecated fields are ignored

### Major Changes
Major versions may introduce schema changes requiring migration logic.

### Migration Strategy
When schema changes occur:
1. The application loads the existing configuration
2. Missing fields are added
3. Deprecated fields are removed
4. The updated configuration is written back to disk

---

## Example: Fully Expanded Configuration

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

---

## Testing the Schema

### Unit Tests
The Rust backend includes tests for:
- Default value generation
- Invalid value handling
- Schema evolution
- Serialization and deserialization

### Manual Testing
Developers can modify the configuration file manually and restart the application to verify behavior.

---

## Related Documentation
- `docs/user-manual/configuration.md`
- `docs/developer/build-instructions.md`
- `docs/developer/release-process.md`