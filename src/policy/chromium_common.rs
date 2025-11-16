/// Common functionality for Chromium-based browsers (Chrome, Edge, Brave, etc.)
///
/// This module extracts shared policy application logic to reduce code duplication
/// between Chrome and Edge, which both use the same underlying policy mechanisms.

use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;

use crate::config::Extension;
use crate::state::BrowserState;

/// Configuration for a specific Chromium-based browser
#[derive(Debug, Clone)]
pub struct ChromiumBrowserConfig {
    /// Human-readable browser name (for logging)
    pub browser_name: &'static str,
    /// Windows registry key path
    pub registry_key: &'static str,
    /// macOS bundle identifier
    pub bundle_id: &'static str,
    /// Linux policy directory function
    pub policy_dir_fn: fn() -> &'static Path,
}

/// Generic configuration for Chromium-based browsers
#[derive(Debug, Clone)]
pub struct ChromiumConfig {
    pub extensions: Vec<Extension>,
    pub disable_private_mode: Option<bool>,  // Incognito/InPrivate
    pub disable_guest_mode: Option<bool>,
}

impl ChromiumConfig {
    /// Create from Chrome-specific config
    pub fn from_chrome(config: &crate::config::ChromeConfig) -> Self {
        Self {
            extensions: config.extensions.clone(),
            disable_private_mode: config.disable_incognito,
            disable_guest_mode: config.disable_guest_mode,
        }
    }

    /// Create from Edge-specific config
    pub fn from_edge(config: &crate::config::EdgeConfig) -> Self {
        Self {
            extensions: config.extensions.clone(),
            disable_private_mode: config.disable_inprivate,
            disable_guest_mode: config.disable_guest_mode,
        }
    }
}

/// Apply Chromium-based browser policies (cross-platform)
pub fn apply_chromium_policies(
    config: &ChromiumConfig,
    browser_config: &ChromiumBrowserConfig,
    dry_run: bool,
) -> Result<BrowserState> {
    let platform = crate::browser::current_platform();

    // Apply platform-specific policies
    match platform {
        crate::browser::Platform::Windows => {
            apply_chromium_windows(config, browser_config, dry_run)?
        }
        crate::browser::Platform::MacOS => {
            apply_chromium_macos(config, browser_config, dry_run)?
        }
        crate::browser::Platform::Linux => {
            apply_chromium_linux(config, browser_config, dry_run)?
        }
    }

    // Build and return state (identical for all Chromium browsers)
    let mut state = BrowserState::new();
    state.extensions = config.extensions.iter().map(|e| e.id.clone()).collect();
    state.disable_incognito = config.disable_private_mode;
    state.disable_inprivate = config.disable_private_mode;
    state.disable_guest_mode = config.disable_guest_mode;

    Ok(state)
}

/// Remove Chromium browser policies (cross-platform)
pub fn remove_chromium_policies(browser_config: &ChromiumBrowserConfig) -> Result<()> {
    let platform = crate::browser::current_platform();

    match platform {
        crate::browser::Platform::Windows => {
            remove_chromium_windows(browser_config)?
        }
        crate::browser::Platform::MacOS => {
            remove_chromium_macos(browser_config)?
        }
        crate::browser::Platform::Linux => {
            remove_chromium_linux(browser_config)?
        }
    }

    Ok(())
}

/// Format a Chromium extension entry for policies
/// Format: "{extension_id};{update_url}"
pub fn format_chromium_extension_entry(ext: &Extension) -> String {
    let update_url = ext
        .update_url
        .as_deref()
        .unwrap_or(crate::config::DEFAULT_CHROME_UPDATE_URL);

    format!("{};{}", ext.id, update_url)
}

// ============================================================================
// Platform-Specific Implementations
// ============================================================================

