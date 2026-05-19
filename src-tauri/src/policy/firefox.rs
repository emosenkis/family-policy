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
    #[cfg(not(target_os = "macos"))]
    let policy_path = get_firefox_policy_path()?;

    #[cfg(target_os = "macos")]
    let policy_target = "macOS managed preferences configuration profile".to_string();
    #[cfg(not(target_os = "macos"))]
    let policy_target = policy_path.display().to_string();

    tracing::debug!(
        "Preparing Firefox policy application: platform={}, dry_run={}, policy_target={}, extensions={}, disable_private_browsing={:?}",
        crate::browser::current_platform().name(),
        dry_run,
        policy_target,
        config.extensions.len(),
        config.disable_private_browsing
    );

    for ext in &config.extensions {
        tracing::debug!(
            "Firefox extension policy entry: name={:?}, id={}, install_url={:?}, settings_keys={:?}",
            ext.name,
            ext.id,
            ext.install_url,
            ext.settings.keys().collect::<Vec<_>>()
        );

        if !ext.settings.is_empty() {
            tracing::debug!(
                "Firefox extension settings are present for {} ({}) but are not written because Firefox policies currently only force-install extensions from install_url",
                ext.name,
                ext.id
            );
        }
    }

    // Create policies.json content
    let policies_json = create_firefox_policies_json(config)?;
    tracing::debug!(
        "Final Firefox policies for {}: {}",
        policy_target,
        serde_json::to_string_pretty(&policies_json).unwrap_or_else(|_| policies_json.to_string())
    );

    #[cfg(target_os = "macos")]
    {
        tracing::debug!(
            "Skipping Firefox distribution policies.json on macOS; applying policies via managed preferences profile instead"
        );
        let policies_plist = create_firefox_policies_plist(config)?;
        crate::platform::macos::install_managed_preferences_profile(
            FIREFOX_MACOS_BUNDLE_ID,
            "Firefox",
            policies_plist,
            dry_run,
        )
        .context("Failed to install Firefox macOS configuration profile")?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Use common JSON file helper
        crate::platform::common::apply_json_file_with_preview(&policy_path, policies_json, dry_run)
            .with_context(|| {
                format!(
                    "Failed to apply Firefox policies: {}",
                    policy_path.display()
                )
            })?;
    }

    tracing::debug!("Finished writing Firefox policies; building applied state");

    // Build and return state
    let mut state = BrowserState::new();
    state.extensions = config.extensions.iter().map(|e| e.id.clone()).collect();
    state.disable_private_browsing = config.disable_private_browsing;

    Ok(state)
}

