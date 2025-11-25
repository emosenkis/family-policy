use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::browser::Browser;
use crate::config::{Config, ChromeConfig, FirefoxConfig, EdgeConfig, Extension};
use crate::state::{State, BrowserState};

/// Complete policy diff across all browsers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDiff {
    pub chrome: Option<BrowserDiff>,
    pub firefox: Option<BrowserDiff>,
    pub edge: Option<BrowserDiff>,
    pub summary: DiffSummary,
}

/// Diff for a single browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserDiff {
    pub browser: Browser,
    pub extensions: Vec<ExtensionDiff>,
    pub privacy_settings: Vec<PrivacySettingDiff>,
}

/// Diff for an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionDiff {
    Added {
        id: String,
        name: String,
    },
    Removed {
        id: String,
        name: Option<String>,
    },
    Unchanged {
        id: String,
        name: String,
    },
}

/// Diff for a privacy setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettingDiff {
    pub setting_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// Summary of changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_additions: usize,
    pub total_removals: usize,
    pub total_changes: usize,
}

/// Generate a diff between proposed config and current state
///
/// # Arguments
/// * `new_config` - Proposed configuration
/// * `current_state` - Current applied state (if any)
///
/// # Returns
/// * `PolicyDiff` describing all changes
pub fn generate_diff(new_config: &Config, current_state: Option<&State>) -> PolicyDiff {
    let (new_chrome, new_firefox, new_edge) = crate::config::to_browser_configs(new_config);

    let chrome_diff = new_chrome.map(|chrome_config| {
        let current_chrome = current_state
            .and_then(|s| s.applied_policies.chrome.as_ref());
        generate_chrome_diff(&chrome_config, current_chrome)
    });

    let firefox_diff = new_firefox.map(|firefox_config| {
        let current_firefox = current_state
            .and_then(|s| s.applied_policies.firefox.as_ref());
        generate_firefox_diff(&firefox_config, current_firefox)
    });

    let edge_diff = new_edge.map(|edge_config| {
        let current_edge = current_state
            .and_then(|s| s.applied_policies.edge.as_ref());
        generate_edge_diff(&edge_config, current_edge)
    });

    let summary = create_summary(&chrome_diff, &firefox_diff, &edge_diff);

    PolicyDiff {
        chrome: chrome_diff,
        firefox: firefox_diff,
        edge: edge_diff,
        summary,
    }
}

fn generate_chrome_diff(
    new_config: &ChromeConfig,
    current_state: Option<&BrowserState>,
) -> BrowserDiff {
    let extensions = generate_extension_diffs(&new_config.extensions, current_state);
    let privacy_settings = generate_chrome_privacy_diff(new_config, current_state);

    BrowserDiff {
        browser: Browser::Chrome,
        extensions,
        privacy_settings,
    }
}

fn generate_firefox_diff(
    new_config: &FirefoxConfig,
    current_state: Option<&BrowserState>,
) -> BrowserDiff {
    let extensions = generate_extension_diffs(&new_config.extensions, current_state);
    let privacy_settings = generate_firefox_privacy_diff(new_config, current_state);

    BrowserDiff {
        browser: Browser::Firefox,
        extensions,
        privacy_settings,
    }
}

fn generate_edge_diff(
    new_config: &EdgeConfig,
    current_state: Option<&BrowserState>,
) -> BrowserDiff {
    let extensions = generate_extension_diffs(&new_config.extensions, current_state);
    let privacy_settings = generate_edge_privacy_diff(new_config, current_state);

    BrowserDiff {
        browser: Browser::Edge,
        extensions,
        privacy_settings,
    }
}

fn generate_extension_diffs(
    new_extensions: &[Extension],
    current_state: Option<&BrowserState>,
) -> Vec<ExtensionDiff> {
    let mut diffs = Vec::new();

    // Get current extension IDs
    let current_ids: HashSet<String> = current_state
        .map(|s| s.extensions.iter().cloned().collect())
        .unwrap_or_default();

    // Create a map of new extension IDs to names
    let new_extensions_map: HashMap<String, String> = new_extensions
        .iter()
        .map(|ext| (ext.id.clone(), ext.name.clone()))
        .collect();

    let new_ids: HashSet<String> = new_extensions_map.keys().cloned().collect();

    // Find additions and unchanged
    for (id, name) in &new_extensions_map {
        if current_ids.contains(id) {
            diffs.push(ExtensionDiff::Unchanged {
                id: id.clone(),
                name: name.clone(),
            });
        } else {
            diffs.push(ExtensionDiff::Added {
                id: id.clone(),
                name: name.clone(),
            });
        }
    }

    // Find removals
    for id in &current_ids {
        if !new_ids.contains(id) {
            diffs.push(ExtensionDiff::Removed {
                id: id.clone(),
                name: None,
            });
        }
    }

    diffs
}