/// Apply Chromium policies on Windows (via Registry)
#[cfg(target_os = "windows")]
fn apply_chromium_windows(
    config: &ChromiumConfig,
    browser_config: &ChromiumBrowserConfig,
    dry_run: bool,
) -> Result<()> {
    use crate::platform::windows::{apply_registry_policy_with_preview, apply_registry_value_with_preview, RegistryValue};

    tracing::debug!(
        "Applying {} policies on Windows (registry key: {})",
        browser_config.browser_name,
        browser_config.registry_key
    );

    // Apply extension policies
    if !config.extensions.is_empty() {
        let extension_key = format!("{}\\ExtensionInstallForcelist", browser_config.registry_key);
        let extension_strings: Vec<String> = config
            .extensions
            .iter()
            .map(format_chromium_extension_entry)
            .collect();

        apply_registry_policy_with_preview(&extension_key, extension_strings, dry_run)
            .with_context(|| {
                format!(
                    "Failed to apply {} extension policy to registry at {}",
                    browser_config.browser_name, extension_key
                )
            })?;
    }

    // Apply privacy controls - Incognito/InPrivate mode
    if let Some(disable_private_mode) = config.disable_private_mode {
        if disable_private_mode {
            let key_name = if browser_config.browser_name == "Chrome" {
                "IncognitoModeAvailability"
            } else {
                "InPrivateModeAvailability"
            };

            apply_registry_value_with_preview(
                browser_config.registry_key,
                key_name,
                RegistryValue::Dword(1), // 1 = Disabled
                dry_run,
            )
            .with_context(|| {
                format!(
                    "Failed to apply {} to registry",
                    key_name
                )
            })?;
        }
    }

    // Apply guest mode control
    if let Some(disable_guest_mode) = config.disable_guest_mode {
        apply_registry_value_with_preview(
            browser_config.registry_key,
            "BrowserGuestModeEnabled",
            RegistryValue::Dword(if disable_guest_mode { 0 } else { 1 }),
            dry_run,
        )
        .with_context(|| {
            format!(
                "Failed to apply BrowserGuestModeEnabled to {} registry",
                browser_config.browser_name
            )
        })?;
    }

    Ok(())
}

/// Apply Chromium policies on macOS (via plist)
#[cfg(target_os = "macos")]
fn apply_chromium_macos(
    config: &ChromiumConfig,
    browser_config: &ChromiumBrowserConfig,
    dry_run: bool,
) -> Result<()> {
    use crate::platform::macos::{
        apply_plist_policy_with_preview, bool_to_plist, integer_to_plist, string_vec_to_plist_array,
    };
    use std::collections::HashMap;

    tracing::debug!(
        "Applying {} policies on macOS (bundle: {})",
        browser_config.browser_name,
        browser_config.bundle_id
    );

    let mut updates = HashMap::new();

    // Apply extension policies
    if !config.extensions.is_empty() {
        let extension_strings: Vec<String> = config
            .extensions
            .iter()
            .map(format_chromium_extension_entry)
            .collect();

        updates.insert(
            "ExtensionInstallForcelist".to_string(),
            string_vec_to_plist_array(extension_strings),
        );
    }

    // Apply privacy controls
    if let Some(disable_private_mode) = config.disable_private_mode {
        if disable_private_mode {
            let key_name = if browser_config.browser_name == "Chrome" {
                "IncognitoModeAvailability"
            } else {
                "InPrivateModeAvailability"
            };

            updates.insert(
                key_name.to_string(),
                integer_to_plist(1), // 1 = Disabled
            );
        }
    }

    // Apply guest mode control
    if let Some(disable_guest_mode) = config.disable_guest_mode {
        updates.insert(
            "BrowserGuestModeEnabled".to_string(),
            bool_to_plist(!disable_guest_mode),
        );
    }

    apply_plist_policy_with_preview(browser_config.bundle_id, updates, dry_run)
        .with_context(|| {
            format!(
                "Failed to apply {} plist policy",
                browser_config.browser_name
            )
        })?;

    Ok(())
}

