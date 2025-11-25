# Family Policy - Detailed Implementation Plan

This document provides file-by-file and function-by-function implementation details for the multi-mode architecture.

## Table of Contents
1. [Core Module Implementation](#core-module-implementation)
2. [CLI Enhancement](#cli-enhancement)
3. [User UI Implementation](#user-ui-implementation)
4. [Admin UI Implementation](#admin-ui-implementation)
5. [State Management Updates](#state-management-updates)
6. [Testing Strategy](#testing-strategy)

---

## Core Module Implementation

### File: `src-tauri/src/core/mod.rs`

**Purpose**: Module exports for core business logic

```rust
pub mod apply;
pub mod diff;
pub mod privileges;

// Re-export commonly used items
pub use apply::{apply_policies_from_config, remove_all_policies, ApplyResult, RemovalResult};
pub use diff::{generate_diff, PolicyDiff, BrowserDiff, ExtensionDiff};
pub use privileges::{check_privileges, is_admin, PrivilegeCheck, PrivilegeLevel};
```

---

### File: `src-tauri/src/core/privileges.rs`

**Purpose**: Centralized privilege checking and elevation logic

#### Data Structures

```rust
use anyhow::{Context, Result};

/// Privilege levels for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeLevel {
    /// Regular user can perform this operation
    User,
    /// Admin/root privileges required
    Admin,
}

/// Privilege check configuration
#[derive(Debug, Clone)]
pub struct PrivilegeCheck {
    /// Required privilege level
    pub required: PrivilegeLevel,
    /// Whether dry-run bypasses admin requirement
    pub allow_dry_run: bool,
}

impl PrivilegeCheck {
    /// Create a check that requires admin privileges
    pub fn admin() -> Self {
        Self {
            required: PrivilegeLevel::Admin,
            allow_dry_run: false,
        }
    }

    /// Create a check that requires admin but allows dry-run for users
    pub fn admin_or_dry_run() -> Self {
        Self {
            required: PrivilegeLevel::Admin,
            allow_dry_run: true,
        }
    }

    /// Create a check that allows any user
    pub fn user() -> Self {
        Self {
            required: PrivilegeLevel::User,
            allow_dry_run: false,
        }
    }
}
```

#### Functions

```rust
/// Check if current process has required privileges
///
/// # Arguments
/// * `check` - Privilege requirements
/// * `is_dry_run` - Whether this is a dry-run operation
///
/// # Returns
/// * `Ok(())` if privileges are sufficient
/// * `Err` if privileges are insufficient
pub fn check_privileges(check: PrivilegeCheck, is_dry_run: bool) -> Result<()> {
    match check.required {
        PrivilegeLevel::User => {
            // Anyone can run user-level operations
            Ok(())
        }
        PrivilegeLevel::Admin => {
            // If dry-run is allowed and this is a dry-run, permit regular users
            if is_dry_run && check.allow_dry_run {
                return Ok(());
            }

            // Otherwise, require admin privileges
            if is_admin() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "This operation requires administrator privileges.\n\
                     Please run as root (Linux/macOS) or Administrator (Windows)."
                ))
            }
        }
    }
}

/// Check if the current process is running with admin/root privileges
///
/// # Returns
/// * `true` if running as admin/root
/// * `false` otherwise
pub fn is_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_is_admin()
    }

    #[cfg(unix)]
    {
        unix_is_admin()
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    {
        // Unknown platform, assume not admin
        false
    }
}

#[cfg(unix)]
fn unix_is_admin() -> bool {
    // Check if effective UID is 0 (root)
    unsafe { libc::geteuid() == 0 }
}

#[cfg(target_os = "windows")]
fn windows_is_admin() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_length = 0u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        CloseHandle(token);

        result != 0 && elevation.TokenIsElevated != 0
    }
}
```

#### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_check_user_level() {
        let check = PrivilegeCheck::user();
        assert!(check_privileges(check, false).is_ok());
        assert!(check_privileges(check, true).is_ok());
    }

    #[test]
    fn test_privilege_check_admin_with_dry_run() {
        let check = PrivilegeCheck::admin_or_dry_run();

        // Dry-run should always pass
        assert!(check_privileges(check.clone(), true).is_ok());

        // Non-dry-run depends on actual privileges
        // (We can't test this reliably without mocking)
    }

    #[test]
    fn test_is_admin() {
        // Just verify the function doesn't panic
        let _ = is_admin();
    }
}
```

---

### File: `src-tauri/src/core/apply.rs`

**Purpose**: Policy application orchestration

#### Data Structures

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::config::Config;
use crate::state::{State, load_state, save_state, compute_config_hash};
use crate::policy;

/// Result of applying policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    /// Whether policies were actually changed
    pub changed: bool,
    /// Number of extensions applied per browser
    pub extensions_applied: BrowserCounts,
    /// Privacy settings applied per browser
    pub privacy_settings_applied: BrowserCounts,
    /// Errors encountered (if any)
    pub errors: Vec<String>,
    /// Warnings (if any)
    pub warnings: Vec<String>,
}

/// Result of removing policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovalResult {
    /// Number of extensions removed per browser
    pub extensions_removed: BrowserCounts,
    /// Number of privacy settings removed per browser
    pub privacy_settings_removed: BrowserCounts,
    /// Errors encountered (if any)
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserCounts {
    pub chrome: usize,
    pub firefox: usize,
    pub edge: usize,
}
```

#### Functions

```rust
/// Apply policies from a configuration
///
/// # Arguments
/// * `config` - Policy configuration to apply
/// * `dry_run` - If true, only show what would be done
///
/// # Returns
/// * `ApplyResult` with details of what was applied
pub fn apply_policies_from_config(config: &Config, dry_run: bool) -> Result<ApplyResult> {
    // Compute hash of new config
    let config_hash = compute_config_hash(config);

    // Load current state
    let current_state = load_state().ok().flatten();

    // Check if config has changed
    let changed = current_state
        .as_ref()
        .map(|s| s.config_hash != config_hash)
        .unwrap_or(true);

    if !changed && !dry_run {
        return Ok(ApplyResult {
            changed: false,
            extensions_applied: BrowserCounts::default(),
            privacy_settings_applied: BrowserCounts::default(),
            errors: vec![],
            warnings: vec!["No changes detected, policies already applied".to_string()],
        });
    }

    // Convert config to browser-specific configs
    let (chrome_config, firefox_config, edge_config) = config.to_browser_configs();

    let mut result = ApplyResult {
        changed,
        extensions_applied: BrowserCounts::default(),
        privacy_settings_applied: BrowserCounts::default(),
        errors: vec![],
        warnings: vec![],
    };

    if dry_run {
        // For dry-run, just count what would be applied
        if let Some(ref chrome) = chrome_config {
            result.extensions_applied.chrome = chrome.extensions.len();
            result.privacy_settings_applied.chrome = count_privacy_settings(chrome);
        }
        if let Some(ref firefox) = firefox_config {
            result.extensions_applied.firefox = firefox.extensions.len();
            result.privacy_settings_applied.firefox = count_privacy_settings(firefox);
        }
        if let Some(ref edge) = edge_config {
            result.extensions_applied.edge = edge.extensions.len();
            result.privacy_settings_applied.edge = count_privacy_settings(edge);
        }

        return Ok(result);
    }

    // Apply Chrome policies
    if let Some(chrome) = chrome_config {
        match policy::chrome::apply_chrome_policies(&chrome) {
            Ok(state) => {
                result.extensions_applied.chrome = state.extensions.len();
                result.privacy_settings_applied.chrome = count_privacy_in_state(&state);
            }
            Err(e) => {
                result.errors.push(format!("Chrome: {}", e));
            }
        }
    }

    // Apply Firefox policies
    if let Some(firefox) = firefox_config {
        match policy::firefox::apply_firefox_policies(&firefox) {
            Ok(state) => {
                result.extensions_applied.firefox = state.extensions.len();
                result.privacy_settings_applied.firefox = count_privacy_in_state(&state);
            }
            Err(e) => {
                result.errors.push(format!("Firefox: {}", e));
            }
        }
    }

    // Apply Edge policies
    if let Some(edge) = edge_config {
        match policy::edge::apply_edge_policies(&edge) {
            Ok(state) => {
                result.extensions_applied.edge = state.extensions.len();
                result.privacy_settings_applied.edge = count_privacy_in_state(&state);
            }
            Err(e) => {
                result.errors.push(format!("Edge: {}", e));
            }
        }
    }

    // Save new state
    let new_state = State {
        version: "1.0".to_string(),
        config_hash,
        last_updated: chrono::Utc::now(),
        applied_policies: create_applied_policies_from_configs(
            chrome_config,
            firefox_config,
            edge_config,
        ),
    };

    save_state(&new_state).context("Failed to save state")?;

    Ok(result)
}

/// Remove all applied policies
///
/// # Arguments
/// * `dry_run` - If true, only show what would be removed
///
/// # Returns
/// * `RemovalResult` with details of what was removed
pub fn remove_all_policies(dry_run: bool) -> Result<RemovalResult> {
    let current_state = load_state()
        .context("Failed to load state")?
        .ok_or_else(|| anyhow::anyhow!("No state file found, nothing to remove"))?;

    let mut result = RemovalResult {
        extensions_removed: BrowserCounts::default(),
        privacy_settings_removed: BrowserCounts::default(),
        errors: vec![],
    };

    // Count what will be removed
    if let Some(ref chrome) = current_state.applied_policies.chrome {
        result.extensions_removed.chrome = chrome.extensions.len();
        result.privacy_settings_removed.chrome = count_privacy_in_state(chrome);
    }
    if let Some(ref firefox) = current_state.applied_policies.firefox {
        result.extensions_removed.firefox = firefox.extensions.len();
        result.privacy_settings_removed.firefox = count_privacy_in_state(firefox);
    }
    if let Some(ref edge) = current_state.applied_policies.edge {
        result.extensions_removed.edge = edge.extensions.len();
        result.privacy_settings_removed.edge = count_privacy_in_state(edge);
    }

    if dry_run {
        return Ok(result);
    }

    // Actually remove policies
    if current_state.applied_policies.chrome.is_some() {
        if let Err(e) = policy::chrome::remove_chrome_policies() {
            result.errors.push(format!("Chrome: {}", e));
        }
    }

    if current_state.applied_policies.firefox.is_some() {
        if let Err(e) = policy::firefox::remove_firefox_policies() {
            result.errors.push(format!("Firefox: {}", e));
        }
    }

    if current_state.applied_policies.edge.is_some() {
        if let Err(e) = policy::edge::remove_edge_policies() {
            result.errors.push(format!("Edge: {}", e));
        }
    }

    // Remove state file
    let state_path = crate::state::get_state_path()?;
    if state_path.exists() {
        std::fs::remove_file(&state_path)
            .context("Failed to remove state file")?;
    }

    Ok(result)
}

// Helper functions

fn count_privacy_settings<T>(config: &T) -> usize
where
    T: HasPrivacySettings,
{
    config.privacy_setting_count()
}

fn count_privacy_in_state(state: &crate::state::BrowserState) -> usize {
    let mut count = 0;
    if state.disable_incognito.is_some() {
        count += 1;
    }
    if state.disable_inprivate.is_some() {
        count += 1;
    }
    if state.disable_private_browsing.is_some() {
        count += 1;
    }
    if state.disable_guest_mode.is_some() {
        count += 1;
    }
    count
}

trait HasPrivacySettings {
    fn privacy_setting_count(&self) -> usize;
}

impl HasPrivacySettings for crate::config::ChromeConfig {
    fn privacy_setting_count(&self) -> usize {
        let mut count = 0;
        if self.disable_incognito.is_some() {
            count += 1;
        }
        if self.disable_guest_mode.is_some() {
            count += 1;
        }
        count
    }
}

impl HasPrivacySettings for crate::config::FirefoxConfig {
    fn privacy_setting_count(&self) -> usize {
        if self.disable_private_browsing.is_some() {
            1
        } else {
            0
        }
    }
}

impl HasPrivacySettings for crate::config::EdgeConfig {
    fn privacy_setting_count(&self) -> usize {
        let mut count = 0;
        if self.disable_inprivate.is_some() {
            count += 1;
        }
        if self.disable_guest_mode.is_some() {
            count += 1;
        }
        count
    }
}

fn create_applied_policies_from_configs(
    chrome: Option<crate::config::ChromeConfig>,
    firefox: Option<crate::config::FirefoxConfig>,
    edge: Option<crate::config::EdgeConfig>,
) -> crate::state::AppliedPolicies {
    // Implementation: convert browser configs to BrowserState
    // This is a helper to create the state structure
    todo!("Convert browser configs to state structure")
}
```

---

### File: `src-tauri/src/core/diff.rs`

**Purpose**: Generate diffs between current state and proposed configuration

#### Data Structures

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::Config;
use crate::state::State;
use crate::browser::Browser;

/// Complete policy diff across all browsers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDiff {
    pub chrome: Option<BrowserDiff>,
    pub firefox: Option<BrowserDiff>,
    pub edge: Option<BrowserDiff>,
    pub summary: DiffSummary,
}

/// Diff for a single browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserDiff {
    pub browser: Browser,
    pub extensions: Vec<ExtensionDiff>,
    pub privacy_settings: Vec<PrivacySettingDiff>,
}

/// Diff for an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionDiff {
    Added {
        id: String,
        name: String,
    },
    Removed {
        id: String,
        name: Option<String>,
    },
    Unchanged {
        id: String,
        name: String,
    },
}

/// Diff for a privacy setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettingDiff {
    pub setting_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// Summary of changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_additions: usize,
    pub total_removals: usize,
    pub total_changes: usize,
}
```

#### Functions

```rust
/// Generate a diff between proposed config and current state
///
/// # Arguments
/// * `new_config` - Proposed configuration
/// * `current_state` - Current applied state (if any)
///
/// # Returns
/// * `PolicyDiff` describing all changes
pub fn generate_diff(new_config: &Config, current_state: Option<&State>) -> PolicyDiff {
    let (new_chrome, new_firefox, new_edge) = new_config.to_browser_configs();

    let chrome_diff = if let Some(chrome_config) = new_chrome {
        let current_chrome = current_state
            .and_then(|s| s.applied_policies.chrome.as_ref());
        Some(generate_browser_diff(
            Browser::Chrome,
            &chrome_config,
            current_chrome,
        ))
    } else {
        None
    };

    let firefox_diff = if let Some(firefox_config) = new_firefox {
        let current_firefox = current_state
            .and_then(|s| s.applied_policies.firefox.as_ref());
        Some(generate_browser_diff(
            Browser::Firefox,
            &firefox_config,
            current_firefox,
        ))
    } else {
        None
    };

    let edge_diff = if let Some(edge_config) = new_edge {
        let current_edge = current_state
            .and_then(|s| s.applied_policies.edge.as_ref());
        Some(generate_browser_diff(
            Browser::Edge,
            &edge_config,
            current_edge,
        ))
    } else {
        None
    };

    let summary = create_summary(&chrome_diff, &firefox_diff, &edge_diff);

    PolicyDiff {
        chrome: chrome_diff,
        firefox: firefox_diff,
        edge: edge_diff,
        summary,
    }
}

