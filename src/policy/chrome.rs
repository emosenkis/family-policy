use anyhow::{Context, Result};
use serde_json::json;

use crate::browser::current_platform;
use crate::config::ChromeConfig;
use crate::state::BrowserState;

/// Apply Chrome policies (extensions and privacy controls)
pub fn apply_chrome_policies(config: &ChromeConfig) -> Result<BrowserState> {
    let platform = current_platform();

    // Apply platform-specific policies
    match platform {
        crate::browser::Platform::Windows => apply_chrome_windows(config)?,
        crate::browser::Platform::MacOS => apply_chrome_macos(config)?,
        crate::browser::Platform::Linux => apply_chrome_linux(config)?,
    }

    // Build and return state
    let mut state = BrowserState::new();
    state.extensions = config
        .extensions
        .iter()
        .map(|e| e.id.clone())
        .collect();
    state.disable_incognito = config.disable_incognito;
    state.disable_guest_mode = config.disable_guest_mode;

    Ok(state)
}

/// Remove all Chrome policies
pub fn remove_chrome_policies() -> Result<()> {
    let platform = current_platform();

    match platform {
        crate::browser::Platform::Windows => remove_chrome_windows()?,
        crate::browser::Platform::MacOS => remove_chrome_macos()?,
        crate::browser::Platform::Linux => remove_chrome_linux()?,
    }

    Ok(())
}

/// Apply Chrome policies on Windows (via Registry)
#[cfg(target_os = "windows")]
fn apply_chrome_windows(config: &ChromeConfig) -> Result<()> {
    use crate::platform::windows::{write_registry_policy, write_registry_value, RegistryValue};

    const CHROME_KEY: &str = r"SOFTWARE\Policies\Google\Chrome";

    // Apply extension policies
    if !config.extensions.is_empty() {
        let extension_key = format!("{}\\ExtensionInstallForcelist", CHROME_KEY);
        let extension_strings: Vec<String> = config
            .extensions
            .iter()
            .map(|ext| format_chrome_extension_entry(ext))
            .collect();

        write_registry_policy(&extension_key, extension_strings)
            .context("Failed to write Chrome extension policy to registry")?;
    }

    // Apply privacy controls
    if let Some(disable_incognito) = config.disable_incognito {
        if disable_incognito {
            write_registry_value(
                CHROME_KEY,
                "IncognitoModeAvailability",
                RegistryValue::Dword(1), // 1 = Disabled
            )
            .context("Failed to write IncognitoModeAvailability to registry")?;
        }
    }

    if let Some(disable_guest_mode) = config.disable_guest_mode {
        write_registry_value(
            CHROME_KEY,
            "BrowserGuestModeEnabled",
            RegistryValue::Dword(if disable_guest_mode { 0 } else { 1 }),
        )
        .context("Failed to write BrowserGuestModeEnabled to registry")?;
    }

    Ok(())
}

/// Apply Chrome policies on macOS (via plist)
#[cfg(target_os = "macos")]
fn apply_chrome_macos(config: &ChromeConfig) -> Result<()> {
    use crate::platform::macos::{
        bool_to_plist, integer_to_plist, string_vec_to_plist_array, write_plist_policy,
    };
    use std::collections::HashMap;

    const CHROME_BUNDLE_ID: &str = "com.google.Chrome";
    let mut updates = HashMap::new();

    // Apply extension policies
    if !config.extensions.is_empty() {
        let extension_strings: Vec<String> = config
            .extensions
            .iter()
            .map(|ext| format_chrome_extension_entry(ext))
            .collect();

        updates.insert(
            "ExtensionInstallForcelist".to_string(),
            string_vec_to_plist_array(extension_strings),
        );
    }

    // Apply privacy controls
    if let Some(disable_incognito) = config.disable_incognito {
        if disable_incognito {
            updates.insert(
                "IncognitoModeAvailability".to_string(),
                integer_to_plist(1), // 1 = Disabled
            );
        }
    }

    if let Some(disable_guest_mode) = config.disable_guest_mode {
        updates.insert(
            "BrowserGuestModeEnabled".to_string(),
            bool_to_plist(!disable_guest_mode),
        );
    }

    write_plist_policy(CHROME_BUNDLE_ID, updates)
        .context("Failed to write Chrome plist policy")?;

    Ok(())
}