/// Apply Chromium policies on Linux (via JSON)
#[cfg(target_os = "linux")]
fn apply_chromium_linux(
    config: &ChromiumConfig,
    browser_config: &ChromiumBrowserConfig,
    dry_run: bool,
) -> Result<()> {
    use crate::platform::common::apply_json_file_with_preview;

    let policy_dir = (browser_config.policy_dir_fn)();
    let policy_file = policy_dir.join("browser-policy.json");

    tracing::debug!(
        "Applying {} policies on Linux (dir: {})",
        browser_config.browser_name,
        policy_dir.display()
    );

    let mut policy = json!({});

    // Apply extension policies
    if !config.extensions.is_empty() {
        let extension_strings: Vec<String> = config
            .extensions
            .iter()
            .map(format_chromium_extension_entry)
            .collect();

        policy["ExtensionInstallForcelist"] = json!(extension_strings);
    }

    // Apply privacy controls
    if let Some(disable_private_mode) = config.disable_private_mode {
        if disable_private_mode {
            let key_name = if browser_config.browser_name == "Chrome" {
                "IncognitoModeAvailability"
            } else {
                "InPrivateModeAvailability"
            };

            policy[key_name] = json!(1); // 1 = Disabled
        }
    }

    // Apply guest mode control
    if let Some(disable_guest_mode) = config.disable_guest_mode {
        policy["BrowserGuestModeEnabled"] = json!(!disable_guest_mode);
    }

    apply_json_file_with_preview(&policy_file, policy, dry_run)
        .with_context(|| {
            format!(
                "Failed to apply {} JSON policy",
                browser_config.browser_name
            )
        })?;

    Ok(())
}

/// Remove Chromium policies on Windows
#[cfg(target_os = "windows")]
fn remove_chromium_windows(browser_config: &ChromiumBrowserConfig) -> Result<()> {
    use crate::platform::windows::{remove_registry_policy, remove_registry_value};

    tracing::debug!(
        "Removing {} policies on Windows",
        browser_config.browser_name
    );

    // Remove extension policy
    let extension_key = format!("{}\\ExtensionInstallForcelist", browser_config.registry_key);
    if let Err(e) = remove_registry_policy(&extension_key) {
        tracing::warn!(
            "Failed to remove {} extension policy at {}: {}",
            browser_config.browser_name,
            extension_key,
            e
        );
    }

    // Remove privacy controls
    let privacy_key = if browser_config.browser_name == "Chrome" {
        "IncognitoModeAvailability"
    } else {
        "InPrivateModeAvailability"
    };

    if let Err(e) = remove_registry_value(browser_config.registry_key, privacy_key) {
        tracing::warn!(
            "Failed to remove {} {}: {}",
            browser_config.browser_name,
            privacy_key,
            e
        );
    }

    if let Err(e) = remove_registry_value(browser_config.registry_key, "BrowserGuestModeEnabled") {
        tracing::warn!(
            "Failed to remove {} BrowserGuestModeEnabled: {}",
            browser_config.browser_name,
            e
        );
    }

    Ok(())
}

/// Remove Chromium policies on macOS
#[cfg(target_os = "macos")]
fn remove_chromium_macos(browser_config: &ChromiumBrowserConfig) -> Result<()> {
    use crate::platform::macos::remove_plist_keys;

    tracing::debug!(
        "Removing {} policies on macOS",
        browser_config.browser_name
    );

    let privacy_key = if browser_config.browser_name == "Chrome" {
        "IncognitoModeAvailability"
    } else {
        "InPrivateModeAvailability"
    };

    let keys = vec![
        "ExtensionInstallForcelist".to_string(),
        privacy_key.to_string(),
        "BrowserGuestModeEnabled".to_string(),
    ];

    remove_plist_keys(browser_config.bundle_id, &keys)
        .with_context(|| {
            format!(
                "Failed to remove {} plist keys",
                browser_config.browser_name
            )
        })?;

    Ok(())
}

