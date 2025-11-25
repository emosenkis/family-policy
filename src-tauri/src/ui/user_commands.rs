use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::core;
use crate::state;
use crate::config;

/// State information for User UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInfo {
    /// Whether any policies are currently applied
    pub policies_applied: bool,
    /// Last time policies were updated
    pub last_updated: Option<String>,
    /// Number of extensions per browser
    pub extensions_count: BrowserCounts,
    /// Number of privacy settings per browser
    pub privacy_settings_count: BrowserCounts,
    /// Hash of current configuration
    pub config_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserCounts {
    pub chrome: usize,
    pub firefox: usize,
    pub edge: usize,
}

/// Configuration summary for User UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    /// Policy names
    pub policy_names: Vec<String>,
    /// Total number of extensions across all browsers
    pub total_extensions: usize,
    /// Browsers that have policies
    pub browsers: Vec<String>,
}

/// Elevation request result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevationResult {
    /// Whether elevation was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Read current state (no admin required)
#[tauri::command]
pub async fn read_state() -> Result<Option<StateInfo>, String> {
    let state = state::load_state()
        .map_err(|e| format!("Failed to load state: {}", e))?;

    let state = match state {
        Some(s) => s,
        None => return Ok(None),
    };

    // Count extensions and privacy settings
    let mut extensions_count = BrowserCounts::default();
    let mut privacy_count = BrowserCounts::default();

    if let Some(ref chrome) = state.applied_policies.chrome {
        extensions_count.chrome = chrome.extensions.len();
        privacy_count.chrome = count_privacy_settings_chrome(chrome);
    }
    if let Some(ref firefox) = state.applied_policies.firefox {
        extensions_count.firefox = firefox.extensions.len();
        privacy_count.firefox = count_privacy_settings_firefox(firefox);
    }
    if let Some(ref edge) = state.applied_policies.edge {
        extensions_count.edge = edge.extensions.len();
        privacy_count.edge = count_privacy_settings_edge(edge);
    }

    let policies_applied = extensions_count.chrome > 0
        || extensions_count.firefox > 0
        || extensions_count.edge > 0
        || privacy_count.chrome > 0
        || privacy_count.firefox > 0
        || privacy_count.edge > 0;

    Ok(Some(StateInfo {
        policies_applied,
        last_updated: Some(state.last_updated.to_rfc3339()),
        extensions_count,
        privacy_settings_count: privacy_count,
        config_hash: state.config_hash,
    }))
}

/// Read configuration summary (no admin required)
#[tauri::command]
pub async fn read_config_summary(config_path: String) -> Result<ConfigSummary, String> {
    let path = std::path::PathBuf::from(config_path);
    let config = config::load_config(&path)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    let policy_names: Vec<String> = config.policies.iter()
        .map(|p| p.name.clone())
        .collect();

    let mut total_extensions = 0;
    let mut browsers_set = std::collections::HashSet::new();

    for policy in &config.policies {
        total_extensions += policy.extensions.len();
        for browser in &policy.browsers {
            browsers_set.insert(format!("{:?}", browser));
        }
    }

    let browsers: Vec<String> = browsers_set.into_iter().collect();

    Ok(ConfigSummary {
        policy_names,
        total_extensions,
        browsers,
    })
}

/// Preview what would happen if config was applied (no admin required)
#[tauri::command]
pub async fn preview_apply(config_path: String) -> Result<core::diff::PolicyDiff, String> {
    let path = std::path::PathBuf::from(config_path);
    let config = config::load_config(&path)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    let current_state = state::load_state()
        .map_err(|e| format!("Failed to load state: {}", e))?;

    let diff = core::diff::generate_diff(&config, current_state.as_ref());

    Ok(diff)
}

/// Check if current process has admin privileges
#[tauri::command]
pub async fn check_admin() -> Result<bool, String> {
    Ok(core::privileges::is_admin())
}

/// Request elevation (platform-specific)
/// Returns true if elevation was successful or already elevated
#[tauri::command]
pub async fn request_elevation() -> Result<ElevationResult, String> {
    if core::privileges::is_admin() {
        return Ok(ElevationResult {
            success: true,
            error: None,
        });
    }

    // On Unix, we can't actually elevate from within the process
    // The user needs to restart with sudo
    #[cfg(unix)]
    {
        Ok(ElevationResult {
            success: false,
            error: Some("Please restart the application with 'sudo' to apply policies.".to_string()),
        })
    }

    // On Windows, we could potentially re-launch with elevation
    // For now, just return an error message
    #[cfg(windows)]
    {
        Ok(ElevationResult {
            success: false,
            error: Some("Please restart the application as Administrator to apply policies.".to_string()),
        })
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(ElevationResult {
            success: false,
            error: Some("Platform not supported for elevation".to_string()),
        })
    }
}

// Helper functions

fn count_privacy_settings_chrome(state: &state::BrowserState) -> usize {
    let mut count = 0;
    if state.disable_incognito.is_some() {
        count += 1;
    }
    if state.disable_guest_mode.is_some() {
        count += 1;
    }
    count
}

fn count_privacy_settings_firefox(state: &state::BrowserState) -> usize {
    let mut count = 0;
    if state.disable_private_browsing.is_some() {
        count += 1;
    }
    count
}

fn count_privacy_settings_edge(state: &state::BrowserState) -> usize {
    let mut count = 0;
    if state.disable_inprivate.is_some() {
        count += 1;
    }
    if state.disable_guest_mode.is_some() {
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BrowserState;

    #[test]
    fn test_count_privacy_settings_chrome() {
        let state = BrowserState {
            extensions: vec![],
            disable_incognito: Some(true),
            disable_inprivate: None,
            disable_private_browsing: None,
            disable_guest_mode: Some(false),
        };
        assert_eq!(count_privacy_settings_chrome(&state), 2);
    }

    #[test]
    fn test_count_privacy_settings_firefox() {
        let state = BrowserState {
            extensions: vec![],
            disable_incognito: None,
            disable_inprivate: None,
            disable_private_browsing: Some(true),
            disable_guest_mode: None,
        };
        assert_eq!(count_privacy_settings_firefox(&state), 1);
    }

    #[test]
    fn test_count_privacy_settings_edge() {
        let state = BrowserState {
            extensions: vec![],
            disable_incognito: None,
            disable_inprivate: Some(true),
            disable_private_browsing: None,
            disable_guest_mode: Some(true),
        };
        assert_eq!(count_privacy_settings_edge(&state), 2);
    }

    #[test]
    fn test_browser_counts_default() {
        let counts = BrowserCounts::default();
        assert_eq!(counts.chrome, 0);
        assert_eq!(counts.firefox, 0);
        assert_eq!(counts.edge, 0);
    }
}