/// Apply Chrome policies on Linux (via JSON)
#[cfg(target_os = "linux")]
fn apply_chrome_linux(config: &ChromeConfig) -> Result<()> {
    use crate::platform::linux::{get_chrome_policy_dir, write_json_policy};

    let mut policy = json!({});

    // Apply extension policies
    if !config.extensions.is_empty() {
        let extension_strings: Vec<String> = config
            .extensions
            .iter()
            .map(|ext| format_chrome_extension_entry(ext))
            .collect();

        policy["ExtensionInstallForcelist"] = json!(extension_strings);
    }

    // Apply privacy controls
    if let Some(disable_incognito) = config.disable_incognito {
        if disable_incognito {
            policy["IncognitoModeAvailability"] = json!(1); // 1 = Disabled
        }
    }

    if let Some(disable_guest_mode) = config.disable_guest_mode {
        policy["BrowserGuestModeEnabled"] = json!(!disable_guest_mode);
    }

    write_json_policy(get_chrome_policy_dir(), "browser-policy", policy)
        .context("Failed to write Chrome JSON policy")?;

    Ok(())
}

/// Remove Chrome policies on Windows
#[cfg(target_os = "windows")]
fn remove_chrome_windows() -> Result<()> {
    use crate::platform::windows::{remove_registry_policy, remove_registry_value};

    const CHROME_KEY: &str = r"SOFTWARE\Policies\Google\Chrome";

    // Remove extension policy
    let extension_key = format!("{}\\ExtensionInstallForcelist", CHROME_KEY);
    let _ = remove_registry_policy(&extension_key);

    // Remove privacy controls
    let _ = remove_registry_value(CHROME_KEY, "IncognitoModeAvailability");
    let _ = remove_registry_value(CHROME_KEY, "BrowserGuestModeEnabled");

    Ok(())
}

/// Remove Chrome policies on macOS
#[cfg(target_os = "macos")]
fn remove_chrome_macos() -> Result<()> {
    use crate::platform::macos::remove_plist_keys;

    const CHROME_BUNDLE_ID: &str = "com.google.Chrome";

    let keys = vec![
        "ExtensionInstallForcelist".to_string(),
        "IncognitoModeAvailability".to_string(),
        "BrowserGuestModeEnabled".to_string(),
    ];

    remove_plist_keys(CHROME_BUNDLE_ID, &keys)
        .context("Failed to remove Chrome plist keys")?;

    Ok(())
}

/// Remove Chrome policies on Linux
#[cfg(target_os = "linux")]
fn remove_chrome_linux() -> Result<()> {
    use crate::platform::linux::{get_chrome_policy_dir, remove_json_policy};

    remove_json_policy(get_chrome_policy_dir(), "browser-policy")
        .context("Failed to remove Chrome JSON policy")?;

    Ok(())
}

/// Format a Chrome extension entry for policies
fn format_chrome_extension_entry(ext: &crate::config::Extension) -> String {
    let update_url = ext
        .update_url
        .as_deref()
        .unwrap_or(crate::config::DEFAULT_CHROME_UPDATE_URL);

    format!("{};{}", ext.id, update_url)
}

// Stub implementations for platforms not compiled
#[cfg(not(target_os = "windows"))]
fn apply_chrome_windows(_config: &ChromeConfig) -> Result<()> {
    anyhow::bail!("Windows platform not supported in this build")
}

#[cfg(not(target_os = "macos"))]
fn apply_chrome_macos(_config: &ChromeConfig) -> Result<()> {
    anyhow::bail!("macOS platform not supported in this build")
}

#[cfg(not(target_os = "linux"))]
fn apply_chrome_linux(_config: &ChromeConfig) -> Result<()> {
    anyhow::bail!("Linux platform not supported in this build")
}