/// Remove Chromium policies on Linux
#[cfg(target_os = "linux")]
fn remove_chromium_linux(browser_config: &ChromiumBrowserConfig) -> Result<()> {
    use crate::platform::linux::remove_json_policy;

    tracing::debug!(
        "Removing {} policies on Linux",
        browser_config.browser_name
    );

    let policy_dir = (browser_config.policy_dir_fn)();

    remove_json_policy(policy_dir, "browser-policy")
        .with_context(|| {
            format!(
                "Failed to remove {} JSON policy",
                browser_config.browser_name
            )
        })?;

    Ok(())
}

// Stub implementations for platforms not compiled
#[cfg(not(target_os = "windows"))]
fn apply_chromium_windows(
    _config: &ChromiumConfig,
    _browser_config: &ChromiumBrowserConfig,
    _dry_run: bool,
) -> Result<()> {
    anyhow::bail!("Windows platform not supported in this build")
}

#[cfg(not(target_os = "macos"))]
fn apply_chromium_macos(
    _config: &ChromiumConfig,
    _browser_config: &ChromiumBrowserConfig,
    _dry_run: bool,
) -> Result<()> {
    anyhow::bail!("macOS platform not supported in this build")
}

#[cfg(not(target_os = "linux"))]
fn apply_chromium_linux(
    _config: &ChromiumConfig,
    _browser_config: &ChromiumBrowserConfig,
    _dry_run: bool,
) -> Result<()> {
    anyhow::bail!("Linux platform not supported in this build")
}

#[cfg(not(target_os = "windows"))]
fn remove_chromium_windows(_browser_config: &ChromiumBrowserConfig) -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn remove_chromium_macos(_browser_config: &ChromiumBrowserConfig) -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn remove_chromium_linux(_browser_config: &ChromiumBrowserConfig) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_test_extension(id: &str) -> Extension {
        Extension {
            id: id.to_string(),
            name: "Test Extension".to_string(),
            update_url: None,
            install_url: None,
            settings: HashMap::new(),
        }
    }

    #[test]
    fn test_format_chromium_extension_entry() {
        let ext = make_test_extension("abcdefghijklmnopqrstuvwxyzabcdef");
        let entry = format_chromium_extension_entry(&ext);

        assert_eq!(
            entry,
            "abcdefghijklmnopqrstuvwxyzabcdef;https://clients2.google.com/service/update2/crx"
        );
    }

    #[test]
    fn test_format_chromium_extension_entry_with_custom_url() {
        let mut ext = make_test_extension("abcdefghijklmnopqrstuvwxyzabcdef");
        ext.update_url = Some("https://example.com/updates".to_string());
        let entry = format_chromium_extension_entry(&ext);

        assert_eq!(
            entry,
            "abcdefghijklmnopqrstuvwxyzabcdef;https://example.com/updates"
        );
    }

    #[test]
    fn test_chromium_config_from_chrome() {
        let chrome_config = crate::config::ChromeConfig {
            extensions: vec![make_test_extension("test123")],
            disable_incognito: Some(true),
            disable_guest_mode: Some(false),
        };

        let chromium_config = ChromiumConfig::from_chrome(&chrome_config);

        assert_eq!(chromium_config.extensions.len(), 1);
        assert_eq!(chromium_config.disable_private_mode, Some(true));
        assert_eq!(chromium_config.disable_guest_mode, Some(false));
    }

    #[test]
    fn test_chromium_config_from_edge() {
        let edge_config = crate::config::EdgeConfig {
            extensions: vec![make_test_extension("test456")],
            disable_inprivate: Some(true),
            disable_guest_mode: Some(true),
        };

        let chromium_config = ChromiumConfig::from_edge(&edge_config);

        assert_eq!(chromium_config.extensions.len(), 1);
        assert_eq!(chromium_config.disable_private_mode, Some(true));
        assert_eq!(chromium_config.disable_guest_mode, Some(true));
    }
}
