# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A cross-platform Rust CLI that manages browser extension force-install policies and privacy controls for Chrome, Firefox, and Edge across Windows, macOS, and Linux. The tool operates in two modes:

1. **Local mode**: Reads YAML config files from disk and applies policies directly
2. **Agent mode**: Polls a GitHub repository for policy changes and automatically applies them

The application applies OS-specific policies (Windows registry, macOS plists, Linux JSON) and maintains state for idempotent operations.

## Build and Test Commands

This is a Tauri application with a Vue frontend and Rust backend. **All build commands must be run inside the devcontainer.**

```bash
# Build the project (must be run in devcontainer, takes several minutes)
devcontainer exec --workspace-folder /var/home/eitan/projects/family-policy pnpm tauri build --bundles appimage

# Start the devcontainer first (if not already running)
devcontainer up --workspace-folder /var/home/eitan/projects/family-policy

# Run tests (some require sudo/admin privileges)
cargo test

# Run tests with privileges (Linux/macOS)
sudo cargo test

# Run a specific test
cargo test test_name

# Run tests for a specific module
cargo test config::tests
cargo test state::tests

# Run the application (local mode)
sudo ./target/release/family-policy --config browser-policy.yaml

# Preview changes without applying (dry-run)
family-policy --dry-run --config browser-policy.yaml

# Remove all policies (local mode)
sudo family-policy --uninstall
```

**Note:** The build output will be at `src-tauri/target/release/bundle/appimage/Family Policy_0.1.0_amd64.AppImage`

## Getting Started

### Create an Example Configuration

Generate a well-documented example configuration file to get started:

```bash
# Create config in current directory
family-policy config init

# Specify custom output path
family-policy config init --output /path/to/config.yaml

# Overwrite existing file
family-policy config init --force
```

The generated config file includes:
- Comprehensive documentation and examples
- Privacy controls (disable private/incognito browsing)
- Extension installation examples (single and multi-browser)
- Extension settings examples
- Platform-specific behavior documentation

### Apply Your Configuration

Once you've customized your config file:

```bash
# Preview changes without applying
family-policy --config family-policy.yaml --dry-run

# Apply the configuration
sudo family-policy --config family-policy.yaml

# Remove all applied policies
sudo family-policy --uninstall
```

## Agent Mode Commands

The agent mode enables remote policy management via GitHub polling:

```bash
# Start agent daemon (foreground mode)
sudo family-policy start --no-daemon

# Check for policy updates immediately
sudo family-policy check-now

# Show agent status
family-policy status

# Show currently applied configuration
family-policy show-config
```

Note: Agent mode configuration is managed through the agent config file (not via CLI setup command).

## Architecture

### Dual-Mode Operation

The application has two entry points:
- **Local mode** (default): Traditional CLI that reads config file from disk (backward compatible with original behavior)
- **Agent mode** (`agent` subcommand): Daemon that polls GitHub for policy updates and applies them automatically

Both modes share the same policy application logic (`src/policy/*.rs`) and state management (`src/state.rs`).

### Multi-layer policy application system

1. **Config Layer** (`src/config.rs`): Parses YAML with multi-browser policy format where each policy can apply to multiple browsers with browser-specific extension IDs
2. **State Layer** (`src/state.rs`): Tracks applied policies via state file for idempotency and clean uninstall
3. **Policy Layer** (`src/policy/*.rs`): Browser-specific modules (chrome, firefox, edge) that handle policy application
4. **Platform Layer** (`src/platform/*.rs`): OS-specific implementations (windows registry, macos plist, linux JSON)
5. **Agent Layer** (`src/agent/*.rs`): GitHub polling, ETag-based change detection, and automatic policy application

### Key architectural patterns

**Cross-platform strategy**: Runtime platform detection with conditional compilation for OS-specific code. Browser policy modules call platform-specific functions based on `current_platform()`.

**Idempotency**: Config is hashed (SHA-256) and compared with state file. Policies only applied if config changed. All policy writes are atomic replacements (clear then rewrite).

**State management**: Platform-specific state files track applied extensions and privacy settings per browser. Enables clean uninstall and change detection. Agent mode maintains separate state (`src/agent/state.rs`) for tracking GitHub ETags and last poll times.

**Browser-specific translations**: Config supports multi-browser format. `to_browser_configs()` in `src/config.rs` translates to browser-specific configs (ChromeConfig, FirefoxConfig, EdgeConfig) with appropriate extension URLs and privacy policy mappings.

**Agent polling**: Uses ETag headers for efficient change detection. Jittered polling interval to avoid thundering herd. Exponential backoff on failures. All GitHub communication via `reqwest` with TLS (`rustls-tls`).

### Policy implementation locations

**Chrome/Edge (Chromium)** - Share common implementation in `src/policy/chromium_common.rs`:
- Windows: Registry at `HKLM\SOFTWARE\Policies\Google\Chrome` or `Microsoft\Edge`
- macOS: Plist at `/Library/Managed Preferences/com.google.Chrome.plist` or `com.microsoft.Edge.plist`
- Linux: JSON at `/etc/opt/chrome/policies/managed/` or `/etc/opt/microsoft/edge/policies/managed/`

