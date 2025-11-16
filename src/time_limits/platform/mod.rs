/// Platform-specific computer locking implementations

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

use anyhow::Result;
use crate::time_limits::config::LockAction;

/// Lock the computer using the platform-specific mechanism
pub fn lock_computer(action: LockAction) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::lock_computer(action)
    }

    #[cfg(target_os = "macos")]
    {
        macos::lock_computer(action)
    }

    #[cfg(target_os = "linux")]
    {
        linux::lock_computer(action)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("Unsupported operating system for computer locking")
    }
}

/// Check if the platform supports a specific lock action
pub fn supports_action(action: LockAction) -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::supports_action(action)
    }

    #[cfg(target_os = "macos")]
    {
        macos::supports_action(action)
    }

    #[cfg(target_os = "linux")]
    {
        linux::supports_action(action)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        false
    }
}
