# Family Policy Multi-Mode Implementation Progress

This document tracks the implementation progress of the new multi-mode architecture with User UI, Admin UI, and enhanced CLI/Daemon modes.

## Implementation Phases

### Phase 1: Core Refactoring ✓ (Complete)

Core business logic refactoring to support privilege separation and dry-run capabilities.

**Status**: Completed in commits 3408c96, 7ddf276
**Integration**: Core modules integrated into local mode and agent daemon

#### 1.1 Create Core Module Structure ✓
- [x] Create `src-tauri/src/core/` directory
- [x] Create `src-tauri/src/core/mod.rs` with module exports
- [x] Update `src-tauri/src/main.rs` to include `mod core;`

#### 1.2 Implement Privilege Checking (`src/core/privileges.rs`) ✓
- [x] Create `src-tauri/src/core/privileges.rs`
- [x] Define `PrivilegeLevel` enum (User, Admin)
- [x] Define `PrivilegeCheck` struct with `required` and `allow_dry_run` fields
- [x] Implement `check_privileges(check: PrivilegeCheck, is_dry_run: bool) -> Result<()>`
- [x] Consolidated `is_admin()` function from multiple locations
- [x] Implement platform-specific `is_admin()` for Windows
- [x] Implement platform-specific `is_admin()` for Unix
- [x] Add unit tests for privilege checking logic

#### 1.3 Implement Policy Application Orchestration (`src/core/apply.rs`) ✓
- [x] Create `src-tauri/src/core/apply.rs`
- [x] Extract policy application logic from `src-tauri/src/commands/local.rs`
- [x] Implement `apply_policies_from_config(config: &Config, dry_run: bool) -> Result<ApplyResult>`
- [x] Implement `remove_all_policies(dry_run: bool) -> Result<RemovalResult>`
- [x] Define `ApplyResult` struct with success/failure details
- [x] Define `RemovalResult` struct with removal summary
- [x] Integrated with local mode and agent daemon
- [x] Add unit tests for apply logic

#### 1.4 Implement Diff Generation (`src/core/diff.rs`) ✓
- [x] Create `src-tauri/src/core/diff.rs`
- [x] Define `PolicyDiff` struct to represent changes
- [x] Define `BrowserDiff` struct for browser-specific changes
- [x] Define `ExtensionDiff` enum (Added, Removed, Unchanged)
- [x] Define `PrivacySettingDiff` struct
- [x] Implement `generate_diff(new_config: &Config, current_state: &State) -> Result<PolicyDiff>`
- [x] Implement diff for extensions (additions, removals)
- [x] Implement diff for privacy settings (changes)
- [x] Implement pretty-printing for diffs (CLI output)
- [x] Serialization support via serde (ready for Tauri commands)
- [x] Add unit tests for diff generation

#### 1.5 Update State File Permissions ✓
- [x] Modify `src-tauri/src/state.rs` in `save_state()` function
- [x] Change Unix permissions from `0o600` to `0o644` after writing state file
- [x] Directory permissions allow reading (0o755)
- [x] Add comment explaining why state file is world-readable
- [x] Documented permission model in state.rs

---

### Phase 2: CLI Enhancement ✓ (Complete)

Update CLI structure to support new subcommands and privilege checking.

**Status**: Completed in commits 8291aeb, 7ddf276
**Improvements**: Integrated dry-run diff preview, better error reporting

#### 2.1 Update CLI Argument Parser (`src/cli.rs`) ✓
- [x] Add `UserUi` variant to `Commands` enum
- [x] Add `AdminUi` variant to `Commands` enum
- [x] Add `Daemon` variant to `Commands` enum
- [x] Add `--systray` flag to `UserUi` command
- [x] Add `--window` flag to `UserUi` command (default)
- [x] Update command descriptions and help text
- [x] Backward compatibility maintained with existing commands

#### 2.2 Implement Privilege Checking in CLI Routing (`src/main.rs`) ✓
- [x] Import `core::privileges` module
- [x] Add privilege checks before each command execution
- [x] For `apply`: require Admin, allow dry-run for User
- [x] For `daemon`: require Admin
- [x] For `start`/`stop`: require Admin
- [x] For `check-now`: require Admin, allow dry-run for User
- [x] For `status`/`show-config`: allow User
- [x] For `user-ui`: allow User
- [x] For `admin-ui`: require Admin
- [x] For `install-service`/`uninstall-service`: require Admin
- [x] Clear error messages for insufficient privileges

