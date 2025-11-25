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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_check_user_level() {
        let check = PrivilegeCheck::user();
        assert!(check_privileges(check.clone(), false).is_ok());
        assert!(check_privileges(check, true).is_ok());
    }

    #[test]
    fn test_privilege_check_admin_with_dry_run() {
        let check = PrivilegeCheck::admin_or_dry_run();

        // Dry-run should always pass
        assert!(check_privileges(check, true).is_ok());

        // Non-dry-run depends on actual privileges
        // We can't test this reliably without actually running as admin
    }

    #[test]
    fn test_privilege_check_admin_only() {
        let check = PrivilegeCheck::admin();

        // Dry-run should still require admin when allow_dry_run is false
        let result = check_privileges(check, true);
        if is_admin() {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_is_admin() {
        // Just verify the function doesn't panic
        let _ = is_admin();
    }
}
