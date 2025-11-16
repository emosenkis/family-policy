use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::browser::Browser;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub policies: Vec<PolicyEntry>,
}

/// A single policy entry that can apply to multiple browsers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyEntry {
    pub name: String,
    pub browsers: Vec<Browser>,

    // Privacy controls (apply to all browsers, with browser-specific translations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_private_mode: Option<bool>,  // Chrome: incognito, Firefox: private browsing, Edge: InPrivate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_guest_mode: Option<bool>,    // Chrome and Edge only (ignored for Firefox)

    // Extensions
    #[serde(default)]
    pub extensions: Vec<ExtensionEntry>,
}

/// Extension entry with browser-specific IDs and arbitrary settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtensionEntry {
    pub name: String,
    pub id: BrowserIdMap,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_installed: Option<bool>,  // Default: true

    /// Arbitrary extension-specific settings (e.g., for uBO Lite)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub settings: HashMap<String, serde_json::Value>,
}

/// Browser-specific extension IDs
/// Can be either a single string (same ID for all browsers) or a map of browser -> ID
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BrowserIdMap {
    Single(String),
    Multiple(HashMap<Browser, String>),
}

impl BrowserIdMap {
    /// Get the extension ID for a specific browser
    pub fn get_id(&self, browser: Browser) -> Option<&str> {
        match self {
            BrowserIdMap::Single(id) => Some(id.as_str()),
            BrowserIdMap::Multiple(map) => map.get(&browser).map(|s| s.as_str()),
        }
    }
}

/// Legacy Chrome-specific configuration (for internal use)
#[derive(Debug, Clone)]
pub struct ChromeConfig {
    pub extensions: Vec<Extension>,
    pub disable_incognito: Option<bool>,
    pub disable_guest_mode: Option<bool>,
}

/// Legacy Firefox-specific configuration (for internal use)
#[derive(Debug, Clone)]
pub struct FirefoxConfig {
    pub extensions: Vec<Extension>,
    pub disable_private_browsing: Option<bool>,
}

/// Legacy Edge-specific configuration (for internal use)
#[derive(Debug, Clone)]
pub struct EdgeConfig {
    pub extensions: Vec<Extension>,
    pub disable_inprivate: Option<bool>,
    pub disable_guest_mode: Option<bool>,
}

/// Legacy extension definition (for internal use by policy modules)
#[derive(Debug, Clone)]
pub struct Extension {
    pub id: String,
    pub name: String,
    /// For Chrome/Edge - update URL (optional, has default)
    pub update_url: Option<String>,
    /// For Firefox - install URL (required for Firefox)
    pub install_url: Option<String>,
    /// Extension-specific settings (e.g., for uBO Lite configuration)
    ///
    /// NOTE: This field is populated from config but not yet used by policy implementations.
    /// It's reserved for future functionality to pass extension-specific configuration.
    #[allow(dead_code)] // Reserved for future extension configuration support
    pub settings: HashMap<String, serde_json::Value>,
}

/// Default Chrome Web Store update URL
pub const DEFAULT_CHROME_UPDATE_URL: &str = "https://clients2.google.com/service/update2/crx";

/// Firefox Add-ons base URL for generating install URLs
const FIREFOX_ADDONS_BASE: &str = "https://addons.mozilla.org/firefox/downloads/latest";

/// Load configuration from a YAML file
pub fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML config file: {}", path.display()))?;

    // Validate the config
    validate_config(&config)?;

    Ok(config)
}

/// Validate configuration
pub fn validate_config(config: &Config) -> Result<()> {
    // Ensure at least one policy is configured
    if config.policies.is_empty() {
        anyhow::bail!("Configuration must specify at least one policy");
    }

    // Validate each policy entry
    for policy in &config.policies {
        validate_policy_entry(policy)
            .with_context(|| format!("Invalid policy '{}'", policy.name))?;
    }

    Ok(())
}

/// Validate a single policy entry
fn validate_policy_entry(policy: &PolicyEntry) -> Result<()> {
    // Ensure at least one browser is specified
    if policy.browsers.is_empty() {
        anyhow::bail!("Policy must specify at least one browser");
    }

    // Validate each extension
    for ext in &policy.extensions {
        validate_extension_entry(ext, &policy.browsers)
            .with_context(|| format!("Invalid extension '{}'", ext.name))?;
    }

    Ok(())
}