#### 2.3 Update Existing Commands for Dry-Run Support ✓
- [x] Modified `commands::run_local_mode()` to use `core::apply`
- [x] `--dry-run` flag respected throughout
- [x] Added diff preview in dry-run mode using `core::diff`
- [x] Dry-run mode tested for local commands
- [x] Command output clearly indicates dry-run vs actual execution
- [x] Agent daemon simplified to use centralized policy application

---

### Phase 3: User UI Implementation ⚙️ (In Progress)

Create User UI mode for status display and admin elevation.

**Status**: Backend Tauri commands complete (commit 1c5ec00)
**Next**: Vue frontend components and window setup

#### 3.1 Backend Tauri Commands ✓
- [x] Create `src-tauri/src/ui/user_commands.rs` for User UI
- [x] Create `src-tauri/src/ui/admin_commands.rs` for Admin UI
- [x] Update `src-tauri/src/ui/mod.rs` to export command modules

#### 3.2 User UI Tauri Commands (`user_commands.rs`) ✓
- [x] Implement `read_state() -> Result<StateInfo, String>`
  - [x] Read state file from standard location
  - [x] Parse and format state information
  - [x] Return extension counts and privacy settings
  - [x] Handle missing state file gracefully
- [x] Implement `read_config_summary(path) -> Result<ConfigSummary, String>`
  - [x] Load and parse policy config
  - [x] Return policy names, extension counts, browsers
- [x] Implement `preview_apply(path) -> Result<PolicyDiff, String>`
  - [x] Load policy config from provided path
  - [x] Load current state
  - [x] Use `core::diff::generate_diff()` to create diff
  - [x] Return serialized diff with changes
- [x] Implement `check_admin() -> Result<bool, String>`
  - [x] Check if running with admin privileges
- [x] Implement `request_elevation() -> Result<ElevationResult, String>`
  - [x] Platform-specific elevation guidance
  - [x] Returns instructions for sudo/Administrator restart
- [x] Define comprehensive types: `StateInfo`, `ConfigSummary`, `BrowserCounts`
- [x] Add error handling for all commands
- [x] Add unit tests for helper functions

#### 3.3 Admin UI Tauri Commands (`admin_commands.rs`) ✓
- [x] Implement `apply_policies(path) -> Result<ApplyResult, String>`
  - [x] Verify admin privileges
  - [x] Apply policies using `core::apply`
  - [x] Return detailed results
- [x] Implement `remove_policies() -> Result<RemovalResult, String>`
  - [x] Verify admin privileges
  - [x] Remove all policies
  - [x] Return removal counts
- [x] Implement `preview_removal() -> Result<RemovalResult, String>`
  - [x] Preview what would be removed (no admin needed)
- [x] Implement `validate_config(path) -> Result<ValidationResult, String>`
  - [x] Validate YAML format and structure
  - [x] Return errors and warnings
- [x] Implement `save_config(path, yaml) -> Result<(), String>`
  - [x] Check admin privileges for system paths
  - [x] Validate before saving
- [x] Implement `get_default_config() -> Result<String, String>`
  - [x] Return example configuration YAML
- [x] Define types: `ValidationResult`, `ElevationResult`
- [x] Add unit tests for validation

#### 3.4 Frontend Implementation ✗
- [ ] Implement Vue components for User UI
- [ ] Implement systray mode with Tauri
- [ ] Implement window mode
- [ ] Add "Launch Admin Settings" functionality
- [ ] Test user UI startup on all platforms

#### 3.4 Implement Platform-Specific Elevation (`src/ui/user/elevation.rs`) ✗
- [ ] Implement Linux elevation with `pkexec`
  - [ ] Get current executable path
  - [ ] Try `pkexec {exe} admin-ui` first
  - [ ] Fallback to `x-terminal-emulator -e sudo {exe} admin-ui`
  - [ ] Handle DISPLAY environment variable
  - [ ] Test on various Linux distributions
- [ ] Implement macOS elevation with `osascript`
  - [ ] Get current executable path
  - [ ] Build AppleScript command
  - [ ] Execute `osascript -e "do shell script ... with administrator privileges"`
  - [ ] Test on macOS
