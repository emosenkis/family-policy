use anyhow::Result;
use crate::time_limits::config::LockAction;
use std::process::Command;

/// Lock the computer on macOS
pub fn lock_computer(action: LockAction) -> Result<()> {
    match action {
        LockAction::Lock => {
            // Use osascript to trigger the lock screen
            Command::new("osascript")
                .arg("-e")
                .arg("tell application \"System Events\" to keystroke \"q\" using {command down, control down}")
                .output()?;
            Ok(())
        }
        LockAction::Logout => {
            // Log out the current user
            Command::new("osascript")
                .arg("-e")
                .arg("tell application \"System Events\" to log out")
                .output()?;
            Ok(())
        }
        LockAction::Shutdown => {
            // Shutdown the computer
            Command::new("osascript")
                .arg("-e")
                .arg("tell application \"System Events\" to shut down")
                .output()?;
            Ok(())
        }
    }
}

/// Check if macOS supports a specific lock action
pub fn supports_action(action: LockAction) -> bool {
    // macOS supports all lock actions
    matches!(action, LockAction::Lock | LockAction::Logout | LockAction::Shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_all_actions() {
        assert!(supports_action(LockAction::Lock));
        assert!(supports_action(LockAction::Logout));
        assert!(supports_action(LockAction::Shutdown));
    }
}
