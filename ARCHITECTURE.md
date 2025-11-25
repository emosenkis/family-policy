# Family Policy Manager - Multi-Mode Architecture

## Overview

This document describes the multi-mode architecture that separates user-facing UI, administrative functions, and policy application logic with proper privilege separation.

## Architecture Principles

1. **Privilege Separation**: User UI runs without privileges, admin operations require elevation
2. **Mode Isolation**: Each mode has a clear purpose and privilege level
3. **Shared Core**: All modes use the same policy application logic
4. **State Transparency**: State file is world-readable for status queries

## Three Operating Modes

### 1. User UI Mode (`user-ui` subcommand)

**Purpose**: User-facing interface for viewing policy status

**Privilege Level**: Regular user (no admin required)

**Capabilities**:
- Show current policy status (read state file)
- Display what policies are currently applied
- Compare current state to available policy file
- Show diff preview (dry-run internally)
- Launch Admin UI with privilege elevation
- Optionally run in system tray (flag-controlled)
- Future: more user-facing features without requiring admin

**Technology**: Tauri v2 with Vue frontend

**Key Design Points**:
- Reads world-readable state file (`/var/lib/browser-extension-policy/state.json` with 0o644)
- Can read policy files to show what would change
- Cannot modify policies or config
- Provides platform-specific elevation to launch Admin UI:
  - **Linux**: `pkexec` or `sudo`
  - **macOS**: `osascript` with administrator privileges
  - **Windows**: ShellExecute with "runas" verb (UAC prompt)

### 2. Admin UI Mode (`admin-ui` subcommand)

**Purpose**: Administrative interface for policy management

**Privilege Level**: Admin/root required (checked at startup)

**Capabilities**:
- Edit agent configuration (GitHub URL, polling interval, etc.)
- Edit local policy files
- Preview changes (dry-run internally)
- Apply policies immediately
- View detailed policy status
- Control daemon (start/stop)
- No system tray presence

**Technology**: Tauri v2 with Vue frontend (separate from User UI)

**Key Design Points**:
- Fails fast if not running as admin
- Full read/write access to config files
- Explicit "Apply" workflow:
  1. User edits config in UI
  2. UI shows preview (runs internal dry-run)
  3. User clicks "Apply"
  4. Policies are applied
  5. Success/failure feedback shown
- Can trigger daemon to reload config

### 3. Daemon/CLI Mode (existing, enhanced)

**Purpose**: Core business logic and policy application

**Privilege Level**: Depends on operation
- **Daemon mode**: Requires admin (applies policies)
- **CLI with `--dry-run`**: Regular user OK (read-only)
- **CLI without `--dry-run`**: Requires admin (applies policies)
- **Status/info commands**: Regular user OK (read-only)

**Capabilities**:

**Daemon Mode** (`daemon` or `start` subcommand):
- Poll GitHub for policy updates (agent mode)
- Automatically apply policy changes
- Write to state file
- Requires admin privileges

**CLI Mode** (default or `apply` subcommand):
- Apply policies from local file (`--config path.yaml`)
- Preview changes (`--dry-run`)
- Remove all policies (`--uninstall`)
- Regular users can run with `--dry-run` to see what would change
- Admin required for actual application

**Info Commands** (no admin required):
- `status`: Show daemon status and last update time
- `show-config`: Display currently applied configuration
- `config init`: Generate example config file

## Communication Between Components

### State File (World-Readable)

**Location** (platform-specific):
- Linux: `/var/lib/browser-extension-policy/state.json`
- macOS: `/Library/Application Support/browser-extension-policy/state.json`
- Windows: `C:\ProgramData\browser-extension-policy\state.json`

**Permissions**: `0o644` (readable by all, writable by root)

**Contents**:
```json
{
  "version": "1.0",
  "config_hash": "sha256:...",
  "last_updated": "2025-11-24T12:00:00Z",
  "applied_policies": {
    "chrome": {
      "extensions": ["id1", "id2"],
      "disable_incognito": true,
      "disable_guest_mode": true
    }
  }
}
```

