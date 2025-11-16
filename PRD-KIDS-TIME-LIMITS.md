# Product Requirements Document: Kids Time Limits

**Version:** 1.0
**Date:** 2025-11-16
**Status:** Draft

## Executive Summary

This PRD defines the requirements for adding comprehensive kids time limits functionality to the family-policy application. The feature will enable parents/administrators to set daily computer usage limits for children, track usage in real-time, and enforce hard blocks when time is exhausted. The system supports both shared login scenarios (multiple kids sharing one account) and individual login scenarios (each kid has their own OS account).

## Background

### Problem Statement

Parents need a reliable way to limit their children's screen time on computers. Current solutions either:
- Require complex third-party software with privacy concerns
- Lack cross-platform support
- Don't integrate well with existing family policy management
- Can't handle multiple children sharing a single computer account

### Goals

1. **Primary Goals**
   - Provide daily time limits per child (configurable hours/minutes)
   - Track real-time usage across sessions
   - Hard block computer access when time expires
   - Support multiple children on shared or individual accounts
   - Provide admin override capabilities
   - Maintain cross-platform compatibility (Windows, macOS, Linux)

2. **Secondary Goals**
   - Intuitive GUI for configuration and monitoring
   - Schedule-based limits (weekday vs. weekend, custom schedules)
   - Grace period warnings before hard block
   - Usage reports and analytics
   - Integration with existing browser policy management

### Non-Goals (v1.0)

- Internet-only filtering (blocks entire computer, not just internet)
- Remote monitoring dashboard (web-based parent portal)
- Reward/incentive system
- App-specific time limits
- Bedtime enforcement (separate from daily limits)

## User Personas

### Primary Persona: Parent/Administrator
- Needs simple setup for multiple children
- Wants reliable enforcement without technical workarounds
- Requires visibility into usage patterns
- Values cross-platform consistency

### Secondary Persona: IT Administrator
- Manages family policies centrally via GitHub
- Needs programmatic configuration
- Requires audit trails and logging

### Tertiary Persona: Child User
- Should understand when time is running low
- Needs clear warnings before lockout
- May share login with siblings (needs identity selection)

## User Stories

### Configuration & Setup

**US-1:** As a parent, I want to add multiple children to the system with individual time limits, so each child has appropriate screen time for their age.

**US-2:** As a parent, I want to set different time limits for weekdays vs. weekends, so children have more flexibility on non-school days.

**US-3:** As a parent, I want to configure which OS user accounts map to which children, so the system knows which child is using the computer.

**US-4:** As a parent using shared logins, I want children to identify themselves at startup, so time is tracked correctly when multiple kids share one account.

**US-5:** As an IT administrator, I want to configure time limits via YAML files in GitHub, so I can manage policies centrally.

### Monitoring & Enforcement

**US-6:** As a child, I want to see how much time I have remaining today, so I can plan my computer usage.

**US-7:** As a child, I want warnings when I have 15, 5, and 1 minute(s) remaining, so I can save my work before lockout.

**US-8:** As a parent, I want the computer to automatically lock/logout when a child's time expires, so limits are enforced without my intervention.

**US-9:** As a parent, I want to view each child's usage history, so I can understand their screen time patterns.

**US-10:** As a parent, I want the system to resist tampering (time changes, process kills), so children cannot bypass limits.

### Admin Controls

**US-11:** As a parent, I want to grant temporary time extensions with a password, so I can allow extra time for homework or special occasions.

**US-12:** As a parent, I want to pause time tracking temporarily, so time doesn't count during supervised educational activities.

**US-13:** As a parent, I want to reset time immediately if I give permission, so children can use the computer after reaching their limit.

**US-14:** As a parent, I want an admin account that's exempt from time limits, so I can always access the computer.

