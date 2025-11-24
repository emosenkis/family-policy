use crate::agent::config::{AgentConfig, get_agent_config_path};
use anyhow::Result;
use std::fs;

/// Load agent configuration from file
pub fn load_config() -> Result<AgentConfig> {
    let path = get_agent_config_path()?;

    // If file doesn't exist, return default config
    if !path.exists() {
        return Ok(AgentConfig::default());
    }

    let content = fs::read_to_string(&path)?;
    let config: AgentConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Save agent configuration to file
pub fn save_config(config: &AgentConfig) -> Result<()> {
    let path = get_agent_config_path()?;

    // Create parent directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize to TOML
    let toml_content = toml::to_string_pretty(config)?;

    // Write to file
    fs::write(&path, toml_content)?;

    // Set restrictive permissions on Unix-like systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

/// Check if the current user has admin/root privileges
pub fn is_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        // On Windows, check if running as administrator
        unsafe {
            use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
            use windows_sys::Win32::Security::{
                GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
            };
            use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

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

    #[cfg(unix)]
    {
        // On Unix-like systems, check if running as root (UID 0)
        unsafe { libc::geteuid() == 0 }
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    {
        false
    }
}
