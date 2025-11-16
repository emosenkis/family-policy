use anyhow::Result;
use crate::time_limits::config::LockAction;

#[cfg(target_os = "windows")]
use windows::Win32::System::Shutdown::{LockWorkStation, ExitWindowsEx, EWX_LOGOFF, EWX_SHUTDOWN, EWX_FORCE};

/// Lock the computer on Windows
pub fn lock_computer(action: LockAction) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            match action {
                LockAction::Lock => {
                    LockWorkStation()?;
                }
                LockAction::Logout => {
                    ExitWindowsEx(EWX_LOGOFF, 0)?;
                }
                LockAction::Shutdown => {
                    ExitWindowsEx(EWX_SHUTDOWN | EWX_FORCE, 0)?;
                }
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("This function is only available on Windows")
    }
}

/// Check if Windows supports a specific lock action
pub fn supports_action(action: LockAction) -> bool {
    // Windows supports all lock actions
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