- [ ] Implement Windows elevation with ShellExecute
  - [ ] Use `windows_sys::Win32::UI::Shell::ShellExecuteW`
  - [ ] Set verb to "runas" for UAC prompt
  - [ ] Pass "admin-ui" as parameter
  - [ ] Test on Windows 10/11
- [ ] Add error handling for elevation failures
- [ ] Add logging for elevation attempts

#### 3.5 Create User UI Vue Components ✗
- [ ] Create `src/` Vue app structure (if not exists)
- [ ] Create `src/views/UserStatus.vue` component
  - [ ] Display current policy status
  - [ ] Show applied extensions per browser
  - [ ] Show privacy settings per browser
  - [ ] Add "Refresh" button
- [ ] Create `src/views/PolicyDiff.vue` component
  - [ ] Display policy diff in readable format
  - [ ] Highlight additions (green), removals (red), changes (yellow)
  - [ ] Show extension changes
  - [ ] Show privacy setting changes
- [ ] Create `src/components/SystemTray.vue` (if needed)
- [ ] Add routing for User UI views
- [ ] Style components with consistent design
- [ ] Test UI responsiveness
- [ ] Add loading states and error handling

---

### Phase 4: Admin UI Implementation ✗

Create Admin UI mode for configuration editing and policy application.

#### 4.1 Create Admin UI Module Structure ✗
- [ ] Create `src-tauri/src/ui/admin/` directory
- [ ] Create `src-tauri/src/ui/admin/mod.rs` with public interface
- [ ] Create `src-tauri/src/ui/admin/commands.rs` for Tauri commands
- [ ] Create `src-tauri/src/ui/admin/config_editor.rs` for config editing logic
- [ ] Update `src-tauri/src/ui/mod.rs` to export admin module

#### 4.2 Implement Admin UI Window Setup (`src/ui/admin/mod.rs`) ✗
- [ ] Implement `run_admin_ui() -> Result<()>`
- [ ] Check admin privileges at startup (fail fast if not admin)
- [ ] Configure Tauri builder with admin UI commands
- [ ] Set up window configuration (size, title, etc.)
- [ ] No systray icon for admin UI
- [ ] Test admin UI startup on all platforms
- [ ] Test privilege check rejection for non-admin

#### 4.3 Implement Admin UI Tauri Commands (`src/ui/admin/commands.rs`) ✗
- [ ] Implement `#[tauri::command] get_agent_config() -> Result<AgentConfig, String>`
  - [ ] Read agent config from standard location
  - [ ] Return config with sensitive fields (token)
  - [ ] Handle missing config (return default)
- [ ] Implement `#[tauri::command] save_agent_config(config: AgentConfig) -> Result<(), String>`
  - [ ] Validate config (URL, intervals, etc.)
  - [ ] Write to standard location
  - [ ] Set appropriate permissions
  - [ ] Trigger daemon reload (if running)
- [ ] Implement `#[tauri::command] get_policy_config(path: String) -> Result<PolicyConfig, String>`
  - [ ] Read policy YAML from provided path
  - [ ] Parse and validate
  - [ ] Return structured config
- [ ] Implement `#[tauri::command] save_policy_config(path: String, config: PolicyConfig) -> Result<(), String>`
  - [ ] Validate policy config
  - [ ] Serialize to YAML
  - [ ] Write to file atomically
- [ ] Implement `#[tauri::command] preview_policy_changes(config: PolicyConfig) -> Result<PolicyDiff, String>`
  - [ ] Use `core::diff::generate_diff()` with provided config
  - [ ] Return diff without applying
- [ ] Implement `#[tauri::command] apply_policies(config: PolicyConfig) -> Result<ApplyResult, String>`
  - [ ] Use `core::apply::apply_policies_from_config()`
  - [ ] Apply policies to system
  - [ ] Return detailed result
- [ ] Implement `#[tauri::command] control_daemon(action: DaemonAction) -> Result<(), String>`
  - [ ] Support "start", "stop", "restart" actions
  - [ ] Call appropriate agent commands
  - [ ] Return success/failure
- [ ] Add comprehensive error handling
- [ ] Add logging for all admin actions

