use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Main time limits configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeLimitsConfig {
    /// Admin configuration
    pub admin: AdminConfig,

    /// Children profiles
    pub children: Vec<ChildProfile>,

    /// Shared login mode settings
    #[serde(default)]
    pub shared_login: SharedLoginConfig,

    /// Enforcement settings
    #[serde(default)]
    pub enforcement: EnforcementConfig,
}

/// Admin configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdminConfig {
    /// Argon2 password hash
    pub password_hash: String,

    /// OS user accounts that are exempt from time limits
    #[serde(default)]
    pub admin_accounts: Vec<String>,
}

/// Child profile with time limits
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChildProfile {
    /// Unique identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// OS user accounts that map to this child (empty for shared login mode)
    #[serde(default)]
    pub os_users: Vec<String>,

    /// Time limits configuration
    pub limits: TimeLimitSchedule,

    /// Warning thresholds in minutes before lockout
    #[serde(default = "default_warnings")]
    pub warnings: Vec<u32>,

    /// Grace period after final warning (seconds)
    #[serde(default = "default_grace_period")]
    pub grace_period: u64,
}

fn default_warnings() -> Vec<u32> {
    vec![15, 5, 1]
}

fn default_grace_period() -> u64 {
    60
}

/// Time limit schedule
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeLimitSchedule {
    /// Weekday limit (Monday-Friday)
    pub weekday: TimeLimit,

    /// Weekend limit (Saturday-Sunday)
    pub weekend: TimeLimit,

    /// Custom day overrides
    #[serde(default)]
    pub custom: Vec<CustomDayLimit>,
}

/// Time limit for a period
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct TimeLimit {
    pub hours: u32,
    pub minutes: u32,
}

impl TimeLimit {
    /// Convert to total seconds
    pub fn to_seconds(&self) -> i64 {
        (self.hours as i64 * 3600) + (self.minutes as i64 * 60)
    }

    /// Create from seconds
    pub fn from_seconds(seconds: i64) -> Self {
        let hours = (seconds / 3600) as u32;
        let minutes = ((seconds % 3600) / 60) as u32;
        Self { hours, minutes }
    }
}

/// Custom day limit override
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomDayLimit {
    /// Days this applies to (lowercase: monday, tuesday, etc.)
    pub days: Vec<String>,

    /// Time limit for these days
    #[serde(flatten)]
    pub limit: TimeLimit,
}

/// Shared login configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SharedLoginConfig {
    /// Whether shared login mode is enabled
    #[serde(default)]
    pub enabled: bool,

    /// OS accounts that are shared
    #[serde(default)]
    pub shared_accounts: Vec<String>,

    /// Require kid selection at startup
    #[serde(default = "default_true")]
    pub require_selection: bool,

    /// Allow switching kids during session
    #[serde(default)]
    pub allow_switching: bool,

    /// Auto-select if only one kid has time remaining
    #[serde(default)]
    pub auto_select_if_unique: bool,
}

impl Default for SharedLoginConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            shared_accounts: Vec::new(),
            require_selection: true,
            allow_switching: false,
            auto_select_if_unique: false,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Enforcement configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnforcementConfig {
    /// Action when time expires
    #[serde(default)]
    pub action: LockAction,

    /// Block system time changes while app is running
    #[serde(default = "default_true")]
    pub prevent_time_manipulation: bool,

    /// Require admin password to close/disable tracker
    #[serde(default = "default_true")]
    pub require_admin_to_quit: bool,

    /// Monitor for process tampering and restart
    #[serde(default = "default_true")]
    pub self_protection: bool,
}

impl Default for EnforcementConfig {
    fn default() -> Self {
        Self {
            action: LockAction::Lock,
            prevent_time_manipulation: true,
            require_admin_to_quit: true,
            self_protection: true,
        }
    }
}

/// Action to take when time limit is reached
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LockAction {
    /// Lock the workstation
    Lock,
    /// Log out the user
    Logout,
    /// Shutdown the computer
    Shutdown,
}

impl Default for LockAction {
    fn default() -> Self {
        Self::Lock
    }
}

/// Get the platform-specific config file path
pub fn get_config_path() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/etc/family-policy/time-limits-config.yaml"))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from(
            "/Library/Application Support/family-policy/time-limits-config.yaml",
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let mut path = PathBuf::from(
            std::env::var("ProgramData")
                .unwrap_or_else(|_| "C:\\ProgramData".to_string()),
        );
        path.push("family-policy");
        path.push("time-limits-config.yaml");
        Ok(path)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        anyhow::bail!("Unsupported operating system");
    }
}

/// Load configuration from YAML file
pub fn load_config(path: &Path) -> Result<TimeLimitsConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: TimeLimitsConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML config file: {}", path.display()))?;

    validate_config(&config)?;

    Ok(config)
}

