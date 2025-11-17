# Architecture Documentation

This document explains how the Family Policy system works, including the authentication model, process architecture, and relationship between components.

## System Components

### 1. CLI Agent (`family-policy`)
- **Location**: Main Rust binary in repository root
- **Purpose**: Background daemon that polls GitHub for policy updates and applies them
- **Runs as**: System service/daemon with root/administrator privileges
- **Process**: Long-running background process

### 2. UI Application (`family-policy-ui`)
- **Location**: `family-policy-ui/` subdirectory
- **Purpose**: System tray application for managing agent configuration
- **Runs as**: Desktop application launched by user
- **Process**: Separate process from the agent

## Authentication & Privilege Model

### How Privileges Work

The UI application uses **process-level privilege checking**:

```rust
// Checks if the CURRENT PROCESS is running with elevated privileges
pub fn is_admin() -> bool {
    #[cfg(unix)]
    unsafe { libc::geteuid() == 0 }  // Check if UID is 0 (root)

    #[cfg(windows)]
    // Check if process token is elevated
    GetTokenInformation(token, TokenElevation, ...)
}
```

### Launch Requirements

**The entire UI application must be launched with elevated privileges:**

- **Linux/macOS**: `sudo family-policy-ui`
- **Windows**: Right-click → "Run as Administrator"

### What Happens Without Privileges?

When launched without admin rights:
1. UI loads normally and shows current config
2. `check_admin_privileges()` returns `false`
3. Warning banner appears: "⚠️ Administrator privileges required to save changes"
4. Save button is disabled
5. User can view settings but cannot modify them

### Why This Approach?

The config file is stored in a protected system location:
- **Linux**: `/etc/family-policy/agent.conf` (requires root to write)
- **macOS**: `/Library/Application Support/family-policy/agent.conf` (requires root)
- **Windows**: `C:\ProgramData\family-policy\agent.conf` (requires admin)

The UI process needs write access to these locations, which requires running the entire process elevated.

## Process Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Desktop Session (User runs this)                       │
│                                                         │
│  ┌───────────────────────────────────────────────┐    │
│  │  family-policy-ui (Tauri Process)             │    │
│  │                                                │    │
│  │  ├─ System Tray Icon                          │    │
│  │  └─ Settings Window (hidden by default)       │    │
│  │                                                │    │
│  │  Reads/Writes:                                 │    │
│  │  /etc/family-policy/agent.conf                │    │
│  └───────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                         ↓
                  Config File
                         ↓
┌─────────────────────────────────────────────────────────┐
│  System Service (Runs in background)                    │
│                                                         │
│  ┌───────────────────────────────────────────────┐    │
│  │  family-policy agent (CLI Process)            │    │
│  │                                                │    │
│  │  ├─ Polls GitHub for policy updates           │    │
│  │  ├─ Applies browser policies                  │    │
│  │  └─ Manages extensions                        │    │
│  │                                                │    │
│  │  Reads:                                        │    │
│  │  /etc/family-policy/agent.conf                │    │
│  └───────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### Key Points

1. **Two Separate Processes**:
   - UI and agent are independent processes
   - They don't communicate directly
   - Communication happens via the config file

2. **Single Process Components**:
   - The tray icon and settings window are **NOT** separate processes
   - They're both part of the same Tauri application process
   - Settings window is just hidden/shown within the same process

3. **Lifecycle**:
   - **Agent**: Runs continuously as system service
   - **UI**: Launched on-demand by user when they want to change settings

## Configuration Flow

```
User clicks "Settings" in tray
         ↓
Settings window shown (same process)
         ↓
User modifies configuration
         ↓
User clicks "Save"
         ↓
UI checks: is_admin()?
    ├─ NO → Show error, disable save
    └─ YES → Continue
         ↓
Write to /etc/family-policy/agent.conf
         ↓
Agent detects file change (on next poll cycle)
         ↓
Agent reloads configuration
         ↓
New settings take effect
```

## Security Considerations

### File Permissions

When the UI saves config, it sets restrictive permissions:
```rust
#[cfg(unix)]
{
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o600);  // Only owner can read/write
    fs::set_permissions(&path, perms)?;
}
```

### Validation

The UI validates settings before saving:
- Policy URL must use HTTPS
- Poll interval must be ≥ 60 seconds
- Numeric values must be within valid ranges

### Token Storage

GitHub access tokens are:
- Stored in the config file with 0600 permissions
- Not displayed in UI (password field type)
- Optional (only needed for private repos)

## Building & Distribution

### CLI Agent
Built via standard Cargo:
- Single binary per platform
- Installed to system location (e.g., `/usr/local/bin`)
- Typically run as system service

### UI Application
Built via Tauri:
- **Linux**: .deb package, AppImage
- **macOS**: .dmg, .app bundle
- **Windows**: MSI installer, NSIS installer

Both are built in the same GitHub Actions workflow but produce separate artifacts.

## Installation Scenarios

### Scenario 1: Server (Headless)
- Install CLI agent only
- Configure via editing `/etc/family-policy/agent.conf` directly
- No UI needed

### Scenario 2: Desktop (with UI)
- Install both CLI agent and UI application
- Agent runs as service in background
- User launches UI when they need to change settings
- UI provides friendly interface for config management

### Scenario 3: Development/Testing
- Run agent with `--config` flag for one-off policy application
- Or use UI to manage agent.conf, then run agent manually

## Future Enhancements

Potential improvements to consider:

1. **Privilege Elevation in UI**:
   - Could use OS-specific elevation prompts (sudo, UAC)
   - Would require platform-specific integration

2. **IPC Between UI and Agent**:
   - Could add direct communication channel
   - Would enable live config reload without polling

3. **User-Level Config**:
   - Could support per-user config files
   - Would eliminate need for admin privileges

4. **Config Validation Service**:
   - Could validate GitHub policy URL before saving
   - Would catch errors earlier in the process
