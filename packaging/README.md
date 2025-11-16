# Packaging Guide

This directory contains packaging configurations and build scripts for creating platform-specific installers.

## Supported Packages

| Platform | Package Type | Installer | Description |
|----------|--------------|-----------|-------------|
| Windows | MSI | ✅ | Windows Installer package |
| macOS | PKG | ✅ | macOS installer package |
| Linux | Binary | ✅ | Raw executable binary |

## Prerequisites

### All Platforms
- Rust toolchain (for building the binary)
- Git (for version control)

### Windows (MSI)
- WiX Toolset v3.14+
- cargo-wix (`cargo install cargo-wix`)

### macOS (PKG)
- Xcode Command Line Tools
- macOS 10.15 or later

### Linux
- No special requirements (ships raw binary)

## Building Packages

### Windows MSI Package

Build the MSI installer for Windows:

```powershell
# Install cargo-wix if not already installed
cargo install cargo-wix

# Build the MSI
cargo wix --target x86_64-pc-windows-msvc
```

**Output:** `target/wix/family-policy-0.1.0-x86_64.msi`

**Install:**
- Double-click the MSI file, or:
```powershell
msiexec /i target\wix\family-policy-0.1.0-x86_64.msi
```

**Uninstall:**
- Via Windows Settings → Apps, or:
```powershell
msiexec /x target\wix\family-policy-0.1.0-x86_64.msi
```

See `packaging/windows/README.md` for detailed Windows installation options.

### macOS PKG Package

Build the PKG installer for macOS (must be run on macOS):

```bash
cd packaging/macos
./build-pkg.sh
```

**Output:** `dist/family-policy-0.1.0.pkg`

**Install:**
```bash
sudo installer -pkg dist/family-policy-0.1.0.pkg -target /
```

**Uninstall:**
```bash
sudo /usr/local/bin/family-policy-uninstall.sh
```

### Linux Binary

Linux ships as a raw binary:

```bash
# Build the release binary
cargo build --release --target x86_64-unknown-linux-gnu

# The binary is located at:
# target/x86_64-unknown-linux-gnu/release/family-policy
```

**Install:**
```bash
sudo cp target/x86_64-unknown-linux-gnu/release/family-policy /usr/local/bin/
sudo chmod 755 /usr/local/bin/family-policy
```

**Uninstall:**
```bash
sudo rm /usr/local/bin/family-policy
sudo family-policy uninstall-service  # if service was installed
```

## Directory Structure

```
packaging/
├── linux/
│   ├── family-policy-agent.service  # Systemd service file
│   ├── install.sh                   # Manual installation script
│   └── uninstall.sh                 # Manual uninstallation script
├── macos/
│   ├── build-pkg.sh                 # PKG build script
│   ├── com.family-policy.agent.plist # LaunchDaemon plist
│   ├── install.sh                   # Manual installation script
│   └── uninstall.sh                 # Manual uninstallation script
├── windows/
│   ├── README.md                    # Windows installation guide
│   ├── install.ps1                  # Manual installation script (ZIP)
│   └── uninstall.ps1                # Manual uninstallation script (ZIP)
└── README.md                        # This file
```

## CI/CD Integration

The GitHub Actions workflow (`.github/workflows/release.yml`) automatically builds packages for all platforms:

- **Windows**: Builds MSI package using cargo-wix
- **macOS**: Builds PKG installer using native tools
- **Linux**: Builds raw binary

All artifacts are uploaded to GitHub releases.

## Manual Installation Scripts

Each platform provides manual installation scripts for users who prefer not to use installers:

### Linux
```bash
cd packaging/linux
sudo ./install.sh
```

### macOS
```bash
cd packaging/macos
sudo ./install.sh
```

### Windows
```powershell
cd packaging\windows
.\install.ps1  # Run as Administrator
```

## Package Features

### Windows MSI
- ✅ Automatic PATH configuration
- ✅ Creates config directories
- ✅ Appears in "Add/Remove Programs"
- ✅ Supports upgrades
- ✅ Clean uninstallation

### macOS PKG
- ✅ Installs to `/usr/local/bin`
- ✅ Includes LaunchDaemon for service
- ✅ Creates config directories
- ✅ Postinstall configuration

### Linux Binary
- ✅ Single static binary
- ✅ No dependencies
- ✅ Systemd service file included
- ✅ Manual installation scripts

## Version Management

When releasing a new version:

1. Update version in `Cargo.toml`
2. Update version in package build scripts:
   - `packaging/macos/build-pkg.sh` (VERSION variable)
   - WiX configuration is auto-updated from `Cargo.toml`
3. Tag the release: `git tag v0.1.0`
4. Push the tag: `git push origin v0.1.0`
5. GitHub Actions will automatically build all packages

## Testing Packages

### Windows MSI
```powershell
# Install in test VM
msiexec /i family-policy-0.1.0-x86_64.msi

# Verify installation
family-policy --version

# Test service installation
family-policy install-service
family-policy start

# Uninstall
msiexec /x family-policy-0.1.0-x86_64.msi
```

### macOS PKG
```bash
# Install
sudo installer -pkg family-policy-0.1.0.pkg -target /

# Verify
family-policy --version

# Test service
sudo family-policy install-service

# Uninstall
sudo packaging/macos/uninstall.sh
```

### Linux Binary
```bash
# Install
sudo cp family-policy /usr/local/bin/

# Verify
family-policy --version

# Test service
sudo family-policy install-service
sudo systemctl start family-policy-agent

# Uninstall
sudo systemctl stop family-policy-agent
sudo systemctl disable family-policy-agent
sudo rm /usr/local/bin/family-policy
```

## Platform Comparison

| Feature | Windows (MSI) | macOS (PKG) | Linux (Binary) |
|---------|---------------|-------------|----------------|
| Installer | ✅ | ✅ | ❌ (manual) |
| Auto PATH | ✅ | ✅ | ❌ (manual) |
| GUI Uninstall | ✅ | ❌ | ❌ |
| Upgrades | ✅ | ✅ | Manual |
| Service Install | CLI | CLI | CLI |
| Config Dirs | Auto | Auto | Manual |

## Troubleshooting

### Windows MSI Build Fails

**Issue**: WiX not found

**Solution**: Install WiX Toolset and ensure it's in PATH:
```powershell
# Download from: https://github.com/wixtoolset/wix3/releases
# Or install cargo-wix which includes WiX
cargo install cargo-wix
```

### macOS PKG Build Fails

**Issue**: `pkgbuild: command not found`

**Solution**: Install Xcode Command Line Tools:
```bash
xcode-select --install
```

### Linux Binary Won't Run

**Issue**: Permission denied

**Solution**: Make binary executable:
```bash
chmod +x family-policy
```

## Additional Resources

- Windows Installation Guide: `packaging/windows/README.md`
- WiX Configuration: `wix/main.wxs`
- macOS LaunchDaemon: `packaging/macos/com.family-policy.agent.plist`
- Linux Systemd Service: `packaging/linux/family-policy-agent.service`