/// Remove all Firefox policies
pub fn remove_firefox_policies() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        return remove_firefox_macos_policy_profile();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let policy_path = get_firefox_policy_path()?;

        tracing::debug!(
            "Preparing Firefox policy removal: platform={}, policy_file={}",
            crate::browser::current_platform().name(),
            policy_path.display()
        );

        if policy_path.exists() {
            tracing::debug!("Removing Firefox policy file: {}", policy_path.display());
            std::fs::remove_file(&policy_path).with_context(|| {
                format!(
                    "Failed to remove Firefox policies: {}",
                    policy_path.display()
                )
            })?;

            // Try to remove the distribution directory if it's empty
            if let Some(parent) = policy_path.parent() {
                if let Ok(mut entries) = std::fs::read_dir(parent) {
                    if entries.next().is_none() {
                        tracing::debug!(
                            "Removing empty Firefox policy directory: {}",
                            parent.display()
                        );
                        let _ = std::fs::remove_dir(parent);
                    }
                }
            }
        } else {
            tracing::debug!(
                "Firefox policy file does not exist; nothing to remove: {}",
                policy_path.display()
            );
        }

        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn remove_firefox_macos_policy_profile() -> Result<()> {
    use crate::platform::macos::{
        remove_library_preferences_plist_keys, remove_managed_preferences_profile,
    };

    let keys = vec![
        "EnterprisePoliciesEnabled".to_string(),
        "ExtensionSettings".to_string(),
        "DisablePrivateBrowsing".to_string(),
    ];

    remove_library_preferences_plist_keys(FIREFOX_MACOS_BUNDLE_ID, &keys)
        .context("Failed to remove Firefox macOS policy defaults plist keys")?;

    if let Err(e) = remove_managed_preferences_profile(FIREFOX_MACOS_BUNDLE_ID) {
        tracing::warn!(
            "Failed to remove Firefox macOS configuration profile: {}",
            e
        );
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn remove_firefox_macos_policy_profile() -> Result<()> {
    Ok(())
}

/// Get platform-specific Firefox policy path
fn get_firefox_policy_path() -> Result<PathBuf> {
    tracing::debug!(
        "Resolving Firefox policy path for platform={}",
        crate::browser::current_platform().name()
    );

    #[cfg(target_os = "windows")]
    {
        // Windows: C:\Program Files\Mozilla Firefox\distribution\policies.json
        let paths = vec![
            PathBuf::from(r"C:\Program Files\Mozilla Firefox\distribution\policies.json"),
            PathBuf::from(r"C:\Program Files (x86)\Mozilla Firefox\distribution\policies.json"),
        ];

        // Use the first existing Firefox installation
        for path in paths {
            tracing::debug!("Checking Firefox policy candidate path: {}", path.display());
            if let Some(parent) = path.parent() {
                if let Some(grandparent) = parent.parent() {
                    if grandparent.exists() {
                        tracing::debug!(
                            "Selected Firefox policy path based on existing install: {}",
                            path.display()
                        );
                        return Ok(path);
                    }
                }
            }
        }

        // Default to first path if none exist yet
        let default_path =
            PathBuf::from(r"C:\Program Files\Mozilla Firefox\distribution\policies.json");
        tracing::debug!(
            "No Firefox install candidate found; defaulting policy path to {}",
            default_path.display()
        );
        Ok(default_path)
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: /Applications/Firefox.app/Contents/Resources/distribution/policies.json
        let path = PathBuf::from(
            "/Applications/Firefox.app/Contents/Resources/distribution/policies.json",
        );
        tracing::debug!("Selected Firefox policy path: {}", path.display());
        Ok(path)
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: /etc/firefox/policies/policies.json (system-wide)
        let path = PathBuf::from("/etc/firefox/policies/policies.json");
        tracing::debug!("Selected Firefox policy path: {}", path.display());
        Ok(path)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("Unsupported platform for Firefox policies");
    }
}

/// Create Firefox policies.json structure
fn create_firefox_policies_json(config: &FirefoxConfig) -> Result<serde_json::Value> {
    tracing::debug!(
        "Generating Firefox policies.json: extensions={}, disable_private_browsing={:?}",
        config.extensions.len(),
        config.disable_private_browsing
    );

    let mut policies = json!({});

    // Add extension settings
    if !config.extensions.is_empty() {
        let mut extension_settings = json!({});

        for ext in &config.extensions {
            let install_url = ext.install_url.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Firefox extension '{}' must have install_url", ext.name)
            })?;

            tracing::debug!(
                "Firefox ExtensionSettings entry: id={}, installation_mode=force_installed, install_url={}",
                ext.id,
                install_url
            );

            extension_settings[&ext.id] = json!({
                "installation_mode": "force_installed",
                "install_url": install_url,
            });
        }

        tracing::debug!(
            "Firefox ExtensionSettings generated for {} extensions",
            config.extensions.len()
        );
        policies["ExtensionSettings"] = extension_settings;
    }

    // Add privacy controls
    if let Some(disable_private_browsing) = config.disable_private_browsing {
        tracing::debug!(
            "Firefox policy value: DisablePrivateBrowsing={}",
            disable_private_browsing
        );
        if disable_private_browsing {
            policies["DisablePrivateBrowsing"] = json!(true);
        } else {
            tracing::debug!(
                "Firefox DisablePrivateBrowsing=false is represented by omitting the policy from policies.json"
            );
        }
    }

    // Wrap in policies object
    let wrapped = json!({
        "policies": policies
    });
    tracing::debug!(
        "Generated Firefox policies object: {}",
        serde_json::to_string_pretty(&wrapped).unwrap_or_else(|_| wrapped.to_string())
    );

    Ok(wrapped)
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