## Technical Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Tauri v2 GUI Application                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Admin Panel  │  │ Kid Selector │  │ Time Display │     │
│  │ (Config/     │  │ (Shared      │  │ (Remaining   │     │
│  │  Override)   │  │  Login Mode) │  │  Time Widget)│     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                           ↕ Tauri Commands (IPC)
┌─────────────────────────────────────────────────────────────┐
│                   Rust Backend (Core Logic)                 │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Time Tracking Service (Background Daemon)            │  │
│  │  - Session monitoring                                │  │
│  │  - Usage accumulation                                │  │
│  │  - Warning notifications                             │  │
│  │  - Lock enforcement                                  │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Configuration Manager                                │  │
│  │  - Load/save time limit configs                      │  │
│  │  - Child profile management                          │  │
│  │  - Schedule handling                                 │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ State Manager                                        │  │
│  │  - Daily usage tracking per child                    │  │
│  │  - Session state persistence                         │  │
│  │  - Admin override state                              │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Platform Lock Manager (OS-specific)                  │  │
│  │  - Windows: Lock workstation API                     │  │
│  │  - macOS: CGSession / lock screen                    │  │
│  │  - Linux: systemd-logind / XScreenSaver              │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Admin Auth Manager                                   │  │
│  │  - Password verification                             │  │
│  │  - Override authorization                            │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                           ↕
┌─────────────────────────────────────────────────────────────┐
│                   Platform-Specific Layer                   │
│  - Auto-start integration (systemd/LaunchAgent/Task Sched) │
│  - System tray integration                                 │
│  - Native notifications                                    │
│  - Session detection (login/logout/lock)                   │
└─────────────────────────────────────────────────────────────┘
                           ↕
┌─────────────────────────────────────────────────────────────┐
│                      Data Storage                           │
│  - time-limits-config.yaml (configuration)                 │
│  - time-limits-state.json (daily usage, sessions)          │
│  - time-limits-history.json (historical data)              │
└─────────────────────────────────────────────────────────────┘
```

### Data Models

#### Configuration (YAML)

```yaml
time_limits:
  # Admin configuration
  admin:
    password_hash: "argon2id$v=19$m=65536,t=3,p=4$..."
    admin_accounts:
      - "parent_username"
      - "admin"

  # Children profiles
  children:
    - id: "kid1"
      name: "Alice"
      # OS user mapping (omit for shared login mode)
      os_users:
        - "alice"
        - "alice.smith"
      # Time limits
      limits:
        weekday:
          hours: 2
          minutes: 0
        weekend:
          hours: 4
          minutes: 0
        # Optional: specific day overrides
        custom:
          - days: ["monday", "wednesday", "friday"]
            hours: 1
            minutes: 30
      # Warning thresholds (minutes before lockout)
      warnings: [15, 5, 1]
      # Grace period after warning (seconds)
      grace_period: 60

    - id: "kid2"
      name: "Bob"
      os_users:
        - "bob"
      limits:
        weekday:
          hours: 1
          minutes: 30
        weekend:
          hours: 3
          minutes: 0
      warnings: [10, 5, 1]
      grace_period: 30

  # Shared login mode (when multiple kids share one OS account)
  shared_login:
    enabled: true
    shared_accounts:
      - "family"
      - "kids"
    # Require kid selection at startup
    require_selection: true
    # Allow kid switching during session
    allow_switching: false
    # Auto-select if only one kid has time remaining
    auto_select_if_unique: true

  # Enforcement settings
  enforcement:
    # Action when time expires: lock | logout | shutdown
    action: "lock"
    # Block time changes while app is running
    prevent_time_manipulation: true
    # Require admin password to close/disable tracker
    require_admin_to_quit: true
    # Monitor for process tampering
    self_protection: true