**Usage**:
- **Daemon/CLI**: Writes on policy application
- **User UI**: Reads to show current status
- **Admin UI**: Reads to show current status

### User UI → Admin UI Communication

**Privilege Elevation Flow**:

1. User clicks "Settings" or "Edit Policies" in User UI
2. User UI calls platform-specific elevation:
   - **Linux**: `pkexec family-policy admin-ui` or `sudo -E family-policy admin-ui`
   - **macOS**: `osascript -e 'do shell script "family-policy admin-ui" with administrator privileges'`
   - **Windows**: `ShellExecuteW` with `lpVerb = "runas"` (triggers UAC)
3. Admin UI launches with elevated privileges
4. Admin UI runs independently (no IPC needed)
5. When admin saves changes, state file is updated
6. User UI detects changes by watching state file (or polling)

### Admin UI → Daemon Communication

**Config Change Detection**:

1. Admin UI saves new config to disk
2. Daemon (if running) has two options:
   - **Inotify/FSEvents**: Watch config file for changes
   - **Polling**: Check config hash periodically
3. Daemon detects change and reloads
4. Alternatively: Admin UI can send signal to daemon (future enhancement)

For now, we'll use the simpler approach: daemon checks config hash on each poll cycle.

## File Structure

```
src-tauri/src/
├── main.rs                 # Entry point, CLI routing
├── cli.rs                  # CLI argument parsing (clap)
├── lib.rs                  # Library interface (for Tauri - create if needed)
│
├── ui/
│   ├── mod.rs              # UI module exports
│   ├── user/               # User UI (no admin required)
│   │   ├── mod.rs          # User UI setup and window management
│   │   ├── commands.rs     # Tauri commands for user UI
│   │   └── elevation.rs    # Platform-specific privilege elevation
│   │
│   └── admin/              # Admin UI (requires admin)
│       ├── mod.rs          # Admin UI setup and window management
│       ├── commands.rs     # Tauri commands for admin UI
│       └── config_editor.rs # Config file editing logic
│
├── commands/               # CLI command implementations
│   ├── mod.rs
│   ├── agent.rs           # Agent/daemon commands
│   ├── local.rs           # Local mode (apply/uninstall)
│   ├── config.rs          # Config management commands
│   └── utils.rs           # Shared utilities
│
├── core/                   # Core business logic (new module)
│   ├── mod.rs
│   ├── apply.rs           # Policy application orchestration
│   ├── diff.rs            # Generate diffs for dry-run
│   └── privileges.rs      # Privilege checking utilities
│
├── config.rs              # Config parsing and validation
├── state.rs               # State management
├── browser.rs             # Browser detection
│
├── policy/                # Browser-specific policy writers
│   ├── mod.rs
│   ├── chrome.rs
│   ├── firefox.rs
│   ├── edge.rs
│   └── chromium_common.rs
│
├── platform/              # OS-specific implementations
│   ├── mod.rs
│   ├── common.rs
│   ├── linux.rs
│   ├── macos.rs
│   └── windows.rs
│
└── agent/                 # Agent mode (GitHub polling)
    ├── mod.rs
    ├── daemon.rs
    ├── poller.rs
    ├── scheduler.rs
    ├── config.rs
    └── state.rs
```

## CLI Command Structure

```bash
family-policy [FLAGS] [SUBCOMMAND]

FLAGS (global):
  --config <PATH>       Path to policy config file
  --dry-run            Show changes without applying (allows regular user)
  --verbose            Enable verbose logging

SUBCOMMANDS:
  # Default/Local Mode
  apply                Apply policies from local config file (default if no subcommand)

  # Config Management
  config init          Generate example config file

  # Agent/Daemon Mode
  daemon               Run as daemon (foreground mode)
  start                Start agent daemon (background)
  stop                 Stop agent daemon
  check-now            Force immediate policy check
  status               Show agent status (no admin required)
  show-config          Show currently applied config (no admin required)

  # UI Modes
  user-ui [FLAGS]      Launch User UI (no admin required)
    --systray          Run in system tray mode
    --window           Run in window mode (default)

  admin-ui             Launch Admin UI (requires admin)

  # Service Management (handled by installer, not app)
  install-service      Install system service (Linux/macOS/Windows)
  uninstall-service    Uninstall system service
```