fn generate_browser_diff<T>(
    browser: Browser,
    new_config: &T,
    current_state: Option<&crate::state::BrowserState>,
) -> BrowserDiff
where
    T: BrowserConfigTrait,
{
    let mut extensions = Vec::new();

    // Get current extension IDs
    let current_ids: HashMap<String, ()> = current_state
        .map(|s| s.extensions.iter().map(|id| (id.clone(), ())).collect())
        .unwrap_or_default();

    // Get new extension IDs
    let new_ids: HashMap<String, String> = new_config
        .extensions()
        .iter()
        .map(|ext| (ext.id.clone(), ext.name.clone()))
        .collect();

    // Find additions and unchanged
    for (id, name) in &new_ids {
        if current_ids.contains_key(id) {
            extensions.push(ExtensionDiff::Unchanged {
                id: id.clone(),
                name: name.clone(),
            });
        } else {
            extensions.push(ExtensionDiff::Added {
                id: id.clone(),
                name: name.clone(),
            });
        }
    }

    // Find removals
    for id in current_ids.keys() {
        if !new_ids.contains_key(id) {
            extensions.push(ExtensionDiff::Removed {
                id: id.clone(),
                name: None,
            });
        }
    }

    // Generate privacy settings diff
    let privacy_settings = generate_privacy_diff(new_config, current_state);

    BrowserDiff {
        browser,
        extensions,
        privacy_settings,
    }
}

