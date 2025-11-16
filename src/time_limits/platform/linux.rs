use anyhow::Result;
use crate::time_limits::config::LockAction;
use std::process::Command;

/// Lock the computer on Linux
pub fn lock_computer(action: LockAction) -> Result<()> {
    match action {
        LockAction::Lock => lock_screen(),
        LockAction::Logout => logout_user(),
        LockAction::Shutdown => shutdown_computer(),
    }
}

/// Lock the screen using various methods (tries multiple approaches)
fn lock_screen() -> Result<()> {
    // Try multiple methods in order of preference

    // 1. systemd-logind (modern, works across desktop environments)
    if try_command("loginctl", &["lock-session"]).is_ok() {
        return Ok(());
    }

    // 2. XDG screensaver
    if try_command("xdg-screensaver", &["lock"]).is_ok() {
        return Ok(());
    }

    // 3. GNOME screensaver
    if try_command("gnome-screensaver-command", &["--lock"]).is_ok() {
        return Ok(());
    }

    // 4. Cinnamon screensaver
    if try_command("cinnamon-screensaver-command", &["--lock"]).is_ok() {
        return Ok(());
    }

    // 5. MATE screensaver
    if try_command("mate-screensaver-command", &["--lock"]).is_ok() {
        return Ok(());
    }

    // 6. XScreenSaver
    if try_command("xscreensaver-command", &["-lock"]).is_ok() {
        return Ok(());
    }

    // 7. Light-locker
    if try_command("light-locker-command", &["--lock"]).is_ok() {
        return Ok(());
    }

    // 8. i3lock (minimalist)
    if try_command("i3lock", &["-c", "000000"]).is_ok() {
        return Ok(());
    }

    // 9. slock (simple X screen locker)
    if try_command("slock", &[]).is_ok() {
        return Ok(());
    }

    anyhow::bail!("No supported screen lock mechanism found on this Linux system")
}

/// Logout the user
fn logout_user() -> Result<()> {
    // Try systemd-logind first
    if try_command("loginctl", &["terminate-user", ""]).is_ok() {
        return Ok(());
    }

    // Try GNOME
    if try_command("gnome-session-quit", &["--logout", "--no-prompt"]).is_ok() {
        return Ok(());
    }

    // Try KDE
    if try_command("qdbus", &["org.kde.ksmserver", "/KSMServer", "logout", "0", "0", "0"]).is_ok() {
        return Ok(());
    }

    // Try XFCE
    if try_command("xfce4-session-logout", &["--logout"]).is_ok() {
        return Ok(());
    }

    anyhow::bail!("No supported logout mechanism found on this Linux system")
}

/// Shutdown the computer
fn shutdown_computer() -> Result<()> {
    // Try systemd shutdown
    if try_command("systemctl", &["poweroff"]).is_ok() {
        return Ok(());
    }

    // Try traditional shutdown command
    if try_command("shutdown", &["-h", "now"]).is_ok() {
        return Ok(());
    }

    // Try poweroff command
    if try_command("poweroff", &[]).is_ok() {
        return Ok(());
    }

    anyhow::bail!("No supported shutdown mechanism found on this Linux system")
}

/// Try to execute a command, returning Ok if successful
fn try_command(cmd: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(cmd)
        .args(args)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("Command failed: {} {:?}", cmd, args)
    }
}

/// Check if Linux supports a specific lock action
pub fn supports_action(action: LockAction) -> bool {
    // Linux generally supports all actions, but availability depends on installed tools
    // We'll return true for all and let the actual execution fail if tools are missing
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

    #[test]
    fn test_try_command_with_invalid_command() {
        assert!(try_command("nonexistent_command_xyz", &[]).is_err());
    }
}