## Privilege Checking Strategy

### New `src/core/privileges.rs` Module

```rust
pub enum PrivilegeLevel {
    User,      // Regular user, can read state
    Admin,     // Admin/root, can modify policies
}

pub struct PrivilegeCheck {
    required: PrivilegeLevel,
    allow_dry_run: bool,  // If true, user can run with --dry-run
}

pub fn check_privileges(check: PrivilegeCheck, is_dry_run: bool) -> Result<()> {
    match check.required {
        PrivilegeLevel::User => Ok(()),  // Anyone can run
        PrivilegeLevel::Admin => {
            if is_dry_run && check.allow_dry_run {
                Ok(())  // Regular user can dry-run
            } else if is_admin() {
                Ok(())
            } else {
                Err(anyhow!("This operation requires administrator privileges"))
            }
        }
    }
}

pub fn is_admin() -> bool {
    // Move existing is_admin() logic here from ui/config_bridge.rs
}
```

### Privilege Requirements by Command

| Command | Required Level | Allow Dry-Run |
|---------|---------------|---------------|
| `apply` | Admin | Yes (user can preview) |
| `config init` | User | N/A |
| `daemon` | Admin | No |
| `start` | Admin | No |
| `stop` | Admin | No |
| `check-now` | Admin | Yes (user can preview) |
| `status` | User | N/A |
| `show-config` | User | N/A |
| `user-ui` | User | N/A |
| `admin-ui` | Admin | No |
| `install-service` | Admin | No |
| `uninstall-service` | Admin | No |

## Tauri Application Structure

### User UI Window (`user-ui` mode)

**Window Configuration**:
```rust
WebviewWindow::builder("user-main", Url::parse("tauri://localhost/user")?)
    .title("Family Policy - Status")
    .inner_size(800.0, 600.0)
    .resizable(true)
    .build()?
```

**Tauri Commands**:
```rust
#[tauri::command]
async fn get_current_status() -> Result<StatusInfo, String>

#[tauri::command]
async fn get_policy_diff(policy_path: String) -> Result<PolicyDiff, String>

#[tauri::command]
async fn launch_admin_ui() -> Result<(), String>  // Triggers privilege elevation
```

### Admin UI Window (`admin-ui` mode)

**Window Configuration**:
```rust
WebviewWindow::builder("admin-main", Url::parse("tauri://localhost/admin")?)
    .title("Family Policy - Admin Settings")
    .inner_size(1000.0, 700.0)
    .resizable(true)
    .build()?
```

**Privilege Check**: Fails at startup if not admin

**Tauri Commands**:
```rust
#[tauri::command]
async fn get_agent_config() -> Result<AgentConfig, String>

#[tauri::command]
async fn save_agent_config(config: AgentConfig) -> Result<(), String>

#[tauri::command]
async fn get_policy_config(path: String) -> Result<PolicyConfig, String>

#[tauri::command]
async fn save_policy_config(path: String, config: PolicyConfig) -> Result<(), String>

#[tauri::command]
async fn preview_policy_changes(config: PolicyConfig) -> Result<PolicyDiff, String>

#[tauri::command]
async fn apply_policies(config: PolicyConfig) -> Result<ApplyResult, String>

#[tauri::command]
async fn control_daemon(action: DaemonAction) -> Result<(), String>  // start/stop/restart
```

## State File Permissions

### Current Behavior
State file is written with restrictive permissions (0o600 on Unix)

### New Behavior
State file should be world-readable:

```rust
// In src/state.rs, after writing state file:
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o644);  // Changed from 0o600
    fs::set_permissions(&path, perms)?;
}
```

This allows:
- Regular users to read state for status display
- Only root/admin can write (directory permissions)

## Platform-Specific Elevation

### Linux