fn generate_chrome_privacy_diff(
    new_config: &ChromeConfig,
    current_state: Option<&BrowserState>,
) -> Vec<PrivacySettingDiff> {
    let mut diffs = Vec::new();

    let old_incognito = current_state.and_then(|s| s.disable_incognito);
    let new_incognito = new_config.disable_incognito;
    if old_incognito != new_incognito {
        diffs.push(PrivacySettingDiff {
            setting_name: "Disable Incognito Mode".to_string(),
            old_value: old_incognito.map(|v| v.to_string()),
            new_value: new_incognito.map(|v| v.to_string()),
        });
    }

    let old_guest = current_state.and_then(|s| s.disable_guest_mode);
    let new_guest = new_config.disable_guest_mode;
    if old_guest != new_guest {
        diffs.push(PrivacySettingDiff {
            setting_name: "Disable Guest Mode".to_string(),
            old_value: old_guest.map(|v| v.to_string()),
            new_value: new_guest.map(|v| v.to_string()),
        });
    }

    diffs
}

fn generate_firefox_privacy_diff(
    new_config: &FirefoxConfig,
    current_state: Option<&BrowserState>,
) -> Vec<PrivacySettingDiff> {
    let mut diffs = Vec::new();

    let old_private = current_state.and_then(|s| s.disable_private_browsing);
    let new_private = new_config.disable_private_browsing;
    if old_private != new_private {
        diffs.push(PrivacySettingDiff {
            setting_name: "Disable Private Browsing".to_string(),
            old_value: old_private.map(|v| v.to_string()),
            new_value: new_private.map(|v| v.to_string()),
        });
    }

    diffs
}

fn generate_edge_privacy_diff(
    new_config: &EdgeConfig,
    current_state: Option<&BrowserState>,
) -> Vec<PrivacySettingDiff> {
    let mut diffs = Vec::new();

    let old_inprivate = current_state.and_then(|s| s.disable_inprivate);
    let new_inprivate = new_config.disable_inprivate;
    if old_inprivate != new_inprivate {
        diffs.push(PrivacySettingDiff {
            setting_name: "Disable InPrivate Mode".to_string(),
            old_value: old_inprivate.map(|v| v.to_string()),
            new_value: new_inprivate.map(|v| v.to_string()),
        });
    }

    let old_guest = current_state.and_then(|s| s.disable_guest_mode);
    let new_guest = new_config.disable_guest_mode;
    if old_guest != new_guest {
        diffs.push(PrivacySettingDiff {
            setting_name: "Disable Guest Mode".to_string(),
            old_value: old_guest.map(|v| v.to_string()),
            new_value: new_guest.map(|v| v.to_string()),
        });
    }

    diffs
}

fn create_summary(
    chrome: &Option<BrowserDiff>,
    firefox: &Option<BrowserDiff>,
    edge: &Option<BrowserDiff>,
) -> DiffSummary {
    let mut total_additions = 0;
    let mut total_removals = 0;
    let mut total_changes = 0;

    for diff in [chrome, firefox, edge].iter().filter_map(|d| d.as_ref()) {
        for ext_diff in &diff.extensions {
            match ext_diff {
                ExtensionDiff::Added { .. } => total_additions += 1,
                ExtensionDiff::Removed { .. } => total_removals += 1,
                ExtensionDiff::Unchanged { .. } => {}
            }
        }
        total_changes += diff.privacy_settings.len();
    }

    DiffSummary {
        total_additions,
        total_removals,
        total_changes,
    }
}

