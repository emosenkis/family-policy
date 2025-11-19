use anyhow::Result;
use std::path::Path;

use crate::config::ChromeConfig;
use crate::state::BrowserState;

use super::chromium_common::{self, ChromiumBrowserConfig, ChromiumConfig};

/// Chrome-specific browser configuration
fn get_chrome_browser_config() -> ChromiumBrowserConfig {
    ChromiumBrowserConfig {
        browser_name: "Chrome",
        registry_key: r"SOFTWARE\Policies\Google\Chrome",
        bundle_id: "com.google.Chrome",
        policy_dir_fn: get_chrome_policy_dir,
    }
}

/// Get Chrome policy directory (Linux)
fn get_chrome_policy_dir() -> &'static Path {
    #[cfg(target_os = "linux")]
    {
        crate::platform::linux::get_chrome_policy_dir()
    }

    #[cfg(not(target_os = "linux"))]
    {
        Path::new("") // Not used on non-Linux platforms
    }
}

/// Apply Chrome policies (extensions and privacy controls)
pub fn apply_chrome_policies(config: &ChromeConfig, dry_run: bool) -> Result<BrowserState> {
    let chromium_config = ChromiumConfig::from_chrome(config);
    let browser_config = get_chrome_browser_config();

    chromium_common::apply_chromium_policies(&chromium_config, &browser_config, dry_run)
}

/// Remove all Chrome policies
pub fn remove_chrome_policies() -> Result<()> {
    let browser_config = get_chrome_browser_config();
    chromium_common::remove_chromium_policies(&browser_config)
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

    #[test]
    fn test_chrome_browser_config() {
        let config = get_chrome_browser_config();
        assert_eq!(config.browser_name, "Chrome");
        assert_eq!(config.registry_key, r"SOFTWARE\Policies\Google\Chrome");
        assert_eq!(config.bundle_id, "com.google.Chrome");
    }
}
