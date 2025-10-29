# Browser Extension Policy Manager

A cross-platform Rust application that manages browser extension force-install policies and privacy controls for Chrome, Firefox, and Edge across Windows, macOS, and Linux.

## Features

- **Cross-Platform**: Works on Windows, macOS, and Linux
- **Multi-Browser Support**: Manages policies for Chrome, Firefox, and Edge
- **Extension Management**: Force-install browser extensions system-wide
- **Privacy Controls**:
  - Disable incognito/private browsing modes
  - Disable guest browsing modes
- **Idempotent**: Safely run multiple times with the same configuration
- **State Tracking**: Tracks applied policies for clean uninstallation
- **Dry-Run Mode**: Preview changes before applying them

## Installation

### From Source

```bash
git clone <repository-url>
cd family-policy
cargo build --release
```

The compiled binary will be at `target/release/family-policy`.

## Usage

### Basic Usage

```bash
# Apply policies from default config file (browser-policy.yaml)
sudo family-policy

# Apply policies from a specific config file
sudo family-policy --config /path/to/config.yaml

# Preview changes without applying (dry-run)
family-policy --dry-run

# Remove all policies
sudo family-policy --uninstall

# Show help
family-policy --help
```

### Platform-Specific Notes

**Linux**: Run with `sudo`:
```bash
sudo ./family-policy
```

**macOS**: Run with `sudo`:
```bash
sudo ./family-policy
```

**Windows**: Run as Administrator (right-click → "Run as Administrator")

## Configuration

Create a YAML configuration file (default: `browser-policy.yaml`):

```yaml
chrome:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite
      update_url: https://clients2.google.com/service/update2/crx

  # Privacy controls (optional)
  disable_incognito: true
  disable_guest_mode: true

firefox:
  extensions:
    - id: uBOLite@raymondhill.net
      name: uBlock Origin Lite
      install_url: https://addons.mozilla.org/firefox/downloads/latest/ublock-origin-lite/latest.xpi

  # Privacy controls (optional)
  disable_private_browsing: true

edge:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite
      update_url: https://clients2.google.com/service/update2/crx

  # Privacy controls (optional)
  disable_inprivate: true
  disable_guest_mode: true
```

### Finding Extension IDs

**Chrome/Edge**:
1. Visit the extension page in the Chrome Web Store or Edge Add-ons store
2. Look at the URL - the ID is the 32-character string
3. Example: `https://chrome.google.com/webstore/detail/EXTENSION_ID_HERE`

**Firefox**:
1. Install the extension in Firefox
2. Go to `about:debugging#/runtime/this-firefox`
3. Find the extension and look for "Extension ID"

## How It Works

### Extension Policies

**Chrome/Edge (Chromium-based)**:
- **Windows**: Writes to registry at `HKLM\SOFTWARE\Policies\Google\Chrome` (or Microsoft\Edge)
- **macOS**: Creates plist at `/Library/Managed Preferences/com.google.Chrome.plist`
- **Linux**: Creates JSON at `/etc/opt/chrome/policies/managed/browser-policy.json`

**Firefox**:
- **All Platforms**: Creates `policies.json` file:
  - Windows: `C:\Program Files\Mozilla Firefox\distribution\policies.json`
  - macOS: `/Applications/Firefox.app/Contents/Resources/distribution/policies.json`
  - Linux: `/etc/firefox/policies/policies.json`

### Privacy Policies

**Chrome**:
- `IncognitoModeAvailability`: 0 = available, 1 = disabled, 2 = forced
- `BrowserGuestModeEnabled`: true = enabled, false = disabled

**Firefox**:
- `DisablePrivateBrowsing`: true = disabled, false = enabled

**Edge**:
- `InPrivateModeAvailability`: 0 = available, 1 = disabled, 2 = forced
- `BrowserGuestModeEnabled`: true = enabled, false = disabled

### State Management

The tool maintains a state file to track applied policies:
- **Linux**: `/var/lib/browser-extension-policy/state.json`
- **macOS**: `/Library/Application Support/browser-extension-policy/state.json`
- **Windows**: `C:\ProgramData\browser-extension-policy\state.json`

This enables:
- Idempotent operations (safe to run multiple times)
- Clean uninstallation
- Change detection (only applies policies when config changes)

## Examples

### Example 1: uBlock Origin Lite Only

```yaml
chrome:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite

firefox:
  extensions:
    - id: uBOLite@raymondhill.net
      name: uBlock Origin Lite
      install_url: https://addons.mozilla.org/firefox/downloads/latest/ublock-origin-lite/latest.xpi

edge:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite
```

### Example 2: Privacy Controls Only

```yaml
chrome:
  disable_incognito: true
  disable_guest_mode: true

firefox:
  disable_private_browsing: true

edge:
  disable_inprivate: true
  disable_guest_mode: true
```

### Example 3: Multiple Extensions

```yaml
chrome:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite
    - id: cjpalhdlnbpafiamejdnhcphjbkeiagm
      name: uBlock Origin
    - id: gcbommkclmclpchllfjekcdonpmejbdp
      name: HTTPS Everywhere
```

## Troubleshooting

### Policies Not Applied

1. **Check privileges**: Ensure you're running as admin/root
2. **Restart browsers**: Close all browser instances and reopen
3. **Check state file**: Look at the state file to see what was applied
4. **Use verbose mode**: Run with `--verbose` flag for more details

### Uninstallation Issues

```bash
# Force removal (ignores errors)
sudo family-policy --uninstall

# Manually delete state file if needed (Linux)
sudo rm /var/lib/browser-extension-policy/state.json
```

### Permission Errors

Ensure you're running with appropriate privileges:
- **Linux/macOS**: Use `sudo`
- **Windows**: Run as Administrator

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running Tests (with admin privileges)

Some tests require admin/root privileges to access system directories:

```bash
# Linux/macOS
sudo cargo test

# Windows (run as Administrator)
cargo test
```

## Architecture

See [DESIGN.md](DESIGN.md) for detailed architecture and implementation notes.

## License

See LICENSE file for details.

## Safety and Security

This tool modifies system-wide browser policies and requires administrator/root privileges. Use with caution:

- Always test with `--dry-run` first
- Keep backups of any existing policies
- Review the configuration file before applying
- Use `--uninstall` to cleanly remove all policies

## Contributing

Contributions are welcome! Please ensure:
- Code compiles on all three platforms (Windows, macOS, Linux)
- Tests pass
- Follow Rust style guidelines
- Update documentation as needed