fn generate_privacy_diff<T>(
    new_config: &T,
    current_state: Option<&crate::state::BrowserState>,
) -> Vec<PrivacySettingDiff>
where
    T: BrowserConfigTrait,
{
    // Compare privacy settings between new config and current state
    // Return list of differences
    todo!("Implement privacy setting diff")
}

fn create_summary(
    chrome: &Option<BrowserDiff>,
    firefox: &Option<BrowserDiff>,
    edge: &Option<BrowserDiff>,
) -> DiffSummary {
    let mut total_additions = 0;
    let mut total_removals = 0;
    let mut total_changes = 0;

    for diff in [chrome, firefox, edge].iter().filter_map(|d| d.as_ref()) {
        for ext_diff in &diff.extensions {
            match ext_diff {
                ExtensionDiff::Added { .. } => total_additions += 1,
                ExtensionDiff::Removed { .. } => total_removals += 1,
                ExtensionDiff::Unchanged { .. } => {}
            }
        }
        total_changes += diff.privacy_settings.len();
    }

    DiffSummary {
        total_additions,
        total_removals,
        total_changes,
    }
}

/// Trait for browser-specific configs
trait BrowserConfigTrait {
    fn extensions(&self) -> &[crate::config::Extension];
}

impl BrowserConfigTrait for crate::config::ChromeConfig {
    fn extensions(&self) -> &[crate::config::Extension] {
        &self.extensions
    }
}

