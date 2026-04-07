# Troubleshooting

## Overview
This document lists common issues encountered when using AtlasIP and provides recommended solutions. Most problems are related to configuration, network conditions, or platform‑specific restrictions. If an issue persists, consult the project’s issue tracker for additional support.

---

## Application Does Not Start

### macOS: “App cannot be opened”
macOS may block the application because it is unsigned or self‑signed.

**Solution:**
1. Open **System Settings**
2. Go to **Privacy & Security**
3. Scroll to the bottom
4. Allow AtlasIP to run

### Windows: Application blocked by SmartScreen
Windows may warn about unknown publishers.

**Solution:**
- Select **More info**
- Choose **Run anyway**

---

## No Results Returned During Analysis

### Invalid IP Address
The input may not be a valid IPv4 or IPv6 address.

**Solution:**
- Verify the format
- Ensure no extra characters or spaces are included

### Network Timeout
The configured timeout may be too low for the current network conditions.

**Solution:**
- Increase the timeout value in **Settings → Network**

### Proxy Misconfiguration
If proxy mode is enabled but the proxy is unreachable, all requests will fail.

**Solution:**
- Disable proxy mode
- Or correct the proxy address and port

---

## Slow Performance

### High Timeout Values
Large timeout settings can delay error detection.

**Solution:**
- Reduce the timeout value in **Settings → Network**

### Unstable Network
Network instability may cause retries and delays.

**Solution:**
- Check the network connection
- Reduce retry count if necessary

---

## Configuration File Issues

### Corrupted Configuration File
If the configuration file contains invalid values, AtlasIP may reset fields or fail to load settings.

**Solution:**
1. Close the application
2. Delete the configuration file:

**macOS**

~/Library/Application Support/atlasip/config.toml

**Windows**

%APPDATA%\atlasip\config.toml

3. Relaunch AtlasIP to generate a fresh configuration

---

## UI Not Updating

### Stale State
The UI may not refresh if the application was left running for an extended period.

**Solution:**
- Restart the application

### Theme Not Applying
If the theme does not update:

**Solution:**
- Ensure the theme is set to `"light"`, `"dark"`, or `"system"`
- Restart the application if the system theme changed

---

## Proxy Not Working

### Incorrect Address or Port
Proxy settings must be valid for the system to route traffic correctly.

**Solution:**
- Verify the proxy address
- Verify the port number
- Ensure the proxy server is reachable

### Proxy Enabled by Mistake
If proxy mode is enabled unintentionally, all requests may fail.

**Solution:**
- Disable proxy mode in **Settings → Network**

---

## Logs Not Updating

### Logging Level Too Restrictive
If the log level is set to `"error"`, informational messages will not appear.

**Solution:**
- Set the log level to `"info"` or `"debug"`

---

## Application Crashes or Freezes

### Outdated Version
Older versions may contain bugs fixed in later releases.

**Solution:**
- Download the latest version from the release page

### Corrupted Installation
Rarely, installation files may be incomplete.

**Solution:**
- Reinstall the application

---

## Resetting AtlasIP
To fully reset the application:

1. Close AtlasIP  
2. Delete the configuration directory:

**macOS**

~/Library/Application Support/atlasip/

**Windows**

%APPDATA%\atlasip\

3. Reinstall the application (optional)

---

## Reporting Issues
If a problem persists:

- Include platform information (macOS/Windows)
- Include AtlasIP version
- Include steps to reproduce the issue
- Include relevant log entries if available

Issues can be submitted through the project’s issue tracker.