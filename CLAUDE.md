# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A cross-platform Rust CLI that manages browser extension force-install policies and privacy controls for Chrome, Firefox, and Edge across Windows, macOS, and Linux. The tool reads YAML config files, applies OS-specific policies (Windows registry, macOS plists, Linux JSON), and maintains state for idempotent operations.

## Build and Test Commands

```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run tests (some require sudo/admin privileges)
cargo test

# Run tests with privileges (Linux/macOS)
sudo cargo test

# Run the application
sudo ./target/release/family-policy

# Apply policies from config
sudo family-policy --config browser-policy.yaml

# Preview changes (dry-run)
family-policy --dry-run

# Remove all policies
sudo family-policy --uninstall
```

## Architecture

### Multi-layer policy application system

1. **Config Layer** (`src/config.rs`): Parses YAML with multi-browser policy format where each policy can apply to multiple browsers with browser-specific extension IDs
2. **State Layer** (`src/state.rs`): Tracks applied policies via state file for idempotency and clean uninstall
3. **Policy Layer** (`src/policy/*.rs`): Browser-specific modules (chrome, firefox, edge) that handle policy application
4. **Platform Layer** (`src/platform/*.rs`): OS-specific implementations (windows registry, macos plist, linux JSON)

### Key architectural patterns

**Cross-platform strategy**: Runtime platform detection with conditional compilation for OS-specific code. Browser policy modules call platform-specific functions based on `current_platform()`.

**Idempotency**: Config is hashed (SHA-256) and compared with state file. Policies only applied if config changed. All policy writes are atomic replacements (clear then rewrite).

**State management**: Platform-specific state files track applied extensions and privacy settings per browser. Enables clean uninstall and change detection.

**Browser-specific translations**: Config supports multi-browser format. `to_browser_configs()` translates to browser-specific configs (ChromeConfig, FirefoxConfig, EdgeConfig) with appropriate extension URLs and privacy policy mappings.

### Policy implementation locations

**Chrome/Edge (Chromium)**:
- Windows: Registry at `HKLM\SOFTWARE\Policies\Google\Chrome` or `Microsoft\Edge`
- macOS: Plist at `/Library/Managed Preferences/com.google.Chrome.plist` or `com.microsoft.Edge.plist`
- Linux: JSON at `/etc/opt/chrome/policies/managed/` or `/etc/opt/microsoft/edge/policies/managed/`

**Firefox**:
- All platforms: `policies.json` in distribution folder
- Windows: `C:\Program Files\Mozilla Firefox\distribution\`
- macOS: `/Applications/Firefox.app/Contents/Resources/distribution/`
- Linux: `/etc/firefox/policies/`

### Privacy controls mapping

Browser privacy features map differently:
- Chrome: `IncognitoModeAvailability` (0/1/2) and `BrowserGuestModeEnabled` (bool)
- Firefox: `DisablePrivateBrowsing` (bool) only (no guest mode)
- Edge: `InPrivateModeAvailability` (0/1/2) and `BrowserGuestModeEnabled` (bool)

The config layer's `disable_private_mode` and `disable_guest_mode` translate to appropriate browser-specific policies.

## Important implementation notes

**Platform-specific code**: Use `#[cfg(target_os = "...")]` for OS-specific modules. Main policy functions use runtime `match current_platform()` to dispatch to platform implementations.

**Extension settings**: Config supports arbitrary extension settings via `HashMap<String, serde_json::Value>`. These pass through without validation to allow extension-specific configuration.

**Atomic writes**: All file operations use temp file + rename pattern via `atomic_write()` in `src/platform/common.rs` to prevent corruption.

**Privilege checking**: App requires admin/root privileges. Check happens early in execution via `ensure_admin_privileges()`.

**Firefox policy merging**: Firefox's `policies.json` may have pre-existing policies. Must merge rather than replace to preserve non-managed settings. See `merge_firefox_policies()` in `src/policy/firefox.rs`.

**State file locations** (platform-specific):
- Linux: `/var/lib/browser-extension-policy/state.json`
- macOS: `/Library/Application Support/browser-extension-policy/state.json`
- Windows: `C:\ProgramData\browser-extension-policy\state.json`

## Configuration format

The YAML config uses a multi-browser policy format where a single policy entry can specify which browsers it applies to and provide browser-specific extension IDs:

```yaml
chrome:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite
  disable_incognito: true
  disable_guest_mode: true

firefox:
  extensions:
    - id: uBOLite@raymondhill.net
      name: uBlock Origin Lite
      install_url: https://addons.mozilla.org/firefox/downloads/latest/ublock-origin-lite/latest.xpi
  disable_private_browsing: true

edge:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite
  disable_inprivate: true
  disable_guest_mode: true
```

See `src/config.rs` for full config structure and DESIGN.md for detailed format documentation.