impl BrowserConfigTrait for crate::config::FirefoxConfig {
    fn extensions(&self) -> &[crate::config::Extension] {
        &self.extensions
    }
}

impl BrowserConfigTrait for crate::config::EdgeConfig {
    fn extensions(&self) -> &[crate::config::Extension] {
        &self.extensions
    }
}

/// Pretty-print a diff for CLI output
pub fn print_diff(diff: &PolicyDiff) {
    println!("Policy Changes:");
    println!();

    if let Some(chrome_diff) = &diff.chrome {
        print_browser_diff("Chrome", chrome_diff);
    }

    if let Some(firefox_diff) = &diff.firefox {
        print_browser_diff("Firefox", firefox_diff);
    }

    if let Some(edge_diff) = &diff.edge {
        print_browser_diff("Edge", edge_diff);
    }

    println!();
    println!("Summary:");
    println!("  Additions: {}", diff.summary.total_additions);
    println!("  Removals: {}", diff.summary.total_removals);
    println!("  Changes: {}", diff.summary.total_changes);
}

fn print_browser_diff(browser_name: &str, diff: &BrowserDiff) {
    println!("{}:", browser_name);

    for ext_diff in &diff.extensions {
        match ext_diff {
            ExtensionDiff::Added { id, name } => {
                println!("  + Add extension: {} ({})", name, id);
            }
            ExtensionDiff::Removed { id, name } => {
                let name_str = name.as_deref().unwrap_or("unknown");
                println!("  - Remove extension: {} ({})", name_str, id);
            }
            ExtensionDiff::Unchanged { .. } => {
                // Don't print unchanged items unless verbose mode
            }
        }
    }

    for privacy_diff in &diff.privacy_settings {
        println!(
            "  ~ {}: {:?} -> {:?}",
            privacy_diff.setting_name, privacy_diff.old_value, privacy_diff.new_value
        );
    }

    println!();
}
```

---

## CLI Enhancement

### File: `src-tauri/src/cli.rs` (Updates)

Add new command variants:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Apply policies from a local configuration file (default command)
    Apply,

    /// Configuration file management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Run as daemon (foreground mode)
    Daemon,

    /// Start agent daemon (background)
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        no_daemon: bool,
    },

    /// Stop agent daemon
    Stop,

    /// Check for policy updates now (don't wait for next poll)
    CheckNow,

    /// Show agent status
    Status,

    /// Show currently applied configuration
    ShowConfig,

    /// Launch User UI (no admin required)
    UserUi {
        /// Run in system tray mode
        #[arg(long)]
        systray: bool,

        /// Run in window mode (default)
        #[arg(long)]
        window: bool,
    },

    /// Launch Admin UI (requires admin privileges)
    AdminUi,

    /// Install agent as a system service
    InstallService,

    /// Uninstall agent system service
    UninstallService,
}
```