/// Pretty-print a diff for CLI output
pub fn print_diff(diff: &PolicyDiff) {
    println!("Policy Changes:");
    println!();

    if let Some(chrome_diff) = &diff.chrome {
        print_browser_diff("Chrome", chrome_diff);
    }

    if let Some(firefox_diff) = &diff.firefox {
        print_browser_diff("Firefox", firefox_diff);
    }

    if let Some(edge_diff) = &diff.edge {
        print_browser_diff("Edge", edge_diff);
    }

    println!();
    println!("Summary:");
    println!("  Additions: {}", diff.summary.total_additions);
    println!("  Removals: {}", diff.summary.total_removals);
    println!("  Changes: {}", diff.summary.total_changes);
}

fn print_browser_diff(browser_name: &str, diff: &BrowserDiff) {
    println!("{}:", browser_name);

    for ext_diff in &diff.extensions {
        match ext_diff {
            ExtensionDiff::Added { id, name } => {
                println!("  + Add extension: {} ({})", name, id);
            }
            ExtensionDiff::Removed { id, name } => {
                let name_str = name.as_deref().unwrap_or("unknown");
                println!("  - Remove extension: {} ({})", name_str, id);
            }
            ExtensionDiff::Unchanged { .. } => {
                // Don't print unchanged items unless verbose mode
            }
        }
    }

    for privacy_diff in &diff.privacy_settings {
        let old_val = privacy_diff.old_value.as_deref().unwrap_or("none");
        let new_val = privacy_diff.new_value.as_deref().unwrap_or("none");
        println!(
            "  ~ {}: {} -> {}",
            privacy_diff.setting_name, old_val, new_val
        );
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::Browser;

    #[test]
    fn test_diff_summary_empty() {
        let summary = create_summary(&None, &None, &None);
        assert_eq!(summary.total_additions, 0);
        assert_eq!(summary.total_removals, 0);
        assert_eq!(summary.total_changes, 0);
    }

    #[test]
    fn test_extension_diff_additions() {
        let new_extensions = vec![Extension {
            id: "test-id".to_string(),
            name: "Test Extension".to_string(),
            update_url: None,
            install_url: None,
            settings: HashMap::new(),
        }];

        let diffs = generate_extension_diffs(&new_extensions, None);

        assert_eq!(diffs.len(), 1);
        match &diffs[0] {
            ExtensionDiff::Added { id, name } => {
                assert_eq!(id, "test-id");
                assert_eq!(name, "Test Extension");
            }
            _ => panic!("Expected Added diff"),
        }
    }

    #[test]
    fn test_extension_diff_removals() {
        let current_state = BrowserState {
            extensions: vec!["removed-id".to_string()],
            disable_incognito: None,
            disable_inprivate: None,
            disable_private_browsing: None,
            disable_guest_mode: None,
        };

        let diffs = generate_extension_diffs(&[], Some(&current_state));

        assert_eq!(diffs.len(), 1);
        match &diffs[0] {
            ExtensionDiff::Removed { id, .. } => {
                assert_eq!(id, "removed-id");
            }
            _ => panic!("Expected Removed diff"),
        }
    }

    #[test]
    fn test_extension_diff_unchanged() {
        let new_extensions = vec![Extension {
            id: "test-id".to_string(),
            name: "Test Extension".to_string(),
            update_url: None,
            install_url: None,
            settings: HashMap::new(),
        }];

        let current_state = BrowserState {
            extensions: vec!["test-id".to_string()],
            disable_incognito: None,
            disable_inprivate: None,
            disable_private_browsing: None,
            disable_guest_mode: None,
        };

        let diffs = generate_extension_diffs(&new_extensions, Some(&current_state));

        assert_eq!(diffs.len(), 1);
        match &diffs[0] {
            ExtensionDiff::Unchanged { id, .. } => {
                assert_eq!(id, "test-id");
            }
            _ => panic!("Expected Unchanged diff"),
        }
    }

    #[test]
    fn test_chrome_privacy_diff() {
        let new_config = ChromeConfig {
            extensions: vec![],
            disable_incognito: Some(true),
            disable_guest_mode: Some(false),
        };

        let current_state = BrowserState {
            extensions: vec![],
            disable_incognito: Some(false),
            disable_inprivate: None,
            disable_private_browsing: None,
            disable_guest_mode: None,
        };

        let diffs = generate_chrome_privacy_diff(&new_config, Some(&current_state));

        assert_eq!(diffs.len(), 2); // incognito changed, guest mode added
        assert!(diffs.iter().any(|d| d.setting_name.contains("Incognito")));
        assert!(diffs.iter().any(|d| d.setting_name.contains("Guest")));
    }
}
