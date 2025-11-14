# Packaging Guide

This directory contains packaging configurations and build scripts for creating platform-specific installers.

## Prerequisites

### All Platforms
- Rust toolchain (for building the binary)
- Git (for version control)

### Debian/Ubuntu (DEB)
```bash
sudo apt-get install dpkg-dev
```

### Fedora/RHEL (RPM)
```bash
sudo dnf install rpm-build
```

### macOS (PKG)
- Xcode Command Line Tools
- macOS 10.15 or later

### Windows (Installer)
- PowerShell 5.1 or later
- Administrator privileges

## Building Packages

### Debian Package (.deb)

Build the DEB package for Debian/Ubuntu systems:

```bash
cd packaging/debian
./build-deb.sh
```

Output: `dist/family-policy_0.1.0_amd64.deb`

**Install:**
```bash
sudo dpkg -i dist/family-policy_0.1.0_amd64.deb
```

**Remove:**
```bash
sudo dpkg -r family-policy
```

### RPM Package (.rpm)

Build the RPM package for Fedora/RHEL systems:

```bash
cd packaging/rpm
./build-rpm.sh
```

Output: `dist/family-policy-0.1.0-1.x86_64.rpm`

**Install:**
```bash
sudo rpm -i dist/family-policy-0.1.0-1.x86_64.rpm
```

**Remove:**
```bash
sudo rpm -e family-policy
```

### macOS Package (.pkg)

Build the PKG installer for macOS (must be run on macOS):

```bash
cd packaging/macos
./build-pkg.sh
```

Output: `dist/family-policy-0.1.0.pkg`

**Install:**
```bash
sudo installer -pkg dist/family-policy-0.1.0.pkg -target /
```

**Remove:**
```bash
sudo packaging/macos/uninstall.sh
```

### Windows Installation

For Windows, use the PowerShell installation script:

1. Build the binary for Windows (on Windows or cross-compile)
2. Copy `family-policy.exe` to the `packaging/windows` directory
3. Run as Administrator:

```powershell
cd packaging\windows
.\install.ps1
```

**Uninstall:**
```powershell
.\uninstall.ps1
```

## Package Contents

All packages include:

1. **Binary**: `/usr/local/bin/family-policy` (or `C:\Program Files\FamilyPolicy\family-policy.exe` on Windows)
2. **Service Files**:
   - Linux: `/etc/systemd/system/family-policy-agent.service`
   - macOS: `/Library/LaunchDaemons/com.family-policy.agent.plist`
   - Windows: Task Scheduler integration (manual)
3. **Configuration Directory**:
   - Linux: `/etc/family-policy/`
   - macOS: `/Library/Application Support/family-policy/`
   - Windows: `C:\ProgramData\family-policy\`
4. **State Directory**:
   - Linux: `/var/lib/browser-extension-policy/`
   - macOS: `/Library/Application Support/browser-extension-policy/`
   - Windows: `C:\ProgramData\browser-extension-policy\`

## Post-Installation

After installing the package, configure the agent:

```bash
# Configure agent
sudo family-policy agent setup \
  --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml \
  --token YOUR_GITHUB_TOKEN

# Install and start service
sudo family-policy agent install
sudo family-policy agent start

# Check status
sudo family-policy agent status
```

## Directory Structure

```
packaging/
├── README.md                    # This file
├── linux/
│   ├── family-policy-agent.service  # Systemd service file
│   ├── install.sh               # Manual installation script
│   └── uninstall.sh             # Manual uninstallation script
├── debian/
│   ├── DEBIAN/
│   │   ├── control              # Package metadata
│   │   ├── postinst             # Post-installation script
│   │   ├── prerm                # Pre-removal script
│   │   └── postrm               # Post-removal script
│   └── build-deb.sh             # DEB package build script
├── rpm/
│   ├── family-policy.spec       # RPM spec file
│   └── build-rpm.sh             # RPM package build script
├── macos/
│   ├── com.family-policy.agent.plist  # LaunchDaemon plist
│   ├── install.sh               # Manual installation script
│   ├── uninstall.sh             # Manual uninstallation script
│   └── build-pkg.sh             # PKG installer build script
└── windows/
    ├── install.ps1              # Installation script
    └── uninstall.ps1            # Uninstallation script
```

## Cross-Platform Builds

### Building for Different Architectures

Use Rust's cross-compilation support:

```bash
# Install cross-compilation toolchain
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-pc-windows-gnu

# Build for specific target
cargo build --release --target x86_64-unknown-linux-gnu
```

### Using Docker for Linux Builds

Build DEB and RPM packages in Docker containers to avoid installing build dependencies:

```bash
# Debian/Ubuntu
docker run --rm -v $(pwd):/workspace -w /workspace \
  ubuntu:22.04 \
  bash -c "apt-get update && apt-get install -y dpkg-dev && packaging/debian/build-deb.sh"

# Fedora
docker run --rm -v $(pwd):/workspace -w /workspace \
  fedora:latest \
  bash -c "dnf install -y rpm-build && packaging/rpm/build-rpm.sh"
```

## Troubleshooting

### DEB Package Build Fails

**Issue**: `dpkg-deb: command not found`

**Solution**: Install dpkg-dev:
```bash
sudo apt-get install dpkg-dev
```

### RPM Package Build Fails

**Issue**: `rpmbuild: command not found`

**Solution**: Install rpm-build:
```bash
sudo dnf install rpm-build
```

### macOS Package Build Fails

**Issue**: `pkgbuild: command not found`

**Solution**: Install Xcode Command Line Tools:
```bash
xcode-select --install
```

### Permission Denied Errors

All package build scripts and installation require appropriate permissions:

- **Linux/macOS**: Run with `sudo` or as root
- **Windows**: Run PowerShell as Administrator

## Versioning

To update the version number:

1. Update `version` in `Cargo.toml`
2. Update `VERSION` variable in build scripts:
   - `packaging/debian/build-deb.sh`
   - `packaging/rpm/build-rpm.sh`
   - `packaging/macos/build-pkg.sh`
3. Update version in `packaging/debian/DEBIAN/control`
4. Update version in `packaging/rpm/family-policy.spec`
5. Update changelog in `packaging/rpm/family-policy.spec`

## Contributing

When adding new features that require packaging changes:

1. Update the appropriate service file (systemd, LaunchDaemon, etc.)
2. Update installation scripts if new dependencies are required
3. Test on all supported platforms
4. Update this README with any new requirements
