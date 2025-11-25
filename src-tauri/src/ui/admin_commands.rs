use serde::{Deserialize, Serialize};

use crate::core;
use crate::config;

/// Apply policies from configuration file
/// Requires admin privileges (checked by caller)
#[tauri::command]
pub async fn apply_policies(config_path: String) -> Result<core::apply::ApplyResult, String> {
    // Verify admin privileges
    if !core::privileges::is_admin() {
        return Err("This operation requires administrator privileges".to_string());
    }

    let path = std::path::PathBuf::from(config_path);
    let config = config::load_config(&path)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    core::apply::apply_policies_from_config(&config, false)
        .map_err(|e| format!("Failed to apply policies: {}", e))
}

/// Remove all applied policies
/// Requires admin privileges (checked by caller)
#[tauri::command]
pub async fn remove_policies() -> Result<core::apply::RemovalResult, String> {
    // Verify admin privileges
    if !core::privileges::is_admin() {
        return Err("This operation requires administrator privileges".to_string());
    }

    core::apply::remove_all_policies(false)
        .map_err(|e| format!("Failed to remove policies: {}", e))
}

/// Preview policy removal (what would be removed)
#[tauri::command]
pub async fn preview_removal() -> Result<core::apply::RemovalResult, String> {
    // Preview doesn't require admin
    core::apply::remove_all_policies(true)
        .map_err(|e| format!("Failed to preview removal: {}", e))
}

/// Validate a configuration file
#[tauri::command]
pub async fn validate_config(config_path: String) -> Result<ValidationResult, String> {
    let path = std::path::PathBuf::from(config_path);

    match config::load_config(&path) {
        Ok(config) => {
            match config::validate_config(&config) {
                Ok(_) => Ok(ValidationResult {
                    valid: true,
                    errors: vec![],
                    warnings: vec![],
                }),
                Err(e) => Ok(ValidationResult {
                    valid: false,
                    errors: vec![format!("{}", e)],
                    warnings: vec![],
                }),
            }
        }
        Err(e) => Ok(ValidationResult {
            valid: false,
            errors: vec![format!("Failed to load config: {}", e)],
            warnings: vec![],
        }),
    }
}

/// Save configuration to file
/// Requires admin privileges for system-wide configs
#[tauri::command]
pub async fn save_config(config_path: String, config_yaml: String) -> Result<(), String> {
    // Verify admin privileges for system paths
    let path = std::path::PathBuf::from(&config_path);

    // Check if path is in a system directory
    let is_system_path = path.starts_with("/etc")
        || path.starts_with("/Library")
        || path.to_str().map(|s| s.contains("ProgramData")).unwrap_or(false);

    if is_system_path && !core::privileges::is_admin() {
        return Err("Writing to system directories requires administrator privileges".to_string());
    }

    // Validate the YAML first
    let _config: config::Config = serde_yaml::from_str(&config_yaml)
        .map_err(|e| format!("Invalid YAML: {}", e))?;

    // Write to file
    std::fs::write(&path, config_yaml)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

/// Get default configuration as YAML string
#[tauri::command]
pub async fn get_default_config() -> Result<String, String> {
    // Return an example configuration
    Ok(r#"# Family Policy Configuration Example
#
# This file configures browser extension policies and privacy controls
# for Chrome, Firefox, and Edge across Windows, macOS, and Linux.

policies:
  # Privacy controls that apply across browsers
  - name: Private browsing restrictions
    browsers:
      - chrome
      - firefox
      - edge
    disable_private_mode: true  # Disables incognito/private browsing/InPrivate
    disable_guest_mode: true    # Disables guest mode (Chrome and Edge only)

  # Extension policy with browser-specific IDs
  - name: uBlock Origin Lite
    browsers:
      - chrome
      - firefox
      - edge
    extensions:
      - name: uBlock Origin Lite
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
          firefox: uBOLite@raymondhill.net
          edge: ddkjiahejlhfcafbddmgiahcphecmpfh
        force_installed: true
"#.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_creation() {
        let result = ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec!["Test warning".to_string()],
        };
        assert!(result.valid);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_validation_result_serialization() {
        let result = ValidationResult {
            valid: false,
            errors: vec!["Error 1".to_string(), "Error 2".to_string()],
            warnings: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ValidationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.valid, result.valid);
        assert_eq!(deserialized.errors.len(), 2);
    }
}
