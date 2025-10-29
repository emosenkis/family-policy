# Browser Extension Policy Manager - Design Document

## Executive Summary

A cross-platform Rust application that manages browser extension force-install policies and privacy controls for Chrome, Firefox, and Edge across Windows, macOS, and Linux. The tool reads a YAML configuration file specifying:
- Which extensions to force-install for each browser
- Privacy mode controls (incognito/private browsing, guest mode)

The application applies the appropriate OS-specific policies and maintains state to enable idempotent updates and clean uninstallation.

## High-Level Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Interface                        │
│                    (clap argument parsing)                   │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Configuration Manager                     │
│          (Parse user config, validate extensions)            │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                       State Manager                          │
│      (Track installed extensions, enable idempotency)        │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Policy Orchestrator                       │
│        (Coordinate policy application across browsers)       │
└───────────┬──────────────┬──────────────┬───────────────────┘
            │              │              │
            ▼              ▼              ▼
   ┌────────────┐  ┌────────────┐  ┌────────────┐
   │   Chrome   │  │  Firefox   │  │    Edge    │
   │   Policy   │  │   Policy   │  │   Policy   │
   │   Writer   │  │   Writer   │  │   Writer   │
   └─────┬──────┘  └─────┬──────┘  └─────┬──────┘
         │               │               │
         └───────┬───────┴───────┬───────┘
                 │               │
                 ▼               ▼
       ┌──────────────┐  ┌──────────────┐
       │   Platform   │  │   Platform   │
       │   Specific   │  │  Abstraction │
       │   Writers    │  │    Layer     │
       └──────────────┘  └──────────────┘
            │                   │
    ┌───────┼───────┬───────────┼───────┐
    ▼       ▼       ▼           ▼       ▼
Windows   macOS  Linux      File I/O  Validation
Registry  plist   JSON
```

### Platform Matrix

| Browser | Windows Method | Linux Method | macOS Method |
|---------|---------------|--------------|--------------|
| Chrome  | Registry (HKLM) | JSON file (/etc/opt/chrome) | plist (/Library/Managed Preferences) |
| Firefox | policies.json | policies.json | policies.json |
| Edge    | Registry (HKLM) | JSON file (/etc/opt/microsoft/edge) | plist (/Library/Managed Preferences) |

## Configuration File Format

The user provides a YAML configuration file specifying policies that apply to multiple browsers:

```yaml
# browser-policy.yaml

policies:
  # Privacy controls that apply across browsers
  - name: Private browsing restrictions
    browsers:
      - chrome
      - firefox
      - edge
    disable_private_mode: true  # Disables incognito/private browsing/InPrivate
    disable_guest_mode: true    # Disables guest mode (Chrome and Edge only, ignored for Firefox)

  # Extension policy with browser-specific IDs and settings
  - name: uBlock Origin Lite
    browsers:
      - chrome
      - firefox
      - edge
    extensions:
      - name: uBlock Origin Lite
        id:
          firefox: uBOLite@raymondhill.net
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
          edge: ddkjiahejlhfcafbddmgiahcphecmpfh
        force_installed: true
        # Extension-specific settings (arbitrary JSON structure)
        # These settings are extension-specific and will be passed through
        # See https://github.com/uBlockOrigin/uBOL-home/wiki/Managed-settings
        settings:
          rulesets:
            - "+default"
            - "+isr-0"
          strictBlockMode: true
          defaultFiltering: optimal
          disableFirstRunPage: true
          disabledFeatures:
            - filteringMode