/// Validate an extension entry
fn validate_extension_entry(ext: &ExtensionEntry, browsers: &[Browser]) -> Result<()> {
    // Validate that the extension has IDs for the required browsers
    for browser in browsers {
        let id = ext.id.get_id(*browser);
        if id.is_none() {
            anyhow::bail!(
                "Extension '{}' does not have an ID for browser '{}'",
                ext.name,
                browser.as_str()
            );
        }

        let id = id.unwrap();

        // Validate ID format based on browser
        match browser {
            Browser::Chrome | Browser::Edge => {
                // Chrome/Edge extension IDs should be 32 lowercase alphanumeric characters
                if id.len() != 32 {
                    anyhow::bail!(
                        "Extension '{}' has invalid {} ID length: expected 32 characters, got {}",
                        ext.name,
                        browser.as_str(),
                        id.len()
                    );
                }

                if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
                    anyhow::bail!(
                        "Extension '{}' has invalid {} ID: must contain only lowercase letters and digits",
                        ext.name,
                        browser.as_str()
                    );
                }
            }
            Browser::Firefox => {
                // Firefox extension IDs are typically in format: name@domain or {uuid}
                if id.is_empty() {
                    anyhow::bail!("Extension '{}' has empty Firefox ID", ext.name);
                }
            }
        }
    }

    Ok(())
}

/// Convert the new config format to browser-specific configurations
pub fn to_browser_configs(config: &Config) -> (Option<ChromeConfig>, Option<FirefoxConfig>, Option<EdgeConfig>) {
    let mut chrome_extensions = Vec::new();
    let mut firefox_extensions = Vec::new();
    let mut edge_extensions = Vec::new();

    let mut chrome_disable_incognito = None;
    let mut chrome_disable_guest_mode = None;
    let mut firefox_disable_private_browsing = None;
    let mut edge_disable_inprivate = None;
    let mut edge_disable_guest_mode = None;

    // Process each policy entry
    for policy in &config.policies {
        // Process privacy settings
        for browser in &policy.browsers {
            match browser {
                Browser::Chrome => {
                    if let Some(disable) = policy.disable_private_mode {
                        chrome_disable_incognito = Some(disable);
                    }
                    if let Some(disable) = policy.disable_guest_mode {
                        chrome_disable_guest_mode = Some(disable);
                    }
                }
                Browser::Firefox => {
                    if let Some(disable) = policy.disable_private_mode {
                        firefox_disable_private_browsing = Some(disable);
                    }
                    // Firefox doesn't have guest mode - ignore
                }
                Browser::Edge => {
                    if let Some(disable) = policy.disable_private_mode {
                        edge_disable_inprivate = Some(disable);
                    }
                    if let Some(disable) = policy.disable_guest_mode {
                        edge_disable_guest_mode = Some(disable);
                    }
                }
            }
        }

        // Process extensions
        for ext_entry in &policy.extensions {
            for browser in &policy.browsers {
                if let Some(id) = ext_entry.id.get_id(*browser) {
                    let extension = Extension {
                        id: id.to_string(),
                        name: ext_entry.name.clone(),
                        update_url: match browser {
                            Browser::Chrome | Browser::Edge => Some(DEFAULT_CHROME_UPDATE_URL.to_string()),
                            Browser::Firefox => None,
                        },
                        install_url: match browser {
                            Browser::Firefox => Some(generate_firefox_install_url(id)),
                            _ => None,
                        },
                        settings: ext_entry.settings.clone(),
                    };

                    match browser {
                        Browser::Chrome => chrome_extensions.push(extension),
                        Browser::Firefox => firefox_extensions.push(extension),
                        Browser::Edge => edge_extensions.push(extension),
                    }
                }
            }
        }
    }

    let chrome_config = if !chrome_extensions.is_empty() || chrome_disable_incognito.is_some() || chrome_disable_guest_mode.is_some() {
        Some(ChromeConfig {
            extensions: chrome_extensions,
            disable_incognito: chrome_disable_incognito,
            disable_guest_mode: chrome_disable_guest_mode,
        })
    } else {
        None
    };

    let firefox_config = if !firefox_extensions.is_empty() || firefox_disable_private_browsing.is_some() {
        Some(FirefoxConfig {
            extensions: firefox_extensions,
            disable_private_browsing: firefox_disable_private_browsing,
        })
    } else {
        None
    };

    let edge_config = if !edge_extensions.is_empty() || edge_disable_inprivate.is_some() || edge_disable_guest_mode.is_some() {
        Some(EdgeConfig {
            extensions: edge_extensions,
            disable_inprivate: edge_disable_inprivate,
            disable_guest_mode: edge_disable_guest_mode,
        })
    } else {
        None
    };

    (chrome_config, firefox_config, edge_config)
}

