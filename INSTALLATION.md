# Installation Guide

This guide covers installation methods for all supported platforms.

## Quick Start

### Linux (Debian/Ubuntu)

```bash
# Download and install the DEB package
sudo dpkg -i family-policy_0.1.0_amd64.deb

# Configure the agent
sudo family-policy setup \
  --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml \
  --token YOUR_GITHUB_TOKEN

# Install and start as a service
sudo family-policy install-service
sudo family-policy start

# Check status
sudo family-policy status
```

### Linux (Fedora/RHEL)

```bash
# Download and install the RPM package
sudo rpm -i family-policy-0.1.0-1.x86_64.rpm

# Configure the agent
sudo family-policy setup \
  --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml \
  --token YOUR_GITHUB_TOKEN

# Install and start as a service
sudo family-policy install-service
sudo family-policy start

# Check status
sudo family-policy status
```

### macOS

```bash
# Install the PKG package
sudo installer -pkg family-policy-0.1.0.pkg -target /

# Configure the agent
sudo family-policy setup \
  --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml \
  --token YOUR_GITHUB_TOKEN

# Install as a LaunchDaemon (will auto-start on boot)
sudo family-policy install-service

# Check status
sudo family-policy status
```

### Windows

```powershell
# Run PowerShell as Administrator
# Navigate to the directory containing install.ps1 and family-policy.exe
.\install.ps1

# Configure the agent
family-policy setup `
  --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml `
  --token YOUR_GITHUB_TOKEN

# Start the agent (manual mode on Windows)
family-policy start --no-daemon
```

## Installation Methods

### Method 1: Package Managers (Recommended)

Use platform-specific package managers for automatic dependency resolution and easy updates.

#### Debian/Ubuntu (.deb)

```bash
# Install
sudo dpkg -i family-policy_VERSION_amd64.deb

# Uninstall
sudo dpkg -r family-policy

# Purge (removes configuration too)
sudo dpkg -P family-policy
```

#### Fedora/RHEL (.rpm)

```bash
# Install
sudo rpm -i family-policy-VERSION.x86_64.rpm

# Upgrade
sudo rpm -U family-policy-VERSION.x86_64.rpm

# Uninstall
sudo rpm -e family-policy
```

#### macOS (.pkg)

```bash
# Install
sudo installer -pkg family-policy-VERSION.pkg -target /

# Uninstall (use the provided script)
sudo /usr/local/bin/family-policy-uninstall.sh
```

### Method 2: Manual Installation

If packages are not available for your platform, install manually using the provided scripts.

#### Linux

```bash
# Clone or download the repository
git clone https://github.com/username/family-policy.git
cd family-policy

# Build from source
cargo build --release

# Run installation script
cd packaging/linux
sudo ./install.sh
```

#### macOS

```bash
# Clone or download the repository
git clone https://github.com/username/family-policy.git
cd family-policy

# Build from source
cargo build --release

# Run installation script
cd packaging/macos
sudo ./install.sh
```

#### Windows

```powershell
# Clone or download the repository
git clone https://github.com/username/family-policy.git
cd family-policy

# Build from source
cargo build --release

# Run installation script (as Administrator)
cd packaging\windows
.\install.ps1
```

### Method 3: Build from Source

For development or custom builds:

```bash
# Prerequisites
# - Rust toolchain (https://rustup.rs)
# - Git

# Clone repository
git clone https://github.com/username/family-policy.git
cd family-policy

# Build
cargo build --release

# The binary will be at: target/release/family-policy
# Copy to your desired location
sudo cp target/release/family-policy /usr/local/bin/

# Manually install service files (see packaging directory)
```

## Post-Installation Setup

### 1. Create GitHub Repository for Policies

Create a GitHub repository to store your policy files:

```bash
mkdir family-policies
cd family-policies

# Create a policy file
cat > policy.yaml << EOF
policies:
  - name: Standard filtering
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
    disable_private_mode: true
EOF

# Initialize git and push
git init
git add policy.yaml
git commit -m "Initial policy"
git remote add origin https://github.com/USER/REPO.git
git push -u origin main
```

### 2. Create GitHub Personal Access Token

1. Go to https://github.com/settings/tokens/new
2. Note: "Family Policy Agent - Read Only"
3. Expiration: 1 year or no expiration
4. Scopes:
   - For **private repos**: Select `repo` (Full control)
   - For **public repos**: No scopes needed (or select `public_repo`)
5. Click "Generate token"
6. **Save the token** - you won't see it again!

### 3. Configure the Agent

```bash
sudo family-policy setup \
  --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml \
  --token ghp_YOUR_TOKEN_HERE \
  --poll-interval 300
```

Configuration is saved to:
- Linux: `/etc/family-policy/agent.conf`
- macOS: `/Library/Application Support/family-policy/agent.conf`
- Windows: `C:\ProgramData\family-policy\agent.conf`

### 4. Install and Start Service

#### Linux (systemd)

```bash
# Enable service to start on boot
sudo family-policy install-service

# Start the service
sudo family-policy start

# Check status
sudo systemctl status family-policy-agent
sudo family-policy status

# View logs
sudo journalctl -u family-policy-agent -f
```

