# Windows Installation Options

Family Policy Agent provides two installation methods for Windows:

## Option 1: MSI Installer (Recommended)

**Best for:** Most users who want a standard Windows installation experience.

### Features:
- ✅ Double-click installation
- ✅ Automatic PATH configuration
- ✅ Creates necessary directories
- ✅ Appears in "Add/Remove Programs"
- ✅ Clean uninstallation through Windows Settings
- ✅ Support for upgrades

### Installation:
1. Download `family-policy-0.1.0-x86_64.msi` from the release
2. Double-click the MSI file
3. Follow the installation wizard
4. Open a new Command Prompt or PowerShell window
5. Verify installation: `family-policy --version`

### Post-Installation:
```powershell
# Create a configuration file
family-policy config init

# Edit the config file, then apply it
family-policy --config family-policy.yaml

# Install as Windows Service (optional, recommended for automatic startup)
family-policy install-service

# Start the agent
family-policy start              # Starts as service
# OR
family-policy start --no-daemon  # Runs in foreground
```

### Uninstallation:
- Open Windows Settings → Apps → Installed apps
- Find "Family Policy Agent" and click Uninstall

## Option 2: Manual Installation (ZIP)

**Best for:** Users who need portable installation or advanced control.

### Features:
- ✅ No installer required
- ✅ Portable - can run from any location
- ✅ Manual PATH configuration
- ⚠️ Requires PowerShell script execution

### Installation:
1. Download `family-policy-windows-x86_64.zip` from the release
2. Extract to a location of your choice
3. Open PowerShell as Administrator
4. Navigate to the extracted directory
5. Run: `.\install.ps1`

### Uninstallation:
1. Open PowerShell as Administrator
2. Navigate to the installation directory
3. Run: `.\uninstall.ps1`

## Service Management

Both installation methods support running the agent as a Windows Service:

```powershell
# Install as service
family-policy install-service

# Start the service
family-policy start

# Check status
family-policy status

# Stop the service
family-policy stop

# Uninstall service
family-policy uninstall-service
```

## Building the MSI (for developers)

Prerequisites:
- Rust toolchain
- WiX Toolset v3.14+
- cargo-wix (`cargo install cargo-wix`)

Build command:
```powershell
cargo wix --target x86_64-pc-windows-msvc
```

The MSI will be created in `target/wix/family-policy-0.1.0-x86_64.msi`

## Configuration Locations

- **Config**: `C:\ProgramData\family-policy\`
- **State**: `C:\ProgramData\browser-extension-policy\`
- **Binary**: `C:\Program Files\FamilyPolicy\family-policy.exe` (MSI install)
