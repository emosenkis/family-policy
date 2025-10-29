use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;

use crate::config::FirefoxConfig;
use crate::state::BrowserState;

/// Apply Firefox policies (extensions and privacy controls)
pub fn apply_firefox_policies(config: &FirefoxConfig) -> Result<BrowserState> {
    let policy_path = get_firefox_policy_path()?;

    // Create policies.json content
    let policies_json = create_firefox_policies_json(config)?;

    // Ensure parent directory exists
    if let Some(parent) = policy_path.parent() {
        crate::platform::common::ensure_directory_exists(parent)?;
    }

    // Write policies.json
    let content = serde_json::to_string_pretty(&policies_json)
        .context("Failed to serialize Firefox policies")?;

    crate::platform::common::atomic_write(&policy_path, content.as_bytes())
        .with_context(|| format!("Failed to write Firefox policies: {}", policy_path.display()))?;

    crate::platform::common::set_permissions_readable_all(&policy_path)?;

    // Build and return state
    let mut state = BrowserState::new();
    state.extensions = config
        .extensions
        .iter()
        .map(|e| e.id.clone())
        .collect();
    state.disable_private_browsing = config.disable_private_browsing;

    Ok(state)
}

/// Remove all Firefox policies
pub fn remove_firefox_policies() -> Result<()> {
    let policy_path = get_firefox_policy_path()?;

    if policy_path.exists() {
        std::fs::remove_file(&policy_path)
            .with_context(|| format!("Failed to remove Firefox policies: {}", policy_path.display()))?;

        // Try to remove the distribution directory if it's empty
        if let Some(parent) = policy_path.parent() {
            if let Ok(mut entries) = std::fs::read_dir(parent) {
                if entries.next().is_none() {
                    let _ = std::fs::remove_dir(parent);
                }
            }
        }
    }

    Ok(())
}

/// Get platform-specific Firefox policy path
fn get_firefox_policy_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Windows: C:\Program Files\Mozilla Firefox\distribution\policies.json
        let paths = vec![
            PathBuf::from(r"C:\Program Files\Mozilla Firefox\distribution\policies.json"),
            PathBuf::from(r"C:\Program Files (x86)\Mozilla Firefox\distribution\policies.json"),
        ];

        // Use the first existing Firefox installation
        for path in paths {
            if let Some(parent) = path.parent() {
                if let Some(grandparent) = parent.parent() {
                    if grandparent.exists() {
                        return Ok(path);
                    }
                }
            }
        }

        // Default to first path if none exist yet
        Ok(PathBuf::from(
            r"C:\Program Files\Mozilla Firefox\distribution\policies.json",
        ))
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: /Applications/Firefox.app/Contents/Resources/distribution/policies.json
        Ok(PathBuf::from(
            "/Applications/Firefox.app/Contents/Resources/distribution/policies.json",
        ))
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: /etc/firefox/policies/policies.json (system-wide)
        Ok(PathBuf::from("/etc/firefox/policies/policies.json"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("Unsupported platform for Firefox policies");
    }
}

/// Create Firefox policies.json structure
fn create_firefox_policies_json(config: &FirefoxConfig) -> Result<serde_json::Value> {
    let mut policies = json!({});

    // Add extension settings
    if !config.extensions.is_empty() {
        let mut extension_settings = json!({});

        for ext in &config.extensions {
            let install_url = ext
                .install_url
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Firefox extension '{}' must have install_url", ext.name))?;

            extension_settings[&ext.id] = json!({
                "installation_mode": "force_installed",
                "install_url": install_url,
            });
        }

        policies["ExtensionSettings"] = extension_settings;
    }

    // Add privacy controls
    if let Some(disable_private_browsing) = config.disable_private_browsing {
        if disable_private_browsing {
            policies["DisablePrivateBrowsing"] = json!(true);
        }
    }

    // Wrap in policies object
    Ok(json!({
        "policies": policies
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Extension;
    use std::collections::HashMap;

    #[test]
    fn test_create_firefox_policies_json() {
        let config = FirefoxConfig {
            extensions: vec![Extension {
                id: "test@example.com".to_string(),
                name: "Test Extension".to_string(),
                update_url: None,
                install_url: Some("https://example.com/extension.xpi".to_string()),
                settings: HashMap::new(),
            }],
            disable_private_browsing: Some(true),
        };

        let policies = create_firefox_policies_json(&config).unwrap();

        assert!(policies["policies"]["ExtensionSettings"]["test@example.com"].is_object());
        assert_eq!(
            policies["policies"]["ExtensionSettings"]["test@example.com"]["installation_mode"],
            "force_installed"
        );
        assert_eq!(policies["policies"]["DisablePrivateBrowsing"], true);
    }

    #[test]
    fn test_create_firefox_policies_json_without_privacy() {
        let config = FirefoxConfig {
            extensions: vec![Extension {
                id: "test@example.com".to_string(),
                name: "Test Extension".to_string(),
                update_url: None,
                install_url: Some("https://example.com/extension.xpi".to_string()),
                settings: HashMap::new(),
            }],
            disable_private_browsing: None,
        };

        let policies = create_firefox_policies_json(&config).unwrap();

        assert!(policies["policies"]["ExtensionSettings"]["test@example.com"].is_object());
        assert!(policies["policies"]["DisablePrivateBrowsing"].is_null());
    }
}