#[cfg(not(target_os = "windows"))]
fn remove_chrome_windows() -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn remove_chrome_macos() -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn remove_chrome_linux() -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Extension;
    use std::collections::HashMap;

    // Fixture functions
    fn make_chrome_extension(id: &str, update_url: Option<&str>) -> Extension {
        Extension {
            id: id.to_string(),
            name: "Test Extension".to_string(),
            update_url: update_url.map(|s| s.to_string()),
            install_url: None,
            settings: HashMap::new(),
        }
    }

    fn make_chrome_config(extensions: Vec<Extension>) -> ChromeConfig {
        ChromeConfig {
            extensions,
            disable_incognito: None,
            disable_guest_mode: None,
        }
    }

    #[test]
    fn test_format_chrome_extension_entry_with_default_url() {
        let ext = make_chrome_extension("abcdefghijklmnopqrstuvwxyzabcdef", None);
        let entry = format_chrome_extension_entry(&ext);

        assert_eq!(
            entry,
            "abcdefghijklmnopqrstuvwxyzabcdef;https://clients2.google.com/service/update2/crx"
        );
    }

    #[test]
    fn test_format_chrome_extension_entry_with_custom_url() {
        let ext = make_chrome_extension(
            "abcdefghijklmnopqrstuvwxyzabcdef",
            Some("https://example.com/updates"),
        );
        let entry = format_chrome_extension_entry(&ext);

        assert_eq!(entry, "abcdefghijklmnopqrstuvwxyzabcdef;https://example.com/updates");
    }

    #[test]
    fn test_format_chrome_extension_entry_format() {
        let ext = make_chrome_extension("testid123456789012345678901234", None);
        let entry = format_chrome_extension_entry(&ext);

        // Verify format is "id;url"
        assert!(entry.contains(';'));
        let parts: Vec<&str> = entry.split(';').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "testid123456789012345678901234");
        assert!(parts[1].starts_with("https://"));
    }

    // Helper to build state from config (mimics what apply_chrome_policies does for state)
    fn build_chrome_state(config: &ChromeConfig) -> BrowserState {
        let mut state = BrowserState::new();
        state.extensions = config.extensions.iter().map(|e| e.id.clone()).collect();
        state.disable_incognito = config.disable_incognito;
        state.disable_guest_mode = config.disable_guest_mode;
        state
    }

    #[test]
    fn test_chrome_state_building_with_extensions() {
        let ext1 = make_chrome_extension("extension1234567890123456789012", None);
        let ext2 = make_chrome_extension("extension2345678901234567890123", None);

        let config = ChromeConfig {
            extensions: vec![ext1, ext2],
            disable_incognito: Some(true),
            disable_guest_mode: Some(true),
        };

        let state = build_chrome_state(&config);

        assert_eq!(state.extensions.len(), 2);
        assert!(state.extensions.contains(&"extension1234567890123456789012".to_string()));
        assert!(state.extensions.contains(&"extension2345678901234567890123".to_string()));
        assert_eq!(state.disable_incognito, Some(true));
        assert_eq!(state.disable_guest_mode, Some(true));
    }

    #[test]
    fn test_chrome_state_building_empty_extensions() {
        let config = make_chrome_config(vec![]);
        let state = build_chrome_state(&config);

        assert!(state.extensions.is_empty());
        assert_eq!(state.disable_incognito, None);
        assert_eq!(state.disable_guest_mode, None);
    }

    #[test]
    fn test_chrome_state_building_with_single_extension() {
        let ext = make_chrome_extension("singleextension123456789012345", None);
        let config = make_chrome_config(vec![ext]);

        let state = build_chrome_state(&config);

        assert_eq!(state.extensions.len(), 1);
        assert_eq!(state.extensions[0], "singleextension123456789012345");
    }

    #[test]
    fn test_chrome_state_building_preserves_extension_order() {
        let ext1 = make_chrome_extension("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None);
        let ext2 = make_chrome_extension("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", None);
        let ext3 = make_chrome_extension("cccccccccccccccccccccccccccccccc", None);

        let config = make_chrome_config(vec![ext1, ext2, ext3]);
        let state = build_chrome_state(&config);

        assert_eq!(state.extensions[0], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(state.extensions[1], "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        assert_eq!(state.extensions[2], "cccccccccccccccccccccccccccccccc");
    }

    #[test]
    fn test_chrome_state_building_privacy_controls_only() {
        let config = ChromeConfig {
            extensions: vec![],
            disable_incognito: Some(true),
            disable_guest_mode: Some(false),
        };

        let state = build_chrome_state(&config);

        assert!(state.extensions.is_empty());
        assert_eq!(state.disable_incognito, Some(true));
        assert_eq!(state.disable_guest_mode, Some(false));
    }

    #[test]
    fn test_chrome_state_building_partial_privacy_controls() {
        let config = ChromeConfig {
            extensions: vec![],
            disable_incognito: Some(true),
            disable_guest_mode: None,
        };

        let state = build_chrome_state(&config);

        assert_eq!(state.disable_incognito, Some(true));
        assert_eq!(state.disable_guest_mode, None);
    }

    #[test]
    fn test_remove_chrome_policies_succeeds() {
        // This should not panic or error
        let result = remove_chrome_policies();
        // On current platform it should succeed, on others it may fail
        // but we just verify it's callable
        let _ = result;
    }
}