#### macOS (LaunchDaemon)

```bash
# Install LaunchDaemon (auto-starts on boot)
sudo family-policy install-service

# Check status
sudo launchctl list | grep family-policy
sudo family-policy status

# View logs
tail -f /var/log/family-policy-agent.log
```

#### Windows (Manual)

Windows doesn't support automatic service installation yet. Run manually:

```powershell
# Run in foreground (for testing)
family-policy start --no-daemon

# Or create a scheduled task to run at startup
# (Use Task Scheduler GUI or PowerShell)
```

## Verification

After installation, verify everything is working:

```bash
# Check agent status
sudo family-policy status

# Force an immediate policy check
sudo family-policy check-now

# View applied configuration
sudo family-policy show-config
```

Expected output:
- Last checked: Recent timestamp
- Last updated: Timestamp when policy was applied
- Applied configuration showing your extensions and privacy settings

## Updating

### Update the Binary

#### Debian/Ubuntu

```bash
sudo dpkg -i family-policy_NEW_VERSION_amd64.deb
sudo systemctl restart family-policy-agent
```

#### Fedora/RHEL

```bash
sudo rpm -U family-policy-NEW_VERSION.x86_64.rpm
sudo systemctl restart family-policy-agent
```

#### macOS

```bash
sudo installer -pkg family-policy-NEW_VERSION.pkg -target /
sudo launchctl unload /Library/LaunchDaemons/com.family-policy.agent.plist
sudo launchctl load /Library/LaunchDaemons/com.family-policy.agent.plist
```

### Update Policies

Simply edit and push your policy file to GitHub:

```bash
cd family-policies
nano policy.yaml  # Make changes
git commit -am "Update policies"
git push
```

The agent will detect changes within the polling interval (default: 5 minutes).

## Uninstallation

### Complete Removal

#### Debian/Ubuntu

```bash
# Stop and remove service
sudo family-policy stop
sudo family-policy uninstall-service

# Remove package (keeps config)
sudo dpkg -r family-policy

# Or remove everything including config
sudo dpkg -P family-policy
sudo rm -rf /etc/family-policy
sudo rm -rf /var/lib/browser-extension-policy
```

#### Fedora/RHEL

```bash
# Stop and remove service
sudo family-policy stop
sudo family-policy uninstall-service

# Remove package
sudo rpm -e family-policy

# Remove config and state
sudo rm -rf /etc/family-policy
sudo rm -rf /var/lib/browser-extension-policy
```

#### macOS

```bash
# Use the uninstall script
sudo packaging/macos/uninstall.sh

# Or manually:
sudo family-policy stop
sudo family-policy uninstall-service
sudo rm /usr/local/bin/family-policy
sudo rm -rf "/Library/Application Support/family-policy"
sudo rm -rf "/Library/Application Support/browser-extension-policy"
```

#### Windows

```powershell
# Run as Administrator
.\uninstall.ps1
```

### Remove Policies Only (Keep Agent)

To remove browser policies but keep the agent installed:

```bash
sudo family-policy --uninstall
```

This removes all applied browser policies but keeps the agent service running.

## Troubleshooting

### Agent Not Starting

**Linux:**
```bash
sudo systemctl status family-policy-agent
sudo journalctl -u family-policy-agent -n 50
```

**macOS:**
```bash
sudo launchctl list | grep family-policy
tail -n 50 /var/log/family-policy-agent.log
```

Common issues:
- Configuration file missing or invalid
- GitHub URL unreachable
- Invalid access token
- Network connectivity issues

### Policies Not Applying

1. Check agent status:
   ```bash
   sudo family-policy status
   ```

2. Force immediate check:
   ```bash
   sudo family-policy check-now
   ```

3. Verify policy file is valid YAML:
   ```bash
   # Download and validate
   curl https://raw.githubusercontent.com/USER/REPO/main/policy.yaml
   ```

4. Check logs for errors (see above)

### Permission Errors

Ensure the agent is running with root/administrator privileges:

```bash
# Linux/macOS
sudo family-policy start

# Windows
# Run PowerShell as Administrator
```

### GitHub Authentication Errors

- Verify token has not expired
- For private repos, ensure token has `repo` scope
- For public repos, try without a token or with `public_repo` scope

## Advanced Configuration

### Custom Polling Interval

```bash
sudo family-policy setup \
  --url YOUR_URL \
  --token YOUR_TOKEN \
  --poll-interval 600  # Check every 10 minutes
```

### Multiple Machines

Install on multiple computers using the same GitHub repository but different policy files:

```
family-policies/
├── kids-pc.yaml
├── living-room-mac.yaml
└── basement-linux.yaml
```

On each machine, point to the specific policy file:

```bash
# Kids PC
sudo family-policy setup \
  --url https://raw.githubusercontent.com/USER/REPO/main/kids-pc.yaml

# Living Room Mac
sudo family-policy setup \
  --url https://raw.githubusercontent.com/USER/REPO/main/living-room-mac.yaml
```

## Getting Help

- **Documentation**: See README.md and CLAUDE.md
- **GitHub Issues**: https://github.com/username/family-policy/issues
- **Logs**: Check platform-specific log locations above