#### 4.4 Implement Config Editor Logic (`src/ui/admin/config_editor.rs`) ✗
- [ ] Implement helper functions for config validation
- [ ] Implement `validate_agent_config(config: &AgentConfig) -> Result<()>`
  - [ ] Check URL is HTTPS
  - [ ] Check poll interval >= 60 seconds
  - [ ] Validate token format if present
- [ ] Implement `validate_policy_config(config: &PolicyConfig) -> Result<()>`
  - [ ] Check at least one policy exists
  - [ ] Validate extension IDs
  - [ ] Ensure browser-specific IDs are present
- [ ] Implement config merging helpers
- [ ] Implement config backup/restore helpers
- [ ] Add unit tests for validation logic

#### 4.5 Create Admin UI Vue Components ✗
- [ ] Create `src/views/AdminSettings.vue` component
  - [ ] Tab layout (Agent Config, Policy Editor, Status)
  - [ ] Form for agent configuration editing
  - [ ] Display current daemon status
  - [ ] Daemon control buttons (start/stop/restart)
- [ ] Create `src/views/PolicyEditor.vue` component
  - [ ] YAML editor or structured form
  - [ ] Policy list view
  - [ ] Add/remove policy entries
  - [ ] Add/remove extensions
  - [ ] Configure privacy settings per browser
- [ ] Create `src/views/PolicyPreview.vue` component
  - [ ] Display preview diff before apply
  - [ ] "Apply" and "Cancel" buttons
  - [ ] Loading state during application
- [ ] Create `src/components/ConfigForm.vue`
  - [ ] Reusable form for config fields
  - [ ] Validation indicators
- [ ] Add routing for Admin UI views
- [ ] Style components with admin theme
- [ ] Test UI workflows (edit → preview → apply)
- [ ] Add confirmation dialogs for destructive actions

---

### Phase 5: Frontend Routing & UI Polish ✗

Configure routing to support both User UI and Admin UI from the same Vue app.

#### 5.1 Configure Multi-Mode Routing ✗
- [ ] Install Vue Router (if not already)
- [ ] Create `src/router/index.ts` with route configuration
- [ ] Define `/user` route for User UI views
- [ ] Define `/admin` route for Admin UI views
- [ ] Configure default route based on runtime mode
- [ ] Test routing between views
- [ ] Add navigation guards (prevent admin routes in user mode)

#### 5.2 Shared UI Components ✗
- [ ] Create `src/components/StatusCard.vue`
  - [ ] Display browser status
  - [ ] Show extension count
  - [ ] Show privacy settings
- [ ] Create `src/components/ExtensionList.vue`
  - [ ] List extensions with IDs
  - [ ] Show force-install status
- [ ] Create `src/components/PrivacySettings.vue`
  - [ ] Display privacy controls per browser
- [ ] Create `src/components/LoadingSpinner.vue`
- [ ] Create `src/components/ErrorDisplay.vue`
- [ ] Add consistent styling and theming

#### 5.3 Icon and Asset Updates ✗
- [ ] Create/update app icon for User UI
- [ ] Create system tray icons (normal, active, error states)
- [ ] Create/update app icon for Admin UI
- [ ] Add browser icons (Chrome, Firefox, Edge)
- [ ] Add status icons (success, error, warning)
- [ ] Optimize assets for different platforms
- [ ] Test icon display on all platforms

---

### Phase 6: Integration & Testing ✗

Comprehensive testing of all modes and their interactions.

#### 6.1 Unit Tests ✗
- [ ] Test `core::privileges` module
  - [ ] Test privilege checking logic
  - [ ] Test dry-run permission logic
  - [ ] Mock `is_admin()` for testing
- [ ] Test `core::apply` module
  - [ ] Test policy application with mock state
  - [ ] Test removal with mock state
  - [ ] Test error handling
- [ ] Test `core::diff` module
  - [ ] Test diff generation for additions
  - [ ] Test diff generation for removals
  - [ ] Test diff generation for changes
  - [ ] Test diff serialization
- [ ] Test `ui::user::elevation` module
  - [ ] Mock platform-specific elevation calls
  - [ ] Test error handling
- [ ] Test `ui::admin::config_editor` module
  - [ ] Test config validation
  - [ ] Test various invalid configs

#### 6.2 Integration Tests ✗
- [ ] Test User UI startup
  - [ ] Test systray mode
  - [ ] Test window mode
  - [ ] Test status retrieval