```

#### State (JSON)

```json
{
  "version": "1.0",
  "state_date": "2025-11-16",
  "children": [
    {
      "id": "kid1",
      "name": "Alice",
      "today": {
        "date": "2025-11-16",
        "used_seconds": 3600,
        "remaining_seconds": 3600,
        "sessions": [
          {
            "start": "2025-11-16T08:00:00Z",
            "end": "2025-11-16T09:00:00Z",
            "duration_seconds": 3600
          }
        ],
        "warnings_shown": ["15min", "5min"],
        "locked_at": null
      }
    },
    {
      "id": "kid2",
      "name": "Bob",
      "today": {
        "date": "2025-11-16",
        "used_seconds": 5400,
        "remaining_seconds": 0,
        "sessions": [
          {
            "start": "2025-11-16T14:00:00Z",
            "end": "2025-11-16T15:30:00Z",
            "duration_seconds": 5400
          }
        ],
        "warnings_shown": ["10min", "5min", "1min"],
        "locked_at": "2025-11-16T15:30:00Z"
      }
    }
  ],
  "active_session": {
    "child_id": "kid1",
    "session_start": "2025-11-16T16:00:00Z",
    "last_activity": "2025-11-16T16:15:00Z"
  },
  "admin_overrides": [
    {
      "child_id": "kid1",
      "type": "extension",
      "additional_seconds": 1800,
      "granted_at": "2025-11-16T10:00:00Z",
      "granted_by": "parent_username",
      "reason": "Homework project"
    }
  ]
}
```

#### History (JSON)

```json
{
  "version": "1.0",
  "records": [
    {
      "date": "2025-11-15",
      "children": [
        {
          "id": "kid1",
          "name": "Alice",
          "used_seconds": 7200,
          "limit_seconds": 7200,
          "sessions_count": 2,
          "overrides": []
        },
        {
          "id": "kid2",
          "name": "Bob",
          "used_seconds": 5400,
          "limit_seconds": 5400,
          "sessions_count": 1,
          "overrides": [
            {
              "type": "extension",
              "additional_seconds": 900,
              "reason": "Online class"
            }
          ]
        }
      ]
    }
  ]
}
```

### Tauri v2 Integration

#### App Structure

```
src-tauri/
├── Cargo.toml                    # Rust dependencies (with Tauri v2)
├── tauri.conf.json              # Tauri configuration
├── src/
│   ├── main.rs                  # Tauri app entry point
│   ├── time_limits/
│   │   ├── mod.rs               # Time limits module
│   │   ├── config.rs            # Configuration management
│   │   ├── tracker.rs           # Usage tracking service
│   │   ├── enforcement.rs       # Lock/logout enforcement
│   │   ├── scheduler.rs         # Schedule calculations
│   │   ├── auth.rs              # Admin authentication
│   │   └── platform/
│   │       ├── mod.rs
│   │       ├── windows.rs       # Windows-specific locking
│   │       ├── macos.rs         # macOS-specific locking
│   │       └── linux.rs         # Linux-specific locking
│   └── commands.rs              # Tauri IPC commands

ui/
├── package.json
├── index.html
├── src/
│   ├── main.ts                  # Vue/React/Svelte entry
│   ├── App.vue                  # Main app component
│   ├── components/
│   │   ├── AdminPanel.vue       # Admin config/override UI
│   │   ├── KidSelector.vue      # Kid selection (shared login)
│   │   ├── TimeDisplay.vue      # Remaining time widget
│   │   ├── UsageChart.vue       # Usage history visualization
│   │   └── WarningDialog.vue    # Time expiring warnings
│   └── stores/
│       └── timeStore.ts         # State management (Pinia/Vuex)
```

#### Tauri Commands (IPC)

```rust
// Configuration commands
#[tauri::command]
fn get_time_limits_config() -> Result<TimeLimitsConfig, String>

#[tauri::command]
fn update_time_limits_config(config: TimeLimitsConfig) -> Result<(), String>

#[tauri::command]
fn add_child(child: ChildProfile) -> Result<(), String>

#[tauri::command]
fn update_child(child_id: String, child: ChildProfile) -> Result<(), String>

#[tauri::command]
fn remove_child(child_id: String) -> Result<(), String>

// State/monitoring commands
#[tauri::command]
fn get_current_state() -> Result<TimeLimitsState, String>

#[tauri::command]
fn get_remaining_time(child_id: String) -> Result<i64, String>

#[tauri::command]
fn get_usage_history(child_id: String, days: u32) -> Result<Vec<DayRecord>, String>

// Session commands
#[tauri::command]
fn select_child(child_id: String) -> Result<(), String>

#[tauri::command]
fn switch_child(child_id: String, admin_password: String) -> Result<(), String>

// Admin commands
#[tauri::command]
fn admin_authenticate(password: String) -> Result<bool, String>

#[tauri::command]
fn admin_grant_extension(child_id: String, additional_minutes: u32, password: String, reason: String) -> Result<(), String>

#[tauri::command]
fn admin_reset_time(child_id: String, password: String) -> Result<(), String>

#[tauri::command]
fn admin_pause_tracking(password: String) -> Result<(), String>

#[tauri::command]
fn admin_resume_tracking(password: String) -> Result<(), String>

