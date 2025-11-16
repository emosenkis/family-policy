use anyhow::Result;
use crate::time_limits::config::{LockAction, TimeLimitsConfig};
use crate::time_limits::state::{TimeLimitsState, ChildState};
use crate::time_limits::platform;
use tracing::{info, warn, error};

/// Lock enforcer handles enforcing time limits
pub struct LockEnforcer {
    config: TimeLimitsConfig,
}

impl LockEnforcer {
    pub fn new(config: TimeLimitsConfig) -> Self {
        Self { config }
    }

    /// Enforce the lock for a child whose time has expired
    pub fn enforce_lock(&self, child: &ChildState) -> Result<()> {
        info!("Enforcing time limit for child: {} ({})", child.name, child.id);

        let action = self.config.enforcement.action;

        // Check if the platform supports this action
        if !platform::supports_action(action) {
            warn!("Platform does not support action {:?}, falling back to Lock", action);
            platform::lock_computer(LockAction::Lock)?;
        } else {
            platform::lock_computer(action)?;
        }

        info!("Successfully enforced {:?} for child: {}", action, child.name);
        Ok(())
    }

    /// Send a warning notification to the user
    pub fn send_warning(&self, child: &ChildState, minutes_remaining: u32) -> Result<()> {
        info!("Sending {} minute warning to child: {}", minutes_remaining, child.name);

        // Send system notification
        self.send_system_notification(
            "Time Limit Warning",
            &format!(
                "{}, you have {} minute{} of computer time remaining. Please save your work.",
                child.name,
                minutes_remaining,
                if minutes_remaining == 1 { "" } else { "s" }
            ),
        )?;

        Ok(())
    }

    /// Send final warning before grace period
    pub fn send_final_warning(&self, child: &ChildState, grace_seconds: u64) -> Result<()> {
        info!("Sending final warning to child: {}", child.name);

        self.send_system_notification(
            "Time Limit Reached",
            &format!(
                "{}, your computer time has run out. The computer will lock in {} seconds.",
                child.name, grace_seconds
            ),
        )?;

        Ok(())
    }

    /// Send a system notification (platform-specific)
    fn send_system_notification(&self, title: &str, message: &str) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            // Use notify-send on Linux
            use std::process::Command;
            Command::new("notify-send")
                .arg(title)
                .arg(message)
                .arg("--urgency=critical")
                .arg("--icon=dialog-warning")
                .output()?;
        }

        #[cfg(target_os = "macos")]
        {
            // Use osascript to display notification on macOS
            use std::process::Command;
            let script = format!(
                "display notification \"{}\" with title \"{}\" sound name \"Glass\"",
                message, title
            );
            Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .output()?;
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, we'd use Windows Toastnotifications
            // For now, we'll log it (full implementation would use windows-rs)
            warn!("Notification: {} - {}", title, message);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_limits::config::{AdminConfig, SharedLoginConfig, EnforcementConfig, ChildProfile, TimeLimitSchedule, TimeLimit};
    use crate::time_limits::state::DayUsage;

    fn make_test_config() -> TimeLimitsConfig {
        TimeLimitsConfig {
            admin: AdminConfig {
                password_hash: "test_hash".to_string(),
                admin_accounts: vec![],
            },
            children: vec![],
            shared_login: SharedLoginConfig::default(),
            enforcement: EnforcementConfig::default(),
        }
    }

    fn make_test_child() -> ChildState {
        ChildState {
            id: "kid1".to_string(),
            name: "Alice".to_string(),
            today: DayUsage {
                date: "2025-11-16".to_string(),
                used_seconds: 7200,
                remaining_seconds: 0,
                sessions: vec![],
                warnings_shown: vec![],
                locked_at: None,
            },
        }
    }

    #[test]
    fn test_enforcer_creation() {
        let config = make_test_config();
        let enforcer = LockEnforcer::new(config);
        assert_eq!(enforcer.config.enforcement.action, LockAction::Lock);
    }

    // Note: We can't test actual locking in unit tests as it would lock the machine
    // These would be integration tests run manually or in a VM
}