```

### Configuration Format Features

**Multi-browser policies**: Each policy entry can apply to multiple browsers simultaneously. Settings that don't apply to a particular browser are automatically ignored (e.g., `disable_guest_mode` is ignored for Firefox).

**Browser-specific extension IDs**: Extensions can have different IDs for different browsers by using a mapping structure. The system automatically generates appropriate install URLs from the browser's native web store.

**Arbitrary extension settings**: Extension settings are stored as an arbitrary JSON mapping (`settings: HashMap<String, serde_json::Value>`), allowing any extension-specific configuration to be passed through without needing to model it explicitly.

### Privacy Mode Policy Details

**Chrome:**
- `IncognitoModeAvailability`: 0 = enabled (default), 1 = disabled, 2 = forced
- `BrowserGuestModeEnabled`: true = enabled (default), false = disabled

**Firefox:**
- `DisablePrivateBrowsing`: true = disabled, false/absent = enabled (default)
- No guest mode available

**Edge:**
- `InPrivateModeAvailability`: 0 = enabled (default), 1 = disabled, 2 = forced
- `BrowserGuestModeEnabled`: true = enabled (default), false = disabled

## State Management

To enable idempotent operations, the tool maintains a state file tracking:
- Which extensions were installed for each browser
- Configuration hash to detect changes
- Timestamp of last modification

State file location (platform-specific):
- **Linux**: `/var/lib/browser-extension-policy/state.json` or `~/.local/share/browser-extension-policy/state.json`
- **macOS**: `/Library/Application Support/browser-extension-policy/state.json`
- **Windows**: `C:\ProgramData\browser-extension-policy\state.json`

State file format:
```json
{
    "version": "1.0",
    "config_hash": "sha256:abc123...",
    "last_updated": "2025-10-26T12:00:00Z",
    "applied_policies": {
        "chrome": {
            "extensions": ["ddkjiahejlhfcafbddmgiahcphecmpfh"],
            "disable_incognito": true,
            "disable_guest_mode": true
        },
        "firefox": {
            "extensions": ["uBOLite@raymondhill.net"],
            "disable_private_browsing": true
        },
        "edge": {
            "extensions": ["ddkjiahejlhfcafbddmgiahcphecmpfh"],
            "disable_inprivate": true,
            "disable_guest_mode": true
        }
    }
}
```

## File-by-File Design

### `src/main.rs`

**Purpose**: Application entry point, CLI parsing, and orchestration

**Dependencies**:
- `clap` (v4) for CLI argument parsing
- `anyhow` for error handling

**Key Functions**:

```rust
fn main() -> anyhow::Result<()>
```
- Parse command-line arguments
- Determine operation mode (install/uninstall)
- Load configuration file
- Load or create state
- Call appropriate orchestrator functions
- Handle errors and exit codes

**CLI Arguments**:
- `--config <PATH>`: Path to configuration file (default: `./browser-policy.yaml`)
- `--uninstall`: Remove all policies created by this tool
- `--dry-run`: Show what would be done without making changes
- `--verbose`: Enable verbose logging

### `src/config.rs`

**Purpose**: Configuration file parsing and validation

**Dependencies**:
- `serde` with derive feature
- `serde_yaml` for YAML parsing
- `url` for URL validation

**Data Structures**:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub policies: Vec<PolicyEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyEntry {
    pub name: String,
    pub browsers: Vec<Browser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_private_mode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_guest_mode: Option<bool>,
    #[serde(default)]
    pub extensions: Vec<ExtensionEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtensionEntry {
    pub name: String,
    pub id: BrowserIdMap,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_installed: Option<bool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BrowserIdMap {
    Single(String),
    Multiple(HashMap<Browser, String>),
}

// Internal structures for policy modules
#[derive(Debug, Clone)]
pub struct ChromeConfig {
    pub extensions: Vec<Extension>,
    pub disable_incognito: Option<bool>,
    pub disable_guest_mode: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct FirefoxConfig {
    pub extensions: Vec<Extension>,
    pub disable_private_browsing: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct EdgeConfig {
    pub extensions: Vec<Extension>,
    pub disable_inprivate: Option<bool>,
    pub disable_guest_mode: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub update_url: Option<String>,
    pub install_url: Option<String>,
    pub settings: HashMap<String, serde_json::Value>,
}
```

**Key Functions**:

```rust
pub fn load_config(path: &Path) -> anyhow::Result<Config>
```
- Read YAML file
- Parse into Config struct with policy entries
- Validate extension IDs and URLs
- Return validated config