#[tauri::command]
fn admin_unlock_child(child_id: String, password: String) -> Result<(), String>
```

### Time Tracking Algorithm

```rust
// Pseudo-code for tracking logic
fn track_time_continuously() {
    let interval = Duration::from_secs(10); // Track every 10 seconds

    loop {
        sleep(interval);

        // Get current active child
        let child = get_active_child();

        // Check if child is exempt (admin account)
        if is_admin_account(current_user()) {
            continue;
        }

        // Detect user activity (mouse/keyboard events)
        if !is_user_active() {
            continue; // Don't count idle time
        }

        // Add interval to used time
        child.used_seconds += interval.as_secs();

        // Calculate remaining time
        let limit = get_limit_for_today(child);
        let remaining = limit - child.used_seconds;

        // Check for warnings
        if should_show_warning(remaining, child.warnings) {
            show_warning_notification(child, remaining);
        }

        // Enforce limit
        if remaining <= 0 {
            enforce_lock(child);
        }

        // Persist state
        save_state();
    }
}
```

### Platform-Specific Lock Mechanisms

#### Windows
```rust
use windows::Win32::System::Shutdown::LockWorkStation;

fn lock_workstation_windows() -> Result<()> {
    unsafe {
        LockWorkStation()?;
    }
    Ok(())
}
```

#### macOS
```rust
use std::process::Command;

fn lock_screen_macos() -> Result<()> {
    Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"q\" using {command down, control down}")
        .output()?;
    Ok(())
}
```

#### Linux
```rust
use std::process::Command;

fn lock_screen_linux() -> Result<()> {
    // Try multiple methods in order of preference

    // 1. systemd-logind (modern)
    if Command::new("loginctl")
        .arg("lock-session")
        .output()
        .is_ok() {
        return Ok(());
    }

    // 2. XScreenSaver
    if Command::new("xscreensaver-command")
        .arg("-lock")
        .output()
        .is_ok() {
        return Ok(());
    }

    // 3. gnome-screensaver
    if Command::new("gnome-screensaver-command")
        .arg("--lock")
        .output()
        .is_ok() {
        return Ok(());
    }

    Err(anyhow::anyhow!("No supported lock mechanism found"))
}
```

### Security Considerations

1. **Tamper Resistance**
   - Store state files in protected locations (requires admin to modify)
   - Hash state files to detect manual edits
   - Monitor system time changes and pause tracking if detected
   - Self-healing: restart tracker service if killed
   - Code signing on executables

2. **Password Security**
   - Use Argon2id for password hashing
   - Never store plaintext passwords
   - Rate limit password attempts
   - Lock admin panel after 3 failed attempts

3. **Data Privacy**
   - Store all data locally (no cloud sync in v1)
   - No tracking of activity content (only time)
   - Clear audit trail of admin actions

4. **Process Isolation**
   - Run tracker as system service with elevated privileges
   - UI runs in user context
   - IPC uses Tauri's secure communication channel

## User Interface Design

### Main Application Window

```
┌────────────────────────────────────────────────────────────┐
│ Family Policy - Time Limits                      [_ □ ×]   │
├────────────────────────────────────────────────────────────┤
│  [Home] [Children] [Schedule] [History] [Settings]         │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Today's Usage                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Alice                                   ⏱ 1h 30m left│  │
│  │  [████████████░░░░░░░░] 50%                           │  │
│  │  Used: 2h 00m  |  Limit: 4h 00m (Weekend)            │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Bob                                     ⏱ 0h 00m left│  │
│  │  [████████████████████] 100%              🔒 LOCKED   │  │
│  │  Used: 1h 30m  |  Limit: 1h 30m (Weekday)            │  │
│  │  [Admin Override...]                                  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Quick Actions                                              │
│  [+ Add Child]  [⚙️ Settings]  [📊 View Reports]           │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Kid Selector (Shared Login Mode)

```
┌────────────────────────────────────────────────────────────┐
│                    Who is using the computer?              │
├────────────────────────────────────────────────────────────┤
│                                                             │
│    ┌───────────────┐     ┌───────────────┐                │
│    │               │     │               │                │
│    │   👧 Alice   │     │   👦 Bob     │                │
│    │               │     │               │                │
│    │  2h 00m left  │     │  Time's up!   │                │
│    │               │     │   (locked)    │                │
│    └───────────────┘     └───────────────┘                │
│                                                             │
│                  [Admin Login]                              │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Time Expiring Warning

```
┌────────────────────────────────────────────────────────────┐
│  ⚠️  Time Running Out                                      │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Alice, you have 5 minutes of computer time remaining.     │
│                                                             │
│  Please save your work and finish up.                      │
│                                                             │
│  Time left: 4:58                                            │
│                                                             │
│  [OK]                                                       │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Admin Override Dialog

