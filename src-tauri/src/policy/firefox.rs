use anyhow::{Context, Result};
#[cfg(any(target_os = "macos", test))]
use plist::Value;
use serde_json::json;
use std::path::PathBuf;

use crate::config::FirefoxConfig;
use crate::state::BrowserState;

#[cfg(target_os = "macos")]
const FIREFOX_MACOS_BUNDLE_ID: &str = "org.mozilla.firefox";

/// Apply Firefox policies (extensions and privacy controls)
pub fn apply_firefox_policies(config: &FirefoxConfig, dry_run: bool) -> Result<BrowserState> {
    apply_firefox_platform_policies(config, dry_run)?;

    Ok(build_firefox_state(config))
}

fn build_firefox_state(config: &FirefoxConfig) -> BrowserState {
    let mut state = BrowserState::new();
    state.extensions = config.extensions.iter().map(|e| e.id.clone()).collect();
    state.disable_private_browsing = config.disable_private_browsing;
    state
}

#[cfg(target_os = "macos")]
fn apply_firefox_platform_policies(config: &FirefoxConfig, dry_run: bool) -> Result<()> {
    let policies_plist = create_firefox_policies_plist(config)?;

    crate::platform::macos::apply_plist_policy_with_preview(
        FIREFOX_MACOS_BUNDLE_ID,
        policies_plist,
        dry_run,
    )
    .context("Failed to apply Firefox plist policy")
}

#[cfg(not(target_os = "macos"))]
fn apply_firefox_platform_policies(config: &FirefoxConfig, dry_run: bool) -> Result<()> {
    let policy_path = get_firefox_policy_path()?;
    let policies_json = create_firefox_policies_json(config)?;

    crate::platform::common::apply_json_file_with_preview(&policy_path, policies_json, dry_run)
        .with_context(|| {
            format!(
                "Failed to apply Firefox policies: {}",
                policy_path.display()
            )
        })
}

/// Remove all Firefox policies
pub fn remove_firefox_policies() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        return crate::platform::macos::remove_plist(FIREFOX_MACOS_BUNDLE_ID)
            .context("Failed to remove Firefox plist policy");
    }

    #[cfg(not(target_os = "macos"))]
    {
        let policy_path = get_firefox_policy_path()?;

        if policy_path.exists() {
            std::fs::remove_file(&policy_path).with_context(|| {
                format!(
                    "Failed to remove Firefox policies: {}",
                    policy_path.display()
                )
            })?;

            // Try to remove the distribution directory if it's empty
            if let Some(parent) = policy_path.parent() {
                let parent_is_empty = std::fs::read_dir(parent)
                    .map(|mut entries| entries.next().is_none())
                    .unwrap_or(false);

                if parent_is_empty {
                    let _ = std::fs::remove_dir(parent);
                }
            }
        }

        Ok(())
    }
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
            let install_url = ext.install_url.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Firefox extension '{}' must have install_url", ext.name)
            })?;

            extension_settings[&ext.id] = json!({
                "installation_mode": "force_installed",
                "install_url": install_url,
            });
        }

        policies["ExtensionSettings"] = extension_settings;
    }

    // Add privacy controls
    if config.disable_private_browsing == Some(true) {
        policies["DisablePrivateBrowsing"] = json!(true);
    }

    // Wrap in policies object
    Ok(json!({
        "policies": policies
    }))
}

/// Create Firefox macOS managed-preferences plist structure.
///
/// Firefox's macOS plist is rooted directly at policy keys, unlike policies.json
/// which wraps them in a top-level `policies` object.
#[cfg(any(target_os = "macos", test))]
fn create_firefox_policies_plist(
    config: &FirefoxConfig,
) -> Result<std::collections::HashMap<String, Value>> {
    let mut policies = std::collections::HashMap::new();
    policies.insert(
        "EnterprisePoliciesEnabled".to_string(),
        Value::Boolean(true),
    );

    if !config.extensions.is_empty() {
        let mut extension_settings = plist::Dictionary::new();

        for ext in &config.extensions {
            let install_url = ext.install_url.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Firefox extension '{}' must have install_url", ext.name)
            })?;

            let mut extension_policy = plist::Dictionary::new();
            extension_policy.insert(
                "installation_mode".to_string(),
                Value::String("force_installed".to_string()),
            );
            extension_policy.insert(
                "install_url".to_string(),
                Value::String(install_url.clone()),
            );

            extension_settings.insert(ext.id.clone(), Value::Dictionary(extension_policy));
        }

        policies.insert(
            "ExtensionSettings".to_string(),
            Value::Dictionary(extension_settings),
        );
    }

    if config.disable_private_browsing == Some(true) {
        policies.insert("DisablePrivateBrowsing".to_string(), Value::Boolean(true));
    }

    Ok(policies)
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

    #[test]
    fn test_create_firefox_policies_plist_matches_macos_shape() {
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

        let policies = create_firefox_policies_plist(&config).unwrap();

        assert_eq!(
            policies.get("EnterprisePoliciesEnabled"),
            Some(&Value::Boolean(true))
        );
        assert_eq!(
            policies.get("DisablePrivateBrowsing"),
            Some(&Value::Boolean(true))
        );
        assert!(!policies.contains_key("policies"));

        let extension_settings = match policies.get("ExtensionSettings") {
            Some(Value::Dictionary(settings)) => settings,
            _ => panic!("Expected ExtensionSettings dictionary"),
        };
        let extension_policy = match extension_settings.get("test@example.com") {
            Some(Value::Dictionary(policy)) => policy,
            _ => panic!("Expected extension policy dictionary"),
        };

        assert_eq!(
            extension_policy.get("installation_mode"),
            Some(&Value::String("force_installed".to_string()))
        );
        assert_eq!(
            extension_policy.get("install_url"),
            Some(&Value::String(
                "https://example.com/extension.xpi".to_string()
            ))
        );
    }
}