**Firefox**:
- All platforms: `policies.json` in distribution folder
- Windows: `C:\Program Files\Mozilla Firefox\distribution\`
- macOS: `/Applications/Firefox.app/Contents/Resources/distribution/`
- Linux: `/etc/firefox/policies/`

### Code organization improvements (2025-11-14/15)

**Chromium Common Module**: Chrome and Edge policy modules (`src/policy/chrome.rs` and `src/policy/edge.rs`) now share common logic through `src/policy/chromium_common.rs`. This reduces code duplication by ~490 lines and makes it easier to add support for other Chromium-based browsers (Brave, Vivaldi, etc.).

**Modular Command Structure**: Main entry point (`src/main.rs`) refactored from 888 lines to 36 lines (-95%). Command logic organized into focused modules:
- `src/cli.rs` - CLI argument parsing (75 lines)
- `src/commands/agent.rs` - Agent subcommands (549 lines)
- `src/commands/local.rs` - Local mode operations (221 lines)
- `src/commands/utils.rs` - Shared utilities (37 lines)

This modular structure improves maintainability, testability, and adheres to Single Responsibility Principle.

### Privacy controls mapping

Browser privacy features map differently:
- Chrome: `IncognitoModeAvailability` (0/1/2) and `BrowserGuestModeEnabled` (bool)
- Firefox: `DisablePrivateBrowsing` (bool) only (no guest mode)
- Edge: `InPrivateModeAvailability` (0/1/2) and `BrowserGuestModeEnabled` (bool)

The config layer's `disable_private_mode` and `disable_guest_mode` translate to appropriate browser-specific policies.

## Important implementation notes

**Platform-specific code**: Use `#[cfg(target_os = "...")]` for OS-specific modules. Main policy functions use runtime `match current_platform()` to dispatch to platform implementations.

**Extension settings**: Config supports arbitrary extension settings via `HashMap<String, serde_json::Value>`. These pass through without validation to allow extension-specific configuration. Note: Settings field is present in `Extension` struct but not yet used by policy implementations (reserved for future functionality).

**Atomic writes**: All file operations use temp file + rename pattern via `atomic_write()` in `src/platform/common.rs` to prevent corruption.

**Privilege checking**: App requires admin/root privileges. Check happens early in execution via `ensure_admin_privileges()`. Agent mode also requires privileges since it applies policies.

**Firefox policy merging**: Firefox's `policies.json` may have pre-existing policies. Must merge rather than replace to preserve non-managed settings. See `merge_firefox_policies()` in `src/policy/firefox.rs`.

**State file locations** (platform-specific):
- Linux: `/var/lib/browser-extension-policy/state.json`
- macOS: `/Library/Application Support/browser-extension-policy/state.json`
- Windows: `C:\ProgramData\browser-extension-policy\state.json`

**Agent configuration locations**:
- Linux: `/etc/browser-extension-policy/agent-config.toml`
- macOS: `/Library/Application Support/browser-extension-policy/agent-config.toml`
- Windows: `C:\ProgramData\browser-extension-policy\agent-config.toml`

**Agent state locations**:
- Linux: `/var/lib/browser-extension-policy/agent-state.json`
- macOS: `/Library/Application Support/browser-extension-policy/agent-state.json`
- Windows: `C:\ProgramData\browser-extension-policy\agent-state.json`

## Configuration format

The YAML config uses a multi-browser policy format where a single policy entry can specify which browsers it applies to and provide browser-specific extension IDs:

```yaml
policies:
  # Privacy controls that apply across browsers
  - name: Private browsing restrictions
    browsers:
      - chrome
      - firefox
      - edge
    disable_private_mode: true  # Disables incognito/private browsing/InPrivate
    disable_guest_mode: true    # Disables guest mode (Chrome and Edge only)

  # Extension policy with browser-specific IDs
  - name: uBlock Origin Lite
    browsers:
      - chrome
      - firefox
      - edge
    extensions:
      - name: uBlock Origin Lite
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
          firefox: uBOLite@raymondhill.net
          edge: ddkjiahejlhfcafbddmgiahcphecmpfh
        force_installed: true
        # Optional extension-specific settings (arbitrary JSON)
        settings:
          someKey: someValue
```

**Extension ID formats**:
- Chrome/Edge: 32-character lowercase alphanumeric strings (e.g., `ddkjiahejlhfcafbddmgiahcphecmpfh`)
- Firefox: Email-style IDs (e.g., `uBOLite@raymondhill.net`) or UUID format

**ID mapping options**:
- Single ID for all browsers: `id: single-id-string`
- Browser-specific IDs: `id: { chrome: chrome-id, firefox: firefox-id, edge: edge-id }`

See `src/config.rs` for full config structure and validation logic. See DESIGN.md for detailed format documentation.

## Agent Architecture

The agent system (`src/agent/`) implements GitHub-based remote policy management:

- **Poller** (`poller.rs`): Fetches policy from raw GitHub URL using ETag for efficiency
- **Scheduler** (`scheduler.rs`): Manages polling intervals with jitter to prevent synchronized requests
- **Daemon** (`daemon.rs`): Main agent loop that polls, detects changes, and applies policies
- **Config** (`config.rs`): Agent-specific configuration (GitHub URL, token, polling interval)
- **State** (`state.rs`): Tracks ETag, last check time, last update time, and applied policies

The agent validates policies before applying them and maintains a separate state file to track the current applied configuration and GitHub metadata.