- [ ] Test Admin UI startup
  - [ ] Test privilege check
  - [ ] Test rejection for non-admin
- [ ] Test privilege elevation flow
  - [ ] Test User UI → Admin UI launch
  - [ ] Verify Admin UI starts with privileges
- [ ] Test config editing workflow
  - [ ] Edit agent config in Admin UI
  - [ ] Save and verify file written
  - [ ] Check permissions on written files
- [ ] Test policy application workflow
  - [ ] Edit policy in Admin UI
  - [ ] Preview changes
  - [ ] Apply policies
  - [ ] Verify state file updated
  - [ ] Verify User UI reflects changes
- [ ] Test daemon integration
  - [ ] Start daemon
  - [ ] Modify config in Admin UI
  - [ ] Verify daemon picks up changes
  - [ ] Check state file updates

#### 6.3 Platform-Specific Testing ✗
- [ ] **Linux Testing**
  - [ ] Test `pkexec` elevation
  - [ ] Test `sudo` fallback
  - [ ] Test state file permissions
  - [ ] Test systray icon display
  - [ ] Test all CLI commands
  - [ ] Test daemon service integration
- [ ] **macOS Testing**
  - [ ] Test `osascript` elevation
  - [ ] Test state file permissions (macOS path)
  - [ ] Test systray/menu bar icon
  - [ ] Test all CLI commands
  - [ ] Test launchd integration
- [ ] **Windows Testing**
  - [ ] Test UAC elevation
  - [ ] Test state file permissions (Windows path)
  - [ ] Test system tray icon
  - [ ] Test all CLI commands
  - [ ] Test Windows Service integration

#### 6.4 End-to-End Scenarios ✗
- [ ] Scenario: Fresh install
  - [ ] Install app
  - [ ] Launch User UI (no policies applied yet)
  - [ ] Launch Admin UI from User UI
  - [ ] Configure policies
  - [ ] Apply policies
  - [ ] Verify in User UI
- [ ] Scenario: Update policies
  - [ ] Start with applied policies
  - [ ] Launch Admin UI
  - [ ] Modify policies
  - [ ] Preview changes (verify diff correct)
  - [ ] Apply changes
  - [ ] Verify in User UI
- [ ] Scenario: Remove all policies
  - [ ] Start with applied policies
  - [ ] Launch Admin UI
  - [ ] Use uninstall/remove feature
  - [ ] Verify all policies removed
  - [ ] Verify User UI shows no policies
- [ ] Scenario: Daemon auto-apply
  - [ ] Start daemon
  - [ ] Push policy update to GitHub (agent mode)
  - [ ] Verify daemon detects and applies
  - [ ] Verify User UI shows updated status
- [ ] Scenario: Dry-run as regular user
  - [ ] As regular user, run `family-policy apply --dry-run --config test.yaml`
  - [ ] Verify diff is shown
  - [ ] Verify no policies actually applied
  - [ ] Verify appropriate message shown

---

### Phase 7: Documentation Updates ✗

Update all documentation to reflect new architecture.

#### 7.1 Update README.md ✗
- [ ] Update project overview with three modes
- [ ] Update installation instructions
- [ ] Add User UI usage instructions
- [ ] Add Admin UI usage instructions
- [ ] Update CLI command examples
- [ ] Add privilege requirements section
- [ ] Update screenshots (if any)
- [ ] Add troubleshooting section for elevation issues

#### 7.2 Update CLAUDE.md ✗
- [ ] Update build commands
- [ ] Update architecture description
- [ ] Add new modules to code organization
- [ ] Update privilege model description
- [ ] Add notes about state file permissions
- [ ] Update agent mode description

#### 7.3 Update DESIGN.md ✗
- [ ] Add multi-mode architecture section
- [ ] Document User UI design
- [ ] Document Admin UI design
- [ ] Document privilege separation model
- [ ] Update file-by-file design with new modules
- [ ] Add elevation mechanism documentation

#### 7.4 Create User Guide ✗
- [ ] Create `docs/USER_GUIDE.md`
- [ ] Explain User UI features
- [ ] Explain how to check policy status
- [ ] Explain how to launch Admin UI
- [ ] Add troubleshooting for common issues
- [ ] Add FAQ section