**Method**: `pkexec` (PolicyKit) or fallback to `sudo`

```rust
#[cfg(target_os = "linux")]
pub fn launch_admin_ui() -> Result<()> {
    let exe_path = env::current_exe()?;

    // Try pkexec first (graphical, better UX)
    if Command::new("pkexec")
        .arg(&exe_path)
        .arg("admin-ui")
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    // Fallback to sudo in terminal
    Command::new("x-terminal-emulator")
        .arg("-e")
        .arg(format!("sudo {} admin-ui", exe_path.display()))
        .spawn()?;

    Ok(())
}
```

### macOS

**Method**: `osascript` with AppleScript

```rust
#[cfg(target_os = "macos")]
pub fn launch_admin_ui() -> Result<()> {
    let exe_path = env::current_exe()?;

    let script = format!(
        r#"do shell script "{} admin-ui" with administrator privileges"#,
        exe_path.display()
    );

    Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn()?;

    Ok(())
}
```

### Windows

**Method**: ShellExecute with "runas" (UAC)

```rust
#[cfg(target_os = "windows")]
pub fn launch_admin_ui() -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    let exe_path = env::current_exe()?;
    let operation = "runas".encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let file = exe_path.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let params = "admin-ui".encode_utf16().chain(Some(0)).collect::<Vec<_>>();

    unsafe {
        ShellExecuteW(
            0,
            operation.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            1,  // SW_SHOWNORMAL
        );
    }

    Ok(())
}
```

## System Service Installation

**Note**: Service installation is handled by the installer package, not by the app itself.

### Linux (systemd)

**File**: `/etc/systemd/system/family-policy.service`

```ini
[Unit]
Description=Family Policy Manager Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/family-policy daemon
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

**Install Commands** (in installer script):
```bash
systemctl daemon-reload
systemctl enable family-policy.service
systemctl start family-policy.service
```

### macOS (launchd)

**File**: `/Library/LaunchDaemons/com.family-policy.daemon.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.family-policy.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/family-policy</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

**Install Commands** (in installer script):
```bash
launchctl load /Library/LaunchDaemons/com.family-policy.daemon.plist
```

### Windows (Windows Service)

**Implementation**: Use `windows-service` crate

**Install**: Done by installer (MSI or NSIS)

## Migration from Current Implementation

### Phase 1: Core Refactoring
1. Create `src/core/` module with privilege checking
2. Move policy application logic to `src/core/apply.rs`
3. Add `src/core/diff.rs` for dry-run diff generation
4. Update state file permissions to 0o644

### Phase 2: CLI Enhancement
1. Update `src/cli.rs` to add `user-ui` and `admin-ui` subcommands
2. Add `--systray` flag for user-ui
3. Implement privilege checking in CLI routing

### Phase 3: User UI
1. Create `src/ui/user/` module
2. Implement Tauri commands for status display
3. Implement platform-specific elevation in `src/ui/user/elevation.rs`
4. Create Vue components for user UI

### Phase 4: Admin UI
1. Create `src/ui/admin/` module
2. Implement Tauri commands for config editing
3. Create Vue components for admin UI
4. Implement preview and apply workflow

### Phase 5: Testing & Polish
1. Test privilege elevation on all platforms
2. Test user UI can read state
3. Test admin UI can edit and apply
4. Test daemon integration
5. Update documentation

## Security Considerations

1. **Privilege Separation**: User UI cannot modify system policies
2. **Elevation Audit**: All privilege elevations go through platform APIs
3. **State File Integrity**: Only admin can write state file (directory permissions)
4. **Config Validation**: Admin UI validates config before saving
5. **No Privilege Escalation**: User UI never tries to write policies directly

## Future Enhancements

1. **Real-time Updates**: User UI watches state file with inotify/FSEvents
2. **IPC for Daemon Control**: User UI can request daemon status via socket
3. **Notification System**: User UI shows notifications on policy changes
4. **Multi-User Support**: Per-user policies in addition to system-wide
5. **Remote Management**: Admin UI can manage multiple machines
