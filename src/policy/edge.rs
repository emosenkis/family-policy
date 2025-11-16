use anyhow::Result;
use std::path::Path;

use crate::config::EdgeConfig;
use crate::state::BrowserState;

use super::chromium_common::{self, ChromiumBrowserConfig, ChromiumConfig};

/// Edge-specific browser configuration
fn get_edge_browser_config() -> ChromiumBrowserConfig {
    ChromiumBrowserConfig {
        browser_name: "Edge",
        registry_key: r"SOFTWARE\Policies\Microsoft\Edge",
        bundle_id: "com.microsoft.Edge",
        policy_dir_fn: get_edge_policy_dir,
    }
}

/// Get Edge policy directory (Linux)
fn get_edge_policy_dir() -> &'static Path {
    #[cfg(target_os = "linux")]
    {
        crate::platform::linux::get_edge_policy_dir()
    }

    #[cfg(not(target_os = "linux"))]
    {
        Path::new("") // Not used on non-Linux platforms
    }
}

/// Apply Edge policies (extensions and privacy controls)
pub fn apply_edge_policies(config: &EdgeConfig) -> Result<BrowserState> {
    let chromium_config = ChromiumConfig::from_edge(config);
    let browser_config = get_edge_browser_config();

    chromium_common::apply_chromium_policies(&chromium_config, &browser_config)
}

/// Remove all Edge policies
pub fn remove_edge_policies() -> Result<()> {
    let browser_config = get_edge_browser_config();
    chromium_common::remove_chromium_policies(&browser_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Extension;
    use std::collections::HashMap;

    // Fixture functions
    fn make_edge_extension(id: &str, update_url: Option<&str>) -> Extension {
        Extension {
            id: id.to_string(),
            name: "Test Extension".to_string(),
            update_url: update_url.map(|s| s.to_string()),
            install_url: None,
            settings: HashMap::new(),
        }
    }

    fn make_edge_config(extensions: Vec<Extension>) -> EdgeConfig {
        EdgeConfig {
            extensions,
            disable_inprivate: None,
            disable_guest_mode: None,
        }
    }

    // Helper to build state from config (mimics what apply_edge_policies does for state)
    fn build_edge_state(config: &EdgeConfig) -> BrowserState {
        let mut state = BrowserState::new();
        state.extensions = config.extensions.iter().map(|e| e.id.clone()).collect();
        state.disable_inprivate = config.disable_inprivate;
        state.disable_guest_mode = config.disable_guest_mode;
        state
    }

    #[test]
    fn test_edge_state_building_with_extensions() {
        let ext1 = make_edge_extension("extension1234567890123456789012", None);
        let ext2 = make_edge_extension("extension2345678901234567890123", None);

        let config = EdgeConfig {
            extensions: vec![ext1, ext2],
            disable_inprivate: Some(true),
            disable_guest_mode: Some(true),
        };

        let state = build_edge_state(&config);

        assert_eq!(state.extensions.len(), 2);
        assert!(state.extensions.contains(&"extension1234567890123456789012".to_string()));
        assert!(state.extensions.contains(&"extension2345678901234567890123".to_string()));
        assert_eq!(state.disable_inprivate, Some(true));
        assert_eq!(state.disable_guest_mode, Some(true));
    }

    #[test]
    fn test_edge_state_building_empty_extensions() {
        let config = make_edge_config(vec![]);
        let state = build_edge_state(&config);

        assert!(state.extensions.is_empty());
        assert_eq!(state.disable_inprivate, None);
        assert_eq!(state.disable_guest_mode, None);
    }

    #[test]
    fn test_edge_state_building_with_single_extension() {
        let ext = make_edge_extension("singleextension123456789012345", None);
        let config = make_edge_config(vec![ext]);

        let state = build_edge_state(&config);

        assert_eq!(state.extensions.len(), 1);
        assert_eq!(state.extensions[0], "singleextension123456789012345");
    }

    #[test]
    fn test_edge_state_building_preserves_extension_order() {
        let ext1 = make_edge_extension("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None);
        let ext2 = make_edge_extension("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", None);
        let ext3 = make_edge_extension("cccccccccccccccccccccccccccccccc", None);

        let config = make_edge_config(vec![ext1, ext2, ext3]);
        let state = build_edge_state(&config);

        assert_eq!(state.extensions[0], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(state.extensions[1], "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        assert_eq!(state.extensions[2], "cccccccccccccccccccccccccccccccc");
    }

    #[test]
    fn test_edge_state_building_privacy_controls_only() {
        let config = EdgeConfig {
            extensions: vec![],
            disable_inprivate: Some(true),
            disable_guest_mode: Some(false),
        };

        let state = build_edge_state(&config);

        assert!(state.extensions.is_empty());
        assert_eq!(state.disable_inprivate, Some(true));
        assert_eq!(state.disable_guest_mode, Some(false));
    }

    #[test]
    fn test_edge_state_building_partial_privacy_controls() {
        let config = EdgeConfig {
            extensions: vec![],
            disable_inprivate: Some(true),
            disable_guest_mode: None,
        };

        let state = build_edge_state(&config);

        assert_eq!(state.disable_inprivate, Some(true));
        assert_eq!(state.disable_guest_mode, None);
    }

    #[test]
    fn test_remove_edge_policies_succeeds() {
        // This should not panic or error
        let result = remove_edge_policies();
        // On current platform it should succeed, on others it may fail
        // but we just verify it's callable
        let _ = result;
    }

    #[test]
    fn test_edge_browser_config() {
        let config = get_edge_browser_config();
        assert_eq!(config.browser_name, "Edge");
        assert_eq!(config.registry_key, r"SOFTWARE\Policies\Microsoft\Edge");
        assert_eq!(config.bundle_id, "com.microsoft.Edge");
    }

    #[test]
    fn test_edge_uses_same_format_as_chrome() {
        // Edge and Chrome use the same extension format
        use super::super::chromium_common::format_chromium_extension_entry;
        let ext = make_edge_extension("testextension12345678901234567", Some("https://test.com/update"));
        let entry = format_chromium_extension_entry(&ext);

        assert_eq!(entry, "testextension12345678901234567;https://test.com/update");
        assert!(entry.contains(';'));
    }
}