#### 7.5 Create Admin Guide ✗
- [ ] Create `docs/ADMIN_GUIDE.md`
- [ ] Explain Admin UI features
- [ ] Document policy configuration format
- [ ] Document agent configuration options
- [ ] Explain preview and apply workflow
- [ ] Add best practices section
- [ ] Add security considerations

---

### Phase 8: Build & Deployment ✗

Update build process and create distribution packages.

#### 8.1 Update Build Configuration ✗
- [ ] Update `Cargo.toml` with new dependencies
  - [ ] Add `windows-sys` for elevation (Windows)
  - [ ] Verify all dependencies are correct
- [ ] Update `src-tauri/tauri.conf.json`
  - [ ] Configure app name and version
  - [ ] Configure icons
  - [ ] Configure system tray support
  - [ ] Configure permissions
- [ ] Update `.github/workflows/` CI configuration
  - [ ] Build for Linux (AppImage, deb)
  - [ ] Build for macOS (dmg, app)
  - [ ] Build for Windows (msi, exe)
  - [ ] Run tests before building
- [ ] Test local builds on all platforms

#### 8.2 Create Installers ✗
- [ ] **Linux Installer**
  - [ ] Create .deb package
  - [ ] Create AppImage
  - [ ] Include systemd service file
  - [ ] Add post-install script to set up service
  - [ ] Test installation and uninstallation
- [ ] **macOS Installer**
  - [ ] Create .dmg
  - [ ] Create .app bundle
  - [ ] Include launchd plist
  - [ ] Add post-install script
  - [ ] Sign binaries (if certificates available)
  - [ ] Test installation and uninstallation
- [ ] **Windows Installer**
  - [ ] Create MSI installer
  - [ ] Include Windows Service setup
  - [ ] Add post-install service installation
  - [ ] Sign binaries (if certificates available)
  - [ ] Test installation and uninstallation

#### 8.3 Release Preparation ✗
- [ ] Tag version in git
- [ ] Create CHANGELOG.md with changes
- [ ] Create GitHub release
- [ ] Upload build artifacts
- [ ] Update download links in README
- [ ] Announce release

---

## Progress Summary

**Overall Progress**: 0% (0/151 tasks completed)

### Phase Completion
- Phase 1: Core Refactoring - 0% (0/28 tasks)
- Phase 2: CLI Enhancement - 0% (0/13 tasks)
- Phase 3: User UI Implementation - 0% (0/31 tasks)
- Phase 4: Admin UI Implementation - 0% (0/32 tasks)
- Phase 5: Frontend Routing & UI Polish - 0% (0/13 tasks)
- Phase 6: Integration & Testing - 0% (0/25 tasks)
- Phase 7: Documentation Updates - 0% (0/20 tasks)
- Phase 8: Build & Deployment - 0% (0/13 tasks)

---

## Notes & Decisions

### Key Architectural Decisions
- State file made world-readable (0o644) to allow User UI to read status
- Privilege elevation uses platform-native APIs (pkexec, osascript, ShellExecute)
- System service installation handled by installer, not by app itself
- User UI and Admin UI share same Vue codebase with different routes
- Daemon checks config hash on each poll cycle (no inotify initially)

### Deferred Features (Post-MVP)
- Real-time state file watching (inotify/FSEvents)
- IPC for direct daemon control from UI
- Notification system for policy changes
- Multi-user support
- Remote management capabilities

### Implementation Order Rationale
1. Core refactoring first to establish shared logic
2. CLI enhancement to support new modes
3. User UI next to establish read-only patterns
4. Admin UI builds on User UI patterns
5. Integration testing ensures all modes work together
6. Documentation and deployment complete the release

---

## Development Commands

### Run Tests
```bash
# All tests
cargo test

# Specific module
cargo test core::privileges

# With output
cargo test -- --nocapture
```

### Run User UI (Development)
```bash
# From project root
pnpm tauri dev -- user-ui --window
```

### Run Admin UI (Development)
```bash
# From project root (must have sudo/admin)
sudo pnpm tauri dev -- admin-ui
```

### Build
```bash
# Development build
pnpm tauri build

# Production build in container
devcontainer exec --workspace-folder /var/home/eitan/projects/family-policy pnpm tauri build --bundles appimage
```

---

**Last Updated**: 2025-11-24
**Current Phase**: Phase 1 - Core Refactoring
**Next Milestone**: Complete privilege checking and state file permission updates