```
┌────────────────────────────────────────────────────────────┐
│  Admin Override - Bob                                      │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Current Status: Time limit reached (0m left)               │
│                                                             │
│  Grant additional time:                                     │
│  ○ 15 minutes                                               │
│  ○ 30 minutes                                               │
│  ● 1 hour                                                   │
│  ○ Custom: [___] hours [___] minutes                        │
│                                                             │
│  Reason (optional):                                         │
│  [Homework assignment                                ]       │
│                                                             │
│  Admin Password:                                            │
│  [••••••••••]                                               │
│                                                             │
│  [Cancel]                        [Grant Time]               │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### System Tray Icon

```
┌────────────────────────────────┐
│ Family Policy                   │
├────────────────────────────────┤
│ Alice: 1h 30m remaining         │
│ Bob: Locked (time's up)         │
├────────────────────────────────┤
│ Open Dashboard                  │
│ Admin Override...               │
│ Pause Tracking (Admin)          │
│ ─────────────────────────       │
│ Settings                        │
│ Quit (requires admin)           │
└────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Core Backend (Week 1-2)

1. **Data Models & Configuration**
   - Define Rust structs for config, state, history
   - YAML/JSON serialization/deserialization
   - Configuration validation
   - State file management with atomic writes

2. **Time Tracking Service**
   - Background daemon/service structure
   - Timer loop (10-second intervals)
   - Usage accumulation logic
   - Daily reset at midnight
   - State persistence

3. **Platform Lock Implementation**
   - Windows lock workstation
   - macOS lock screen
   - Linux lock screen (multiple methods)
   - Platform abstraction layer

### Phase 2: Tauri Application (Week 3-4)

1. **Tauri v2 Setup**
   - Initialize Tauri project
   - Configure build targets (Windows, macOS, Linux)
   - Set up IPC commands
   - System tray integration

2. **UI Framework**
   - Choose frontend framework (Vue 3 recommended)
   - Component structure
   - State management (Pinia)
   - Styling (Tailwind CSS)

3. **Core UI Components**
   - Main dashboard
   - Kid selector
   - Time display widget
   - Warning dialogs
   - Admin panels

### Phase 3: Admin & Security (Week 5)

1. **Authentication**
   - Password hashing (Argon2)
   - Admin verification
   - Session management

2. **Override System**
   - Time extension grants
   - Pause/resume tracking
   - Manual unlocks
   - Audit logging

3. **Tamper Resistance**
   - System time manipulation detection
   - Process self-healing
   - State file integrity checks

### Phase 4: CLI Integration (Week 6)

1. **CLI Commands**
   - `family-policy time-limits init` - Initialize config
   - `family-policy time-limits add-child` - Add child profile
   - `family-policy time-limits status` - Show current usage
   - `family-policy time-limits override` - Admin override from CLI
   - `family-policy time-limits start` - Start tracking service
   - `family-policy time-limits stop` - Stop tracking service

2. **Service Management**
   - Auto-start on boot (systemd, LaunchAgent, Task Scheduler)
   - Service installation/uninstallation
   - Service status monitoring

### Phase 5: Testing & Polish (Week 7-8)

1. **Testing**
   - Unit tests for tracking logic
   - Integration tests for state management
   - Platform-specific testing (Win/Mac/Linux)
   - Tamper resistance testing
   - UI/UX testing

2. **Documentation**
   - User guide
   - Admin guide
   - Configuration examples
   - Troubleshooting guide

3. **Polish**
   - Error handling
   - Logging improvements
   - Performance optimization
   - UI refinements

## Testing Strategy

### Unit Tests

- Configuration parsing and validation
- Time calculation logic (limits, remaining time)
- Schedule resolution (weekday/weekend/custom)
- Password hashing and verification
- State serialization/deserialization

### Integration Tests

- End-to-end time tracking flow
- Admin override workflows
- Child switching (shared login)
- Daily reset functionality
- Cross-platform lock mechanisms

### Manual Testing Scenarios

1. **Basic Usage**
   - Child reaches time limit → computer locks
   - Warning notifications appear at correct intervals
   - Time resets at midnight

2. **Shared Login**
   - Multiple kids select identity at startup
   - Correct time tracking per selected kid
   - Prevent switching without admin password

3. **Admin Override**
   - Grant time extension → additional time available
   - Pause tracking → time doesn't accumulate
   - Manual unlock → child can use computer despite limit

4. **Tamper Resistance**
   - Change system time → tracking pauses or adjusts
   - Kill tracker process → auto-restarts
   - Edit state file → detects corruption and resets

5. **Platform-Specific**
   - Test lock mechanisms on Windows, macOS, Linux
   - Verify auto-start on boot
   - System tray integration

## Success Metrics

### Functional Metrics
- ✅ Time limits enforced accurately (±30 seconds)
- ✅ Locks execute within 5 seconds of time expiration
- ✅ Warnings appear at correct thresholds
- ✅ State persists across reboots
- ✅ Admin overrides work 100% of time

### Performance Metrics
- CPU usage < 1% when tracking
- Memory usage < 50MB for tracker service
- UI startup time < 2 seconds
- State file writes < 100ms

### Security Metrics
- 0 exploitable bypasses in security review
- Password hashing meets OWASP standards
- Tamper detection catches 95%+ of manipulation attempts

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Kids find bypass method | High | Medium | Implement multiple tamper-resistance techniques, regular security reviews |
| Platform lock mechanism fails | High | Low | Fallback mechanisms per platform, logout/shutdown as alternatives |
| Time tracking inaccurate | Medium | Low | Extensive testing, 10-second granularity acceptable |
| Admin forgets password | Medium | Medium | Password recovery mechanism or config file reset instructions |
| Performance issues | Low | Low | Lightweight implementation, performance testing |
| Cross-platform bugs | Medium | Medium | Test on all three platforms, CI/CD pipeline |

## Open Questions

1. **Password Recovery**: How should admins recover if they forget the password?
   - **Answer**: Provide manual config file edit instructions in docs (delete password hash)

2. **Network-Based Time Sync**: Should we verify system time against NTP to prevent manipulation?
   - **Answer**: V2 feature - v1 will detect sudden time jumps and pause tracking

3. **App-Specific Exemptions**: Should certain apps (e.g., educational software) not count toward limits?
   - **Answer**: Not in v1 - future feature

4. **Multi-User Concurrent Sessions**: How to handle fast user switching on Windows/macOS?
   - **Answer**: Track per-session, accumulate when each child is active

5. **Cloud Sync**: Should usage data sync across multiple computers?
   - **Answer**: Not in v1 - local-only for privacy

## Appendix

### Technology Stack

- **Language**: Rust (backend), TypeScript (frontend)
- **UI Framework**: Tauri v2 with Vue 3
- **Styling**: Tailwind CSS
- **State Management**: Pinia
- **Password Hashing**: Argon2
- **Serialization**: serde (YAML/JSON)
- **Testing**: cargo test, vitest
- **CI/CD**: GitHub Actions

### Dependencies (Cargo.toml additions)

```toml
[dependencies]
tauri = "2.0"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
chrono = { version = "0.4", features = ["serde"] }
argon2 = "0.5"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4", "serde"] }

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.52", features = ["Win32_System_Shutdown", "Win32_Foundation"] }

[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.25"
objc = "0.2"
```

### File Locations

**Linux:**
- Config: `/etc/family-policy/time-limits-config.yaml`
- State: `/var/lib/family-policy/time-limits-state.json`
- History: `/var/lib/family-policy/time-limits-history.json`

**macOS:**
- Config: `/Library/Application Support/family-policy/time-limits-config.yaml`
- State: `/Library/Application Support/family-policy/time-limits-state.json`
- History: `/Library/Application Support/family-policy/time-limits-history.json`

**Windows:**
- Config: `C:\ProgramData\family-policy\time-limits-config.yaml`
- State: `C:\ProgramData\family-policy\time-limits-state.json`
- History: `C:\ProgramData\family-policy\time-limits-history.json`

### References

- Tauri v2 Documentation: https://v2.tauri.app/
- Argon2 Specification: https://github.com/P-H-C/phc-winner-argon2
- Windows Lock API: https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-lockworkstation
- systemd-logind: https://www.freedesktop.org/software/systemd/man/loginctl.html