### File: `src-tauri/src/main.rs` (Updates)

Update command routing with privilege checks:

```rust
use cli::{Args, Commands, ConfigCommands};
use core::privileges::{check_privileges, PrivilegeCheck};

fn run() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands with privilege checking
    match args.command {
        Some(Commands::Apply) | None => {
            // Require admin, but allow dry-run for regular users
            check_privileges(PrivilegeCheck::admin_or_dry_run(), args.dry_run)?;
            commands::run_local_mode(args)
        }

        Some(Commands::Config { command }) => {
            // Config init doesn't require admin
            check_privileges(PrivilegeCheck::user(), false)?;
            match command {
                ConfigCommands::Init { output, force } => {
                    commands::config::init(output, force, args.verbose)
                }
            }
        }

        Some(Commands::Daemon) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::daemon(args.verbose)
        }

        Some(Commands::Start { no_daemon }) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::start(no_daemon, args.verbose)
        }

        Some(Commands::Stop) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::stop(args.verbose)
        }

        Some(Commands::CheckNow) => {
            check_privileges(PrivilegeCheck::admin_or_dry_run(), args.dry_run)?;
            commands::agent::check_now(args.dry_run, args.verbose)
        }

        Some(Commands::Status) => {
            check_privileges(PrivilegeCheck::user(), false)?;
            commands::agent::status(args.verbose)
        }

        Some(Commands::ShowConfig) => {
            check_privileges(PrivilegeCheck::user(), false)?;
            commands::agent::show_config(args.verbose)
        }

        Some(Commands::UserUi { systray, window }) => {
            check_privileges(PrivilegeCheck::user(), false)?;
            let systray_mode = systray || !window; // Default to systray if neither specified
            ui::user::run(systray_mode)
        }

        Some(Commands::AdminUi) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            ui::admin::run()
        }

        Some(Commands::InstallService) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::install_service(args.verbose)
        }

        Some(Commands::UninstallService) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::uninstall_service(args.verbose)
        }
    }
}
```

