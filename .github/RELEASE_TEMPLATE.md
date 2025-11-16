# Release Notes Template

Use this template when creating a new release.

## Release v0.1.0

### New Features
- GitHub-based remote policy management via polling agent
- Support for Chrome, Firefox, and Edge browsers
- Multi-browser policy configuration format
- Privacy controls (disable private/incognito modes, guest mode)
- Platform-specific installers (MSI, PKG)
- Service/daemon installation for automatic startup
- ETag-based efficient policy updates

### Improvements
- Idempotent policy application with hash-based change detection
- Atomic file operations prevent corruption
- Comprehensive error handling and logging
- Security hardening in service files

### Bug Fixes
- None (initial release)

### Breaking Changes
- None (initial release)

### Installation

#### Linux
```bash
wget https://github.com/USERNAME/family-policy/releases/download/v0.1.0/family-policy-linux-x86_64
chmod +x family-policy-linux-x86_64
sudo mv family-policy-linux-x86_64 /usr/local/bin/family-policy
```

#### macOS
```bash
wget https://github.com/USERNAME/family-policy/releases/download/v0.1.0/family-policy-0.1.0.pkg
sudo installer -pkg family-policy-0.1.0.pkg -target /
```

#### Windows
**Option 1: MSI Installer (Recommended)**
1. Download `family-policy-0.1.0-x86_64.msi`
2. Double-click to install

**Option 2: Manual Install**
1. Download `family-policy-windows-x86_64.zip`
2. Extract to a folder
3. Run PowerShell as Administrator
4. Navigate to the extracted folder
5. Run `.\install.ps1`

### Quick Start

After installation:

```bash
# Configure the agent
sudo family-policy agent setup \
  --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml \
  --token YOUR_GITHUB_TOKEN

# Install and start service
sudo family-policy agent install
sudo family-policy agent start

# Check status
sudo family-policy agent status
```

See [INSTALLATION.md](INSTALLATION.md) for detailed setup instructions.

### What's Changed
- Initial release of Family Policy
- Full documentation available in README.md and CLAUDE.md

### Checksums (SHA-256)
See attached `SHA256SUMS` file for binary checksums.

### Assets
- `family-policy-linux-x86_64` - Linux binary (standalone)
- `family-policy-macos-universal` - macOS binary (Universal - Intel + Apple Silicon)
- `family-policy-0.1.0.pkg` - macOS installer package
- `family-policy-windows-x86_64.exe` - Windows binary (standalone)
- `family-policy-0.1.0-x86_64.msi` - Windows MSI installer (recommended)
- `family-policy-windows-x86_64.zip` - Windows manual install package
- `SHA256SUMS` - Checksums for verification

### Known Issues
- Some antivirus software may flag the binary. This is a false positive (unsigned binary).

### Documentation
- [README.md](README.md) - Overview and features
- [INSTALLATION.md](INSTALLATION.md) - Detailed installation guide
- [CLAUDE.md](CLAUDE.md) - Development guide
- [BUILD.md](BUILD.md) - Building from source
- [packaging/README.md](packaging/README.md) - Creating packages

### Contributors
- @username - Initial implementation

### Support
- Report issues: https://github.com/USERNAME/family-policy/issues
- Documentation: https://github.com/USERNAME/family-policy/wiki (if available)

---

**Full Changelog**: https://github.com/USERNAME/family-policy/commits/v0.1.0