/// Save configuration to YAML file
pub fn save_config(path: &Path, config: &TimeLimitsConfig) -> Result<()> {
    // Validate before saving
    validate_config(config)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    // Serialize to YAML
    let content = serde_yaml::to_string(config)
        .context("Failed to serialize config to YAML")?;

    // Write atomically
    crate::platform::common::atomic_write(path, content.as_bytes())
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

/// Validate configuration
pub fn validate_config(config: &TimeLimitsConfig) -> Result<()> {
    // Ensure at least one child is configured
    if config.children.is_empty() {
        anyhow::bail!("Configuration must specify at least one child");
    }

    // Validate child IDs are unique
    let mut ids = std::collections::HashSet::new();
    for child in &config.children {
        if !ids.insert(&child.id) {
            anyhow::bail!("Duplicate child ID: {}", child.id);
        }

        // Validate child profile
        validate_child_profile(child)
            .with_context(|| format!("Invalid child profile '{}'", child.name))?;
    }

    // Validate shared login configuration
    if config.shared_login.enabled {
        // Ensure children don't have os_users configured if shared login is enabled
        for child in &config.children {
            if !child.os_users.is_empty() {
                anyhow::bail!(
                    "Child '{}' has os_users configured, but shared_login mode is enabled. \
                     In shared login mode, children should not have os_users.",
                    child.name
                );
            }
        }

        // Ensure at least one shared account is configured
        if config.shared_login.shared_accounts.is_empty() {
            anyhow::bail!("Shared login mode is enabled but no shared_accounts are configured");
        }
    } else {
        // Individual login mode: ensure all children have os_users
        for child in &config.children {
            if child.os_users.is_empty() {
                anyhow::bail!(
                    "Child '{}' has no os_users configured. \
                     In individual login mode, each child must have at least one os_user.",
                    child.name
                );
            }
        }
    }

    Ok(())
}

/// Validate a child profile
fn validate_child_profile(child: &ChildProfile) -> Result<()> {
    // Ensure ID is not empty
    if child.id.is_empty() {
        anyhow::bail!("Child ID cannot be empty");
    }

    // Ensure name is not empty
    if child.name.is_empty() {
        anyhow::bail!("Child name cannot be empty");
    }

    // Validate warnings are in descending order
    for i in 1..child.warnings.len() {
        if child.warnings[i] >= child.warnings[i - 1] {
            anyhow::bail!(
                "Warning thresholds must be in descending order, got: {:?}",
                child.warnings
            );
        }
    }

    // Validate custom day limits
    for custom in &child.limits.custom {
        if custom.days.is_empty() {
            anyhow::bail!("Custom day limit must specify at least one day");
        }

        for day in &custom.days {
            let day_lower = day.to_lowercase();
            if !matches!(
                day_lower.as_str(),
                "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday"
            ) {
                anyhow::bail!("Invalid day name: {}", day);
            }
        }
    }

    Ok(())
}

/// Example configuration file content
pub const EXAMPLE_CONFIG: &str = include_str!("../../example-time-limits-config.yaml");

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_config() -> TimeLimitsConfig {
        TimeLimitsConfig {
            admin: AdminConfig {
                password_hash: "test_hash".to_string(),
                admin_accounts: vec!["admin".to_string()],
            },
            children: vec![ChildProfile {
                id: "kid1".to_string(),
                name: "Alice".to_string(),
                os_users: vec!["alice".to_string()],
                limits: TimeLimitSchedule {
                    weekday: TimeLimit {
                        hours: 2,
                        minutes: 0,
                    },
                    weekend: TimeLimit {
                        hours: 4,
                        minutes: 0,
                    },
                    custom: vec![],
                },
                warnings: vec![15, 5, 1],
                grace_period: 60,
            }],
            shared_login: SharedLoginConfig {
                enabled: false,
                ..Default::default()
            },
            enforcement: EnforcementConfig::default(),
        }
    }

    #[test]
    fn test_time_limit_conversion() {
        let limit = TimeLimit {
            hours: 2,
            minutes: 30,
        };
        assert_eq!(limit.to_seconds(), 9000);

        let limit2 = TimeLimit::from_seconds(9000);
        assert_eq!(limit2.hours, 2);
        assert_eq!(limit2.minutes, 30);
    }

    #[test]
    fn test_validate_config_requires_children() {
        let mut config = make_test_config();
        config.children.clear();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_rejects_duplicate_ids() {
        let mut config = make_test_config();
        let child2 = config.children[0].clone();
        config.children.push(child2);
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_shared_login_mode() {
        let mut config = make_test_config();
        config.shared_login.enabled = true;
        config.shared_login.shared_accounts = vec!["family".to_string()];
        config.children[0].os_users.clear(); // Must clear os_users in shared mode
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_shared_login_rejects_os_users() {
        let mut config = make_test_config();
        config.shared_login.enabled = true;
        config.shared_login.shared_accounts = vec!["family".to_string()];
        // os_users should be empty in shared login mode
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_warnings_order() {
        let mut config = make_test_config();
        config.children[0].warnings = vec![5, 10, 1]; // Wrong order
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_custom_days() {
        let mut config = make_test_config();
        config.children[0].limits.custom = vec![CustomDayLimit {
            days: vec!["monday".to_string(), "wednesday".to_string()],
            limit: TimeLimit {
                hours: 1,
                minutes: 30,
            },
        }];
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_invalid_day_name() {
        let mut config = make_test_config();
        config.children[0].limits.custom = vec![CustomDayLimit {
            days: vec!["funday".to_string()],
            limit: TimeLimit {
                hours: 1,
                minutes: 30,
            },
        }];
        assert!(validate_config(&config).is_err());
    }
}