```rust
pub fn validate_config(config: &Config) -> anyhow::Result<()>
```
- Ensure at least one policy is configured
- Validate each policy entry has at least one browser
- Ensure extensions have IDs for all specified browsers
- Validate Chrome/Edge extension IDs (32 char alphanumeric)
- Validate Firefox extension IDs (non-empty)

```rust
pub fn to_browser_configs(config: &Config) -> (Option<ChromeConfig>, Option<FirefoxConfig>, Option<EdgeConfig>)
```
- Convert the multi-browser policy format to browser-specific configurations
- Merge settings from all policies that apply to each browser
- Generate appropriate update/install URLs for extensions
- Return browser-specific configs for use by policy modules

### `src/state.rs`

**Purpose**: State tracking for idempotent operations

**Dependencies**:
- `serde` + `serde_json`
- `chrono` for timestamps
- `sha2` for config hashing
- `directories` for platform-specific paths

**Data Structures**:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct State {
    pub version: String,
    pub config_hash: String,
    pub last_updated: DateTime<Utc>,
    pub applied_policies: AppliedPolicies,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppliedPolicies {
    pub chrome: Option<BrowserState>,
    pub firefox: Option<BrowserState>,
    pub edge: Option<BrowserState>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrowserState {
    pub extensions: Vec<String>, // Extension IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_incognito: Option<bool>,     // Chrome only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_inprivate: Option<bool>,     // Edge only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_private_browsing: Option<bool>,  // Firefox only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_guest_mode: Option<bool>,    // Chrome/Edge only
}
```

**Key Functions**:

```rust
pub fn load_state() -> anyhow::Result<Option<State>>
```
- Load state from platform-specific location
- Return None if state doesn't exist
- Validate state version compatibility

```rust
pub fn save_state(state: &State) -> anyhow::Result<()>
```
- Serialize state to JSON
- Atomically write to state file (write temp, rename)
- Create parent directories if needed

```rust
pub fn compute_config_hash(config: &Config) -> String
```
- Serialize config to stable JSON representation
- Compute SHA-256 hash
- Return hex-encoded hash

```rust
pub fn get_state_path() -> PathBuf
```
- Return platform-specific state file path
- Use `directories` crate for standard locations

### `src/browser.rs`

**Purpose**: Browser detection and type definitions

**Data Structures**:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Browser {
    Chrome,
    Firefox,
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}
```

**Key Functions**:

```rust
pub fn current_platform() -> Platform
```
- Use `cfg!()` macros to determine current OS
- Return appropriate Platform enum

```rust
pub fn is_browser_available(browser: Browser) -> bool
```
- Check if browser is installed on the system
- Platform-specific logic to find browser installation

### `src/policy/mod.rs`

**Purpose**: Policy orchestration across browsers

**Dependencies**:
- `anyhow` for error handling

**Key Functions**:

```rust
pub fn apply_policies(
    config: &Config,
    current_state: Option<&State>,
) -> anyhow::Result<AppliedPolicies>
```
- For each browser in config:
  - Check if extensions changed from current_state
  - Call browser-specific apply function
  - Collect results
- Return new AppliedPolicies

```rust
pub fn remove_policies(state: &State) -> anyhow::Result<()>
```
- For each browser in state:
  - Call browser-specific removal function
- Clean up all policies created by this tool

```rust
fn policies_changed(
    browser: Browser,
    config: &BrowserConfig,
    state: Option<&BrowserState>,
) -> bool
```
- Compare current config with previous state
- Return true if policies need updating

### `src/policy/chrome.rs`

**Purpose**: Chrome-specific policy implementation

**Key Functions**:

```rust
pub fn apply_chrome_policies(
    config: &ChromeConfig,
) -> anyhow::Result<BrowserState>
```
- Detect current platform
- Apply extension policies via platform-specific implementation:
  - Windows: `apply_chrome_windows()`
  - macOS: `apply_chrome_macos()`
  - Linux: `apply_chrome_linux()`
- Apply privacy policies:
  - Set `IncognitoModeAvailability` if configured
  - Set `BrowserGuestModeEnabled` if configured
- Return BrowserState with applied extensions and privacy settings

```rust
pub fn remove_chrome_policies() -> anyhow::Result<()>
```
- Call platform-specific removal function
- Clean up all Chrome policies

```rust
#[cfg(target_os = "windows")]
fn apply_chrome_windows(config: &ChromeConfig) -> anyhow::Result<()>
```
- Open registry key: `HKLM\SOFTWARE\Policies\Google\Chrome`
- For extensions:
  - Clear and set `ExtensionInstallForcelist` subkey values (1, 2, 3, ...)
  - Format: `{id};{update_url}`
- For privacy controls:
  - Set `IncognitoModeAvailability` DWORD = 1 if disable_incognito is true
  - Set `BrowserGuestModeEnabled` DWORD = 0 if disable_guest_mode is true

```rust
#[cfg(target_os = "macos")]
fn apply_chrome_macos(config: &ChromeConfig) -> anyhow::Result<()>
```
- Create plist at `/Library/Managed Preferences/com.google.Chrome.plist`
- Set `ExtensionInstallForcelist` array for extensions
- Set `IncognitoModeAvailability` integer if configured
- Set `BrowserGuestModeEnabled` boolean if configured

```rust
#[cfg(target_os = "linux")]
fn apply_chrome_linux(config: &ChromeConfig) -> anyhow::Result<()>
```
- Create JSON file at `/etc/opt/chrome/policies/managed/browser-policy.json`
- Set `ExtensionInstallForcelist` array
- Set `IncognitoModeAvailability` number if configured
- Set `BrowserGuestModeEnabled` boolean if configured
- Ensure proper permissions (readable by all users)

**Helper Functions**:

```rust
fn format_chrome_extension_entry(ext: &Extension) -> String
```
- Format extension for Chrome policy
- Use default update URL if not specified
- Default: `https://clients2.google.com/service/update2/crx`

### `src/policy/firefox.rs`

**Purpose**: Firefox-specific policy implementation

**Key Functions**:

```rust
pub fn apply_firefox_policies(
    config: &FirefoxConfig,
) -> anyhow::Result<BrowserState>
```
- Detect current platform
- Find Firefox installation path
- Create/update policies.json at appropriate location with:
  - ExtensionSettings for force-installed extensions
  - DisablePrivateBrowsing if configured
- Return BrowserState with applied extensions and privacy settings

```rust
pub fn remove_firefox_policies() -> anyhow::Result<()>
```
- Find and remove policies.json files
- Clean up distribution directories if empty

```rust
fn get_firefox_policy_path() -> anyhow::Result<PathBuf>
```
- Return platform-specific policies.json path:
  - Windows: `C:\Program Files\Mozilla Firefox\distribution\policies.json`
  - macOS: `/Applications/Firefox.app/Contents/Resources/distribution/policies.json`
  - Linux: `/etc/firefox/policies/policies.json`
- Verify Firefox installation exists

```rust
fn create_firefox_policies_json(config: &FirefoxConfig) -> anyhow::Result<serde_json::Value>
```
- Create Firefox policies.json structure:
```json
{
  "policies": {
    "ExtensionSettings": {
      "extension-id@example.com": {
        "installation_mode": "force_installed",
        "install_url": "https://addons.mozilla.org/..."
      }
    },
    "DisablePrivateBrowsing": true
  }
}
```
- Include DisablePrivateBrowsing only if configured in config

```rust
fn merge_firefox_policies(
    existing: Option<serde_json::Value>,
    config: &FirefoxConfig,
) -> anyhow::Result<serde_json::Value>
```
- If policies.json exists, read it
- Preserve policies not managed by us
- Update our managed settings:
  - ExtensionSettings entries for our extensions
  - DisablePrivateBrowsing if configured
- Mark our entries with special metadata for tracking

### `src/policy/edge.rs`

**Purpose**: Edge-specific policy implementation

**Key Functions**:

```rust
pub fn apply_edge_policies(
    config: &EdgeConfig,
) -> anyhow::Result<BrowserState>
```
- Similar structure to Chrome (Edge uses same Chromium policy system)
- Apply extension and privacy policies via platform-specific implementation
- Privacy controls: InPrivateModeAvailability, BrowserGuestModeEnabled

```rust
#[cfg(target_os = "windows")]
fn apply_edge_windows(config: &EdgeConfig) -> anyhow::Result<()>
```
- Use registry key: `HKLM\SOFTWARE\Policies\Microsoft\Edge`
- Set `ExtensionInstallForcelist` subkey for extensions
- Set `InPrivateModeAvailability` DWORD = 1 if disable_inprivate is true
- Set `BrowserGuestModeEnabled` DWORD = 0 if disable_guest_mode is true

```rust
#[cfg(target_os = "macos")]
fn apply_edge_macos(config: &EdgeConfig) -> anyhow::Result<()>
```
- Use plist: `/Library/Managed Preferences/com.microsoft.Edge.plist`
- Set ExtensionInstallForcelist, InPrivateModeAvailability, BrowserGuestModeEnabled

```rust
#[cfg(target_os = "linux")]
fn apply_edge_linux(config: &EdgeConfig) -> anyhow::Result<()>
```
- Use JSON: `/etc/opt/microsoft/edge/policies/managed/browser-policy.json`
- Set all extension and privacy policies in single JSON file

### `src/platform/mod.rs`

**Purpose**: Platform abstraction and common utilities

**Modules**:
- `pub mod registry;` (Windows only)
- `pub mod plist;` (macOS only)
- `pub mod json_policy;` (Linux only)
- `pub mod common;` (all platforms)

**Key Functions**:

```rust
pub fn ensure_admin_privileges() -> anyhow::Result<()>
```
- Check if running with necessary privileges
- Windows: Check for Administrator
- macOS/Linux: Check for root/sudo
- Return error if insufficient privileges

### `src/platform/windows.rs`

**Purpose**: Windows registry operations

**Dependencies**:
- `winreg` for registry access

**Key Functions**:

```rust
#[cfg(target_os = "windows")]
pub fn write_registry_policy(
    key_path: &str,
    values: Vec<(String, String)>,
) -> anyhow::Result<()>
```
- Open or create registry key at HKLM\{key_path}
- Delete all existing numbered values (for idempotency)
- Write new values as 1, 2, 3, ...
- Values are REG_SZ type

```rust
#[cfg(target_os = "windows")]
pub fn write_registry_value(
    key_path: &str,
    value_name: &str,
    value: RegistryValue,
) -> anyhow::Result<()>
```
- Open or create registry key at HKLM\{key_path}
- Write a single named value (REG_DWORD, REG_SZ, etc.)
- Used for privacy control policies

```rust
#[cfg(target_os = "windows")]
pub enum RegistryValue {
    Dword(u32),
    String(String),
}
```
- Enum to support different registry value types

```rust
#[cfg(target_os = "windows")]
pub fn remove_registry_policy(key_path: &str) -> anyhow::Result<()>
```
- Delete registry key and all subkeys
- Handle case where key doesn't exist (idempotent)

```rust
#[cfg(target_os = "windows")]
pub fn read_registry_policy(key_path: &str) -> anyhow::Result<Vec<String>>
```
- Read all numbered values from registry key
- Return as vector of strings
- Used for detecting current state

### `src/platform/macos.rs`

**Purpose**: macOS plist operations

**Dependencies**:
- `plist` for plist serialization

**Key Functions**:

```rust
#[cfg(target_os = "macos")]
pub fn write_plist_policy(
    bundle_id: &str,
    updates: HashMap<String, plist::Value>,
) -> anyhow::Result<()>
```
- Create/update plist at `/Library/Managed Preferences/{bundle_id}.plist`
- Merge updates into existing plist (preserving unrelated keys)
- Support multiple value types (arrays, integers, booleans)
- Set proper permissions (readable by all)
- Updates can include extension arrays and privacy policy values

```rust
#[cfg(target_os = "macos")]
pub fn remove_plist_policy(
    bundle_id: &str,
    key: &str,
) -> anyhow::Result<()>
```
- Read plist file
- Remove specified key
- If plist becomes empty, delete file
- Otherwise write updated plist

```rust
#[cfg(target_os = "macos")]
fn merge_plist(
    existing: Option<plist::Value>,
    key: &str,
    value: plist::Value,
) -> plist::Value
```
- Helper to merge new values into existing plist
- Preserve unrelated keys

### `src/platform/linux.rs`

**Purpose**: Linux JSON policy file operations

**Dependencies**:
- `serde_json` for JSON manipulation

**Key Functions**:

```rust
#[cfg(target_os = "linux")]
pub fn write_json_policy(
    policy_dir: &Path,
    policy_name: &str,
    data: serde_json::Value,
) -> anyhow::Result<()>
```
- Create directory at policy_dir if doesn't exist
- Write JSON file: {policy_dir}/{policy_name}.json
- Format JSON with pretty printing
- Set permissions: 644 (readable by all)

```rust
#[cfg(target_os = "linux")]
pub fn remove_json_policy(
    policy_dir: &Path,
    policy_name: &str,
) -> anyhow::Result<()>
```
- Remove JSON policy file
- Clean up directory if empty

```rust
#[cfg(target_os = "linux")]
pub fn read_json_policy(
    policy_dir: &Path,
    policy_name: &str,
) -> anyhow::Result<Option<serde_json::Value>>
```
- Read existing JSON policy
- Return None if doesn't exist
- Parse and return as serde_json::Value

### `src/platform/common.rs`

**Purpose**: Cross-platform utility functions

**Key Functions**:

```rust
pub fn atomic_write(path: &Path, content: &[u8]) -> anyhow::Result<()>
```
- Write to temporary file in same directory
- Sync to disk
- Rename to target path (atomic on Unix/NTFS)
- Ensures no partial writes on crash

```rust
pub fn set_permissions_readable_all(path: &Path) -> anyhow::Result<()>
```
- Platform-specific permission setting
- Unix: chmod 644 for files, 755 for dirs
- Windows: Grant read access to Everyone group

```rust
pub fn ensure_directory_exists(path: &Path) -> anyhow::Result<()>
```
- Create directory and all parents if needed
- Set appropriate permissions
- Idempotent (no error if exists)

## Dependency List

Based on the design, these crates will be added to `Cargo.toml`:

**Core Dependencies**:
- `serde = { version = "1", features = ["derive"] }` - Serialization framework
- `serde_json = "1"` - JSON support
- `serde_yaml = "0.9"` - YAML config parsing
- `anyhow = "1"` - Error handling
- `clap = { version = "4", features = ["derive"] }` - CLI parsing
- `directories = "5"` - Platform-specific paths
- `chrono = { version = "0.4", features = ["serde"] }` - Timestamps
- `sha2 = "0.10"` - Config hashing
- `url = "2"` - URL validation

**Platform-Specific Dependencies**:
- `winreg = "0.52"` - Windows registry (Windows only)
- `plist = "1"` - macOS plist files (macOS only)

**Optional/Development Dependencies**:
- `tempfile = "3"` - Temporary files for testing

## Conditional Compilation Strategy

To avoid code duplication while supporting multiple platforms, we use:

1. **Platform detection at runtime** where possible:
```rust
pub fn apply_chrome_policies(extensions: &[Extension]) -> anyhow::Result<BrowserState> {
    match current_platform() {
        Platform::Windows => apply_chrome_windows(extensions),
        Platform::MacOS => apply_chrome_macos(extensions),
        Platform::Linux => apply_chrome_linux(extensions),
    }
}
```

2. **Conditional compilation for platform-specific modules**:
```rust
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;
```

3. **Trait-based abstraction** for common operations:
```rust
trait PolicyWriter {
    fn write_policy(&self, key: &str, values: Vec<String>) -> anyhow::Result<()>;
    fn remove_policy(&self, key: &str) -> anyhow::Result<()>;
}

// Implement for each platform
#[cfg(target_os = "windows")]
impl PolicyWriter for RegistryWriter { ... }

#[cfg(target_os = "macos")]
impl PolicyWriter for PlistWriter { ... }

#[cfg(target_os = "linux")]
impl PolicyWriter for JsonWriter { ... }
```

## Idempotency Strategy

The tool achieves idempotency through:

1. **State tracking**: Store hash of config and list of applied extensions
2. **Comparison before action**: Only modify policies if config changed
3. **Atomic replacements**: Always clear and rewrite policies completely
4. **Atomic file writes**: Use temp file + rename pattern

Example flow:
```
1. Load browser-policy.yaml
2. Compute hash of config
3. Load state.json
4. Compare config hash with state.config_hash
5. If different:
   a. Clear existing browser policies (extensions + privacy controls)
   b. Write new browser policies:
      - Extension force-install policies
      - Privacy mode restrictions (incognito/private/guest)
   c. Update state.json with new hash, extension list, and privacy settings
6. If same:
   a. Log "No changes detected"
   b. Exit (no-op)
```

## Uninstall Process

When run with `--uninstall`:

1. Load state.json
2. For each browser in state.applied_policies:
   - Call browser-specific remove function
   - Remove registry keys / delete JSON files / remove plist entries
3. Delete state.json
4. Log summary of removed policies

## Error Handling Strategy

- Use `anyhow::Result<T>` for all fallible operations
- Add context to errors as they propagate:
  ```rust
  apply_chrome_windows(extensions)
      .context("Failed to apply Chrome policies on Windows")?
  ```
- At top level (main.rs), catch errors and display user-friendly messages
- Exit with non-zero code on failure
- Log detailed errors to stderr

## Security Considerations

1. **Privilege checking**: Verify admin/root privileges before making system changes
2. **Path validation**: Sanitize file paths to prevent directory traversal
3. **Atomic operations**: Use atomic file writes to prevent corruption
4. **Permission setting**: Ensure policy files are readable but not writable by normal users
5. **Input validation**: Validate extension IDs and URLs from config file

## Testing Strategy

1. **Unit tests**: Test individual functions with mock data
2. **Integration tests**: Test full flow with temporary directories
3. **Platform-specific tests**: Use `#[cfg(target_os = "...")]` for platform tests
4. **Dry-run mode**: Allow testing without making actual changes

## Privacy Mode Implementation Details

### Chrome Policies

**IncognitoModeAvailability** (Registry/JSON: integer, plist: number)
- Path: Chrome policies root
- Values: 0 = Available (default), 1 = Disabled, 2 = Forced
- Implementation: Set to 1 when disable_incognito = true

**BrowserGuestModeEnabled** (Registry: DWORD, JSON/plist: boolean)
- Path: Chrome policies root
- Values: true = Enabled (default), false = Disabled
- Implementation: Set to false when disable_guest_mode = true

### Firefox Policies

**DisablePrivateBrowsing** (JSON: boolean)
- Path: policies.json → policies → DisablePrivateBrowsing
- Values: true = Disabled, false/absent = Enabled (default)
- Implementation: Add to policies object when disable_private_browsing = true

### Edge Policies

**InPrivateModeAvailability** (Registry/JSON: integer, plist: number)
- Path: Edge policies root
- Values: 0 = Available (default), 1 = Disabled, 2 = Forced
- Implementation: Set to 1 when disable_inprivate = true

**BrowserGuestModeEnabled** (Registry: DWORD, JSON/plist: boolean)
- Path: Edge policies root
- Values: true = Enabled (default), false = Disabled
- Implementation: Set to false when disable_guest_mode = true

## Future Enhancements

1. Support for additional browsers (Brave, Vivaldi, Opera)
2. Support for extension settings/configuration (not just installation)
3. Additional privacy controls:
   - Disable third-party cookies
   - Enforce safe search
   - Block dangerous downloads
4. Web UI for config generation
5. Centralized config server support
6. Logging to syslog/Windows Event Log
7. Rollback functionality
8. Extension blocklist support (complementary to force-install)
9. Profile-specific policies (not just system-wide)
10. Remote config fetching with signature verification