---

## User UI Implementation

### File: `src-tauri/src/ui/user/mod.rs`

**Purpose**: User UI window setup and lifecycle

```rust
use anyhow::Result;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, WebviewWindow,
};

mod commands;
mod elevation;

/// Run the User UI
///
/// # Arguments
/// * `systray_mode` - If true, run in system tray mode; if false, show window
pub fn run(systray_mode: bool) -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            if systray_mode {
                setup_tray(app.handle())?;
                // Hide main window initially
                if let Some(window) = app.get_webview_window("main") {
                    window.hide()?;
                }
            } else {
                // Show window mode
                setup_window(app.handle())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_current_status,
            commands::get_policy_diff,
            commands::launch_admin_ui,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("Failed to run User UI: {}", e))?;

    Ok(())
}

fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(app, "status", "View Status", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Admin Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&status_item, &settings_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "status" => {
                show_status_window(app);
            }
            "settings" => {
                // Launch admin UI with elevation
                if let Err(e) = elevation::launch_admin_ui() {
                    eprintln!("Failed to launch admin UI: {}", e);
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                show_status_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

fn setup_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    // Window mode - main window is already created by Tauri
    // Just ensure it's visible and configured
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_title("Family Policy - Status")?;
    }
    Ok(())
}

fn show_status_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
```

### File: `src-tauri/src/ui/user/commands.rs`

**Purpose**: Tauri commands for User UI

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::core::{diff, privileges};
use crate::state::{load_state, State};
use crate::config::{load_config, Config};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInfo {
    pub has_state: bool,
    pub last_updated: Option<String>,
    pub chrome_extensions: usize,
    pub firefox_extensions: usize,
    pub edge_extensions: usize,
    pub is_daemon_running: bool,
}

#[tauri::command]
pub async fn get_current_status() -> Result<StatusInfo, String> {
    let state = load_state()
        .map_err(|e| format!("Failed to load state: {}", e))?;

    if let Some(state) = state {
        Ok(StatusInfo {
            has_state: true,
            last_updated: Some(state.last_updated.to_rfc3339()),
            chrome_extensions: state
                .applied_policies
                .chrome
                .as_ref()
                .map(|c| c.extensions.len())
                .unwrap_or(0),
            firefox_extensions: state
                .applied_policies
                .firefox
                .as_ref()
                .map(|f| f.extensions.len())
                .unwrap_or(0),
            edge_extensions: state
                .applied_policies
                .edge
                .as_ref()
                .map(|e| e.extensions.len())
                .unwrap_or(0),
            is_daemon_running: check_daemon_status(),
        })
    } else {
        Ok(StatusInfo {
            has_state: false,
            last_updated: None,
            chrome_extensions: 0,
            firefox_extensions: 0,
            edge_extensions: 0,
            is_daemon_running: check_daemon_status(),
        })
    }
}

#[tauri::command]
pub async fn get_policy_diff(policy_path: String) -> Result<diff::PolicyDiff, String> {
    let config_path = PathBuf::from(policy_path);
    let config = load_config(&config_path)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    let current_state = load_state()
        .map_err(|e| format!("Failed to load state: {}", e))?;

    let diff = diff::generate_diff(&config, current_state.as_ref());

    Ok(diff)
}

#[tauri::command]
pub async fn launch_admin_ui() -> Result<(), String> {
    super::elevation::launch_admin_ui()
        .map_err(|e| format!("Failed to launch admin UI: {}", e))
}

fn check_daemon_status() -> bool {
    // Check if daemon process is running
    // This could check for PID file, systemd status, etc.
    // For now, return false as placeholder
    false
}
```

### File: `src-tauri/src/ui/user/elevation.rs`

**Purpose**: Platform-specific privilege elevation

```rust
use anyhow::{Context, Result};
use std::env;
use std::process::Command;

/// Launch the Admin UI with elevated privileges
pub fn launch_admin_ui() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        launch_admin_ui_linux()
    }

    #[cfg(target_os = "macos")]
    {
        launch_admin_ui_macos()
    }

    #[cfg(target_os = "windows")]
    {
        launch_admin_ui_windows()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(anyhow::anyhow!("Privilege elevation not supported on this platform"))
    }
}