/// Generate Firefox add-on install URL from extension ID
fn generate_firefox_install_url(id: &str) -> String {
    // For Firefox, we can generate the install URL from the extension ID
    // Format: https://addons.mozilla.org/firefox/downloads/latest/{slug}/latest.xpi
    // We'll use the ID as the slug for now (this may need adjustment based on actual add-on slugs)
    format!("{}/{}/latest.xpi", FIREFOX_ADDONS_BASE, id)
}

/// Example configuration file with comprehensive documentation
///
/// The content is loaded from example-config.yaml at compile time
pub const EXAMPLE_CONFIG: &str = include_str!("../example-config.yaml");

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper to create a temporary YAML config file for testing
    fn create_temp_yaml_config(content: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    // Config Validation Tests

    #[test]
    fn config_with_no_policies_fails_validation() {
        let config = Config {
            policies: vec![],
        };
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn config_with_valid_policy_passes_validation() {
        let yaml = r#"
policies:
  - name: Test Policy
    browsers:
      - chrome
    extensions:
      - name: Test Extension
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
"#;
        let file = create_temp_yaml_config(yaml);
        let config = load_config(file.path()).unwrap();
        assert_eq!(config.policies.len(), 1);
        assert_eq!(config.policies[0].name, "Test Policy");
    }

    #[test]
    fn policy_with_no_browsers_fails_validation() {
        let yaml = r#"
policies:
  - name: Test Policy
    browsers: []
"#;
        let file = create_temp_yaml_config(yaml);
        assert!(load_config(file.path()).is_err());
    }

    #[test]
    fn policy_with_invalid_chrome_id_fails_validation() {
        let yaml = r#"
policies:
  - name: Test Policy
    browsers:
      - chrome
    extensions:
      - name: Test Extension
        id:
          chrome: invalid
"#;
        let file = create_temp_yaml_config(yaml);
        assert!(load_config(file.path()).is_err());
    }

    #[test]
    fn policy_with_missing_browser_id_fails_validation() {
        let yaml = r#"
policies:
  - name: Test Policy
    browsers:
      - chrome
      - firefox
    extensions:
      - name: Test Extension
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
"#;
        let file = create_temp_yaml_config(yaml);
        assert!(load_config(file.path()).is_err());
    }

    #[test]
    fn policy_with_privacy_settings_passes_validation() {
        let yaml = r#"
policies:
  - name: Privacy Policy
    browsers:
      - chrome
      - firefox
      - edge
    disable_private_mode: true
    disable_guest_mode: true
"#;
        let file = create_temp_yaml_config(yaml);
        let config = load_config(file.path()).unwrap();
        assert_eq!(config.policies[0].disable_private_mode, Some(true));
        assert_eq!(config.policies[0].disable_guest_mode, Some(true));
    }

    #[test]
    fn policy_with_extension_settings_passes_validation() {
        let yaml = r#"
policies:
  - name: uBO Lite
    browsers:
      - chrome
    extensions:
      - name: uBlock Origin Lite
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
        force_installed: true
        settings:
          rulesets:
            - "+default"
            - "+isr-0"
          strictBlockMode: true
"#;
        let file = create_temp_yaml_config(yaml);
        let config = load_config(file.path()).unwrap();
        let ext = &config.policies[0].extensions[0];
        assert_eq!(ext.settings.len(), 2);
        assert!(ext.settings.contains_key("rulesets"));
        assert!(ext.settings.contains_key("strictBlockMode"));
    }

    #[test]
    fn multiple_policies_are_parsed_correctly() {
        let yaml = r#"
policies:
  - name: Privacy Policy
    browsers:
      - chrome
      - firefox
    disable_private_mode: true
  - name: Extensions Policy
    browsers:
      - chrome
    extensions:
      - name: Test Extension
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
"#;
        let file = create_temp_yaml_config(yaml);
        let config = load_config(file.path()).unwrap();
        assert_eq!(config.policies.len(), 2);
    }

    // Browser ID Map Tests

    #[test]
    fn browser_id_map_single_returns_same_id_for_all_browsers() {
        let id_map = BrowserIdMap::Single("test-id".to_string());
        assert_eq!(id_map.get_id(Browser::Chrome), Some("test-id"));
        assert_eq!(id_map.get_id(Browser::Firefox), Some("test-id"));
        assert_eq!(id_map.get_id(Browser::Edge), Some("test-id"));
    }

    #[test]
    fn browser_id_map_multiple_returns_correct_ids() {
        let mut map = HashMap::new();
        map.insert(Browser::Chrome, "chrome-id".to_string());
        map.insert(Browser::Firefox, "firefox-id".to_string());
        let id_map = BrowserIdMap::Multiple(map);

        assert_eq!(id_map.get_id(Browser::Chrome), Some("chrome-id"));
        assert_eq!(id_map.get_id(Browser::Firefox), Some("firefox-id"));
        assert_eq!(id_map.get_id(Browser::Edge), None);
    }

    // Config Conversion Tests

    #[test]
    fn to_browser_configs_creates_correct_chrome_config() {
        let yaml = r#"
policies:
  - name: Test Policy
    browsers:
      - chrome
    disable_private_mode: true
    extensions:
      - name: Test Extension
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
"#;
        let file = create_temp_yaml_config(yaml);
        let config = load_config(file.path()).unwrap();
        let (chrome, firefox, edge) = to_browser_configs(&config);

        assert!(chrome.is_some());
        assert!(firefox.is_none());
        assert!(edge.is_none());

        let chrome = chrome.unwrap();
        assert_eq!(chrome.extensions.len(), 1);
        assert_eq!(chrome.disable_incognito, Some(true));
        assert_eq!(chrome.extensions[0].id, "ddkjiahejlhfcafbddmgiahcphecmpfh");
    }

    #[test]
    fn to_browser_configs_handles_multiple_browsers() {
        let yaml = r#"
policies:
  - name: Test Policy
    browsers:
      - chrome
      - firefox
      - edge
    extensions:
      - name: Test Extension
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
          firefox: test@example.com
          edge: ddkjiahejlhfcafbddmgiahcphecmpfh
"#;
        let file = create_temp_yaml_config(yaml);
        let config = load_config(file.path()).unwrap();
        let (chrome, firefox, edge) = to_browser_configs(&config);

        assert!(chrome.is_some());
        assert!(firefox.is_some());
        assert!(edge.is_some());

        assert_eq!(chrome.unwrap().extensions[0].id, "ddkjiahejlhfcafbddmgiahcphecmpfh");
        assert_eq!(firefox.unwrap().extensions[0].id, "test@example.com");
        assert_eq!(edge.unwrap().extensions[0].id, "ddkjiahejlhfcafbddmgiahcphecmpfh");
    }

    #[test]
    fn to_browser_configs_preserves_extension_settings() {
        let yaml = r#"
policies:
  - name: Test Policy
    browsers:
      - chrome
    extensions:
      - name: Test Extension
        id:
          chrome: ddkjiahejlhfcafbddmgiahcphecmpfh
        settings:
          key1: value1
          key2: 42
"#;
        let file = create_temp_yaml_config(yaml);
        let config = load_config(file.path()).unwrap();
        let (chrome, _, _) = to_browser_configs(&config);

        let chrome = chrome.unwrap();
        let ext_settings = &chrome.extensions[0].settings;
        assert_eq!(ext_settings.len(), 2);
        assert!(ext_settings.contains_key("key1"));
        assert!(ext_settings.contains_key("key2"));
    }
}
