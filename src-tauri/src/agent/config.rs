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
    /// Create at: https://github.com/settings/tokens
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

/// Security configuration (for advanced users)
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SecurityConfig {
    /// Verify GPG signature on policy file (optional, for paranoid users)
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

impl AgentConfig {
    /// Load configuration from file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: AgentConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // Validate config
        config.validate()?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Serialize to TOML
        let toml = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        // Write to file
        fs::write(path, toml)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        // Set restrictive permissions
        crate::platform::common::set_file_permissions(path, 0o600)?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate policy URL
        let url = url::Url::parse(&self.github.policy_url)
            .context("Invalid policy URL")?;

        // Ensure HTTPS only
        if url.scheme() != "https" {
            anyhow::bail!("Policy URL must use HTTPS (got: {})", url.scheme());
        }

        // Validate it's a raw GitHub URL
        if !url.host_str().map(|h| h.contains("github")).unwrap_or(false) {
            eprintln!("Warning: Policy URL doesn't appear to be a GitHub URL");
            eprintln!("  Expected: https://raw.githubusercontent.com/...");
            eprintln!("  Got: {}", url);
        }

        // Validate poll interval
        if self.agent.poll_interval < 60 {
            anyhow::bail!("Poll interval must be at least 60 seconds (got: {})", self.agent.poll_interval);
        }

        Ok(())
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
        Ok(PathBuf::from("/Library/Application Support/family-policy/agent.conf"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_config_validates_https() {
        let config = AgentConfig {
            github: GitHubConfig {
                policy_url: "http://example.com/policy.yaml".to_string(),
                access_token: None,
            },
            agent: AgentSettings::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn agent_config_accepts_https() {
        let config = AgentConfig {
            github: GitHubConfig {
                policy_url: "https://raw.githubusercontent.com/user/repo/main/policy.yaml".to_string(),
                access_token: None,
            },
            agent: AgentSettings::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn agent_config_validates_poll_interval() {
        let config = AgentConfig {
            github: GitHubConfig {
                policy_url: "https://raw.githubusercontent.com/user/repo/main/policy.yaml".to_string(),
                access_token: None,
            },
            agent: AgentSettings {
                poll_interval: 30, // Too short
                ..Default::default()
            },
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn agent_settings_default_values() {
        let settings = AgentSettings::default();
        assert_eq!(settings.poll_interval, 300);
        assert_eq!(settings.poll_jitter, 60);
        assert_eq!(settings.retry_interval, 60);
        assert_eq!(settings.max_retries, 3);
    }

    #[test]
    fn logging_config_default_values() {
        let logging = LoggingConfig::default();
        assert_eq!(logging.level, "info");
        assert!(logging.file.is_none());
    }
}