#[cfg(target_os = "linux")]
fn launch_admin_ui_linux() -> Result<()> {
    let exe_path = env::current_exe()
        .context("Failed to get current executable path")?;

    // Try pkexec first (graphical, PolicyKit-based)
    let pkexec_result = Command::new("pkexec")
        .arg(&exe_path)
        .arg("admin-ui")
        .spawn();

    if pkexec_result.is_ok() {
        return Ok(());
    }

    // Fallback to terminal-based sudo
    Command::new("x-terminal-emulator")
        .arg("-e")
        .arg(format!("sudo {} admin-ui; read -p 'Press Enter to close...'", exe_path.display()))
        .spawn()
        .context("Failed to launch admin UI with sudo")?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_admin_ui_macos() -> Result<()> {
    let exe_path = env::current_exe()
        .context("Failed to get current executable path")?;

    let script = format!(
        r#"do shell script "{} admin-ui" with administrator privileges"#,
        exe_path.display()
    );

    Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn()
        .context("Failed to launch admin UI with osascript")?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_admin_ui_windows() -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe_path = env::current_exe()
        .context("Failed to get current executable path")?;

    let operation = "runas"
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();

    let file = exe_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();

    let params = "admin-ui"
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();

    unsafe {
        let result = ShellExecuteW(
            0,
            operation.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );

        // ShellExecuteW returns a value > 32 on success
        if result as isize <= 32 {
            return Err(anyhow::anyhow!(
                "ShellExecuteW failed with code: {}",
                result as isize
            ));
        }
    }

    Ok(())
}
```

---

## Admin UI Implementation

### File: `src-tauri/src/ui/admin/mod.rs`

**Purpose**: Admin UI window setup and lifecycle

```rust
use anyhow::Result;
use tauri::{AppHandle, Manager, WebviewWindow};
use crate::core::privileges;

mod commands;
mod config_editor;

/// Run the Admin UI
pub fn run() -> Result<()> {
    // Verify admin privileges at startup
    if !privileges::is_admin() {
        return Err(anyhow::anyhow!(
            "Admin UI requires administrator privileges.\n\
             Please run with sudo (Linux/macOS) or as Administrator (Windows)."
        ));
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_admin_window(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_agent_config,
            commands::save_agent_config,
            commands::get_policy_config,
            commands::save_policy_config,
            commands::preview_policy_changes,
            commands::apply_policies,
            commands::control_daemon,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("Failed to run Admin UI: {}", e))?;

    Ok(())
}

fn setup_admin_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_title("Family Policy - Admin Settings")?;
        window.show()?;
    }
    Ok(())
}
```

### File: `src-tauri/src/ui/admin/commands.rs`

**Purpose**: Tauri commands for Admin UI

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::agent::config::{AgentConfig, get_agent_config_path};
use crate::config::{Config, load_config};
use crate::core::{diff, apply};
use super::config_editor;

#[tauri::command]
pub async fn get_agent_config() -> Result<AgentConfig, String> {
    config_editor::load_agent_config()
        .map_err(|e| format!("Failed to load agent config: {}", e))
}

#[tauri::command]
pub async fn save_agent_config(config: AgentConfig) -> Result<(), String> {
    // Validate first
    config_editor::validate_agent_config(&config)
        .map_err(|e| format!("Invalid config: {}", e))?;

    // Save
    config_editor::save_agent_config(&config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_policy_config(path: String) -> Result<Config, String> {
    let config_path = PathBuf::from(path);
    load_config(&config_path)
        .map_err(|e| format!("Failed to load policy config: {}", e))
}

#[tauri::command]
pub async fn save_policy_config(path: String, config: Config) -> Result<(), String> {
    let config_path = PathBuf::from(path);

    // Validate first
    config_editor::validate_policy_config(&config)
        .map_err(|e| format!("Invalid policy config: {}", e))?;

    // Save
    config_editor::save_policy_config(&config_path, &config)
        .map_err(|e| format!("Failed to save policy config: {}", e))
}

#[tauri::command]
pub async fn preview_policy_changes(config: Config) -> Result<diff::PolicyDiff, String> {
    let current_state = crate::state::load_state()
        .map_err(|e| format!("Failed to load state: {}", e))?;

    let diff = diff::generate_diff(&config, current_state.as_ref());
    Ok(diff)
}

#[tauri::command]
pub async fn apply_policies(config: Config) -> Result<apply::ApplyResult, String> {
    apply::apply_policies_from_config(&config, false)
        .map_err(|e| format!("Failed to apply policies: {}", e))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DaemonAction {
    Start,
    Stop,
    Restart,
}

#[tauri::command]
pub async fn control_daemon(action: DaemonAction) -> Result<(), String> {
    match action {
        DaemonAction::Start => {
            // Start daemon
            todo!("Implement daemon start")
        }
        DaemonAction::Stop => {
            // Stop daemon
            todo!("Implement daemon stop")
        }
        DaemonAction::Restart => {
            // Restart daemon
            todo!("Implement daemon restart")
        }
    }
}
```

