use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Agent configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    pub github: GitHubConfig,
    pub agent: AgentSettings,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

/// GitHub repository settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubConfig {
    /// Raw file URL to poll
    pub policy_url: String,

    /// For private repositories (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

/// Agent settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSettings {
    /// How often to check for changes (seconds)
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,

    /// Add random jitter to prevent thundering herd (seconds)
    #[serde(default = "default_jitter")]
    pub poll_jitter: u64,

    /// Retry on failure
    #[serde(default = "default_retry_interval")]
    pub retry_interval: u64,

    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub require_signature: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trusted_key: Option<String>,
}

// Default values
fn default_poll_interval() -> u64 {
    300 // 5 minutes
}

fn default_jitter() -> u64 {
    60 // ±1 minute
}

fn default_retry_interval() -> u64 {
    60 // 1 minute
}

fn default_max_retries() -> u32 {
    3
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            poll_interval: default_poll_interval(),
            poll_jitter: default_jitter(),
            retry_interval: default_retry_interval(),
            max_retries: default_max_retries(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            github: GitHubConfig {
                policy_url: String::new(),
                access_token: None,
            },
            agent: AgentSettings::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// Get the platform-specific agent config file path
pub fn get_agent_config_path() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/etc/family-policy/agent.conf"))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from(
            "/Library/Application Support/family-policy/agent.conf",
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let mut path = PathBuf::from(
            std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string()),
        );
        path.push("family-policy");
        path.push("agent.conf");
        Ok(path)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        anyhow::bail!("Unsupported operating system");
    }
}

/// Load agent configuration from file
pub fn load_config() -> Result<AgentConfig> {
    let path = get_agent_config_path()?;

    // If file doesn't exist, return default config
    if !path.exists() {
        return Ok(AgentConfig::default());
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: AgentConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}

/// Save agent configuration to file
pub fn save_config(config: &AgentConfig) -> Result<()> {
    let path = get_agent_config_path()?;

    // Create parent directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Serialize to TOML
    let toml_content =
        toml::to_string_pretty(config).context("Failed to serialize config")?;

    // Write to file
    fs::write(&path, toml_content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

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
        use std::ptr;
        unsafe {
            use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
            use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
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