### File: `src-tauri/src/ui/admin/config_editor.rs`

**Purpose**: Config editing and validation logic

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use crate::agent::config::{AgentConfig, get_agent_config_path};
use crate::config::Config;

/// Load agent configuration
pub fn load_agent_config() -> Result<AgentConfig> {
    let path = get_agent_config_path()?;

    if !path.exists() {
        return Ok(AgentConfig::default());
    }

    let content = fs::read_to_string(&path)
        .context("Failed to read agent config file")?;

    let config: AgentConfig = toml::from_str(&content)
        .context("Failed to parse agent config")?;

    Ok(config)
}

/// Save agent configuration
pub fn save_agent_config(config: &AgentConfig) -> Result<()> {
    let path = get_agent_config_path()?;

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize to TOML
    let toml_content = toml::to_string_pretty(config)
        .context("Failed to serialize agent config")?;

    // Write atomically
    crate::platform::common::atomic_write(&path, toml_content.as_bytes())?;

    // Set restrictive permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

/// Validate agent configuration
pub fn validate_agent_config(config: &AgentConfig) -> Result<()> {
    // Check URL is HTTPS
    if !config.policy_url.starts_with("https://") {
        return Err(anyhow::anyhow!("Policy URL must use HTTPS"));
    }

    // Check poll interval is reasonable
    if config.poll_interval_seconds < 60 {
        return Err(anyhow::anyhow!("Poll interval must be at least 60 seconds"));
    }

    Ok(())
}

/// Save policy configuration
pub fn save_policy_config(path: &Path, config: &Config) -> Result<()> {
    let yaml_content = serde_yaml::to_string(config)
        .context("Failed to serialize policy config")?;

    crate::platform::common::atomic_write(path, yaml_content.as_bytes())?;

    Ok(())
}

/// Validate policy configuration
pub fn validate_policy_config(config: &Config) -> Result<()> {
    // Use existing validation from config module
    crate::config::validate_config(config)
}
```

---

## State Management Updates

### File: `src-tauri/src/state.rs` (Updates)

Update `save_state()` to make state file world-readable:

```rust
pub fn save_state(state: &State) -> Result<()> {
    let path = get_state_path()?;

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize state to JSON
    let json = serde_json::to_string_pretty(state)
        .context("Failed to serialize state")?;

    // Write atomically
    crate::platform::common::atomic_write(&path, json.as_bytes())?;

    // Set world-readable permissions (0o644 on Unix)
    // This allows User UI (running as regular user) to read the state
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o644); // Changed from 0o600 - world-readable
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

Each module should have comprehensive unit tests:

```rust
// In src/core/privileges.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_levels() {
        // Test basic privilege level checks
    }

    #[test]
    fn test_dry_run_bypass() {
        // Test that dry-run bypasses admin requirement
    }
}

// In src/core/diff.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_additions() {
        // Test detecting added extensions
    }

    #[test]
    fn test_diff_removals() {
        // Test detecting removed extensions
    }

    #[test]
    fn test_diff_changes() {
        // Test detecting changed privacy settings
    }
}
```

### Integration Tests

```rust
// In tests/integration_tests.rs
#[test]
fn test_user_ui_can_read_state() {
    // Setup: create state file with 0o644 permissions
    // Test: read state as non-root user
    // Verify: state can be read
}

#[test]
fn test_admin_ui_requires_privileges() {
    // Test: try to launch admin UI without privileges
    // Verify: appropriate error is returned
}

#[test]
fn test_policy_application_workflow() {
    // Test: load config -> generate diff -> apply -> verify state
}
```

---

## Summary

This implementation plan provides:

1. **Detailed module structure** with clear responsibilities
2. **Function signatures** with documentation
3. **Platform-specific implementations** for elevation and privilege checking
4. **Data structures** for all new types (diffs, results, etc.)
5. **Integration points** between components
6. **Testing approach** for validation

The implementation should proceed in the order outlined in PROGRESS.md, starting with core refactoring and building up to the full UI implementation.
