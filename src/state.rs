use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::config::Config;

/// Current state version
const STATE_VERSION: &str = "1.0";

/// State tracking for idempotent operations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct State {
    pub version: String,
    pub config_hash: String,
    pub last_updated: DateTime<Utc>,
    pub applied_policies: AppliedPolicies,
}

/// Applied policies for all browsers
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppliedPolicies {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chrome: Option<BrowserState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firefox: Option<BrowserState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<BrowserState>,
}

/// State for a single browser
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrowserState {
    pub extensions: Vec<String>, // Extension IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_incognito: Option<bool>, // Chrome only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_inprivate: Option<bool>, // Edge only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_private_browsing: Option<bool>, // Firefox only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_guest_mode: Option<bool>, // Chrome/Edge only
}

impl BrowserState {
    /// Create a new empty browser state
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
            disable_incognito: None,
            disable_inprivate: None,
            disable_private_browsing: None,
            disable_guest_mode: None,
        }
    }

    /// Check if this state is empty (no policies applied)
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
            && self.disable_incognito.is_none()
            && self.disable_inprivate.is_none()
            && self.disable_private_browsing.is_none()
            && self.disable_guest_mode.is_none()
    }
}

impl Default for BrowserState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the platform-specific state file path
pub fn get_state_path() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        // Try system location first, fall back to user location
        let system_path = PathBuf::from("/var/lib/browser-extension-policy/state.json");
        if system_path.parent().map(|p| p.exists()).unwrap_or(false) {
            return Ok(system_path);
        }

        // Fall back to user location
        if let Some(data_dir) = directories::ProjectDirs::from("", "", "browser-extension-policy")
        {
            let mut path = data_dir.data_local_dir().to_path_buf();
            path.push("state.json");
            return Ok(path);
        }

        anyhow::bail!("Could not determine state file location");
    }

    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from(
            "/Library/Application Support/browser-extension-policy/state.json",
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let mut path = PathBuf::from(
            std::env::var("ProgramData")
                .unwrap_or_else(|_| "C:\\ProgramData".to_string()),
        );
        path.push("browser-extension-policy");
        path.push("state.json");
        Ok(path)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        anyhow::bail!("Unsupported operating system");
    }
}

/// Load state from the state file
pub fn load_state() -> Result<Option<State>> {
    let state_path = get_state_path()?;

    if !state_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read state file: {}", state_path.display()))?;

    let state: State = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse state file: {}", state_path.display()))?;

    // Validate state version
    if state.version != STATE_VERSION {
        eprintln!(
            "Warning: State file version mismatch (expected {}, got {}). Treating as new state.",
            STATE_VERSION, state.version
        );
        return Ok(None);
    }

    Ok(Some(state))
}

/// Save state to the state file
pub fn save_state(state: &State) -> Result<()> {
    let state_path = get_state_path()?;

    // Ensure parent directory exists
    if let Some(parent) = state_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create state directory: {}",
                parent.display()
            )
        })?;
    }

    // Serialize state to JSON
    let content = serde_json::to_string_pretty(state)
        .context("Failed to serialize state")?;

    // Write atomically
    crate::platform::common::atomic_write(&state_path, content.as_bytes())
        .with_context(|| format!("Failed to write state file: {}", state_path.display()))?;

    Ok(())
}

/// Delete the state file
pub fn delete_state() -> Result<()> {
    let state_path = get_state_path()?;

    if state_path.exists() {
        std::fs::remove_file(&state_path)
            .with_context(|| format!("Failed to delete state file: {}", state_path.display()))?;
    }

    Ok(())
}

/// Compute hash of configuration for change detection
pub fn compute_config_hash(config: &Config) -> Result<String> {
    // Serialize config to stable JSON representation
    let json = serde_json::to_string(config)
        .context("Failed to serialize config for hashing")?;

    // Compute SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let result = hasher.finalize();

    // Return hex-encoded hash
    Ok(format!("sha256:{}", hex::encode(&result)))
}

/// Create a new state from config
pub fn create_state(config: &Config, applied_policies: AppliedPolicies) -> Result<State> {
    Ok(State {
        version: STATE_VERSION.to_string(),
        config_hash: compute_config_hash(config)?,
        last_updated: Utc::now(),
        applied_policies,
    })
}

// Helper module for hex encoding
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::Browser;
    use crate::config::{BrowserIdMap, ExtensionEntry, PolicyEntry};
    use std::collections::HashMap;

    // Test fixtures

    fn make_test_config() -> Config {
        Config {
            policies: vec![PolicyEntry {
                name: "Test Policy".to_string(),
                browsers: vec![Browser::Chrome],
                disable_private_mode: Some(true),
                disable_guest_mode: None,
                extensions: vec![ExtensionEntry {
                    name: "Test".to_string(),
                    id: BrowserIdMap::Single("test123".to_string()),
                    force_installed: Some(true),
                    settings: HashMap::new(),
                }],
            }],
        }
    }

    fn make_test_browser_state() -> BrowserState {
        BrowserState {
            extensions: vec!["extension1".to_string(), "extension2".to_string()],
            disable_incognito: Some(true),
            disable_inprivate: None,
            disable_private_browsing: None,
            disable_guest_mode: Some(false),
        }
    }

    fn make_test_applied_policies() -> AppliedPolicies {
        AppliedPolicies {
            chrome: Some(make_test_browser_state()),
            firefox: None,
            edge: None,
        }
    }

    // BrowserState Tests

    #[test]
    fn browser_state_new_creates_empty_state() {
        let state = BrowserState::new();
        assert!(state.extensions.is_empty());
        assert!(state.disable_incognito.is_none());
        assert!(state.disable_inprivate.is_none());
        assert!(state.disable_private_browsing.is_none());
        assert!(state.disable_guest_mode.is_none());
    }

    #[test]
    fn browser_state_is_empty_returns_true_for_new_state() {
        let state = BrowserState::new();
        assert!(state.is_empty());
    }

    #[test]
    fn browser_state_is_empty_returns_false_with_extensions() {
        let mut state = BrowserState::new();
        state.extensions.push("test".to_string());
        assert!(!state.is_empty());
    }

    #[test]
    fn browser_state_is_empty_returns_false_with_privacy_settings() {
        let mut state = BrowserState::new();
        state.disable_incognito = Some(true);
        assert!(!state.is_empty());
    }

    #[test]
    fn browser_state_default_creates_empty_state() {
        let state = BrowserState::default();
        assert!(state.is_empty());
    }

    // AppliedPolicies Tests

    #[test]
    fn applied_policies_default_creates_empty() {
        let policies = AppliedPolicies::default();
        assert!(policies.chrome.is_none());
        assert!(policies.firefox.is_none());
        assert!(policies.edge.is_none());
    }

    // Config Hashing Tests

    #[test]
    fn compute_config_hash_returns_sha256_prefixed_hash() {
        let config = make_test_config();
        let hash = compute_config_hash(&config).unwrap();
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn compute_config_hash_returns_correct_length() {
        let config = make_test_config();
        let hash = compute_config_hash(&config).unwrap();
        // "sha256:" (7 chars) + 64 hex chars
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn compute_config_hash_is_deterministic() {
        let config = make_test_config();
        let hash1 = compute_config_hash(&config).unwrap();
        let hash2 = compute_config_hash(&config).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn compute_config_hash_differs_for_different_configs() {
        let config1 = make_test_config();

        let mut config2 = make_test_config();
        config2.policies[0].disable_guest_mode = Some(true);

        let hash1 = compute_config_hash(&config1).unwrap();
        let hash2 = compute_config_hash(&config2).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn compute_config_hash_handles_empty_config() {
        let config = Config {
            policies: vec![PolicyEntry {
                name: "Empty Policy".to_string(),
                browsers: vec![Browser::Chrome],
                disable_private_mode: None,
                disable_guest_mode: None,
                extensions: vec![],
            }],
        };

        let hash = compute_config_hash(&config).unwrap();
        assert!(hash.starts_with("sha256:"));
    }

    // State Creation Tests

    #[test]
    fn create_state_sets_correct_version() {
        let config = make_test_config();
        let policies = make_test_applied_policies();

        let state = create_state(&config, policies).unwrap();
        assert_eq!(state.version, STATE_VERSION);
    }

    #[test]
    fn create_state_computes_config_hash() {
        let config = make_test_config();
        let policies = make_test_applied_policies();

        let state = create_state(&config, policies).unwrap();
        assert!(state.config_hash.starts_with("sha256:"));
    }

    #[test]
    fn create_state_sets_last_updated() {
        let config = make_test_config();
        let policies = make_test_applied_policies();

        let before = Utc::now();
        let state = create_state(&config, policies).unwrap();
        let after = Utc::now();

        assert!(state.last_updated >= before);
        assert!(state.last_updated <= after);
    }

    #[test]
    fn create_state_preserves_applied_policies() {
        let config = make_test_config();
        let policies = make_test_applied_policies();
        let original_chrome = policies.chrome.as_ref().unwrap().clone();

        let state = create_state(&config, policies).unwrap();

        assert!(state.applied_policies.chrome.is_some());
        let chrome = state.applied_policies.chrome.as_ref().unwrap();
        assert_eq!(chrome.extensions, original_chrome.extensions);
    }

    // State Persistence Tests (using temp directories)

    #[test]
    fn save_and_load_state_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_path = temp_dir.path().join("state.json");

        let config = make_test_config();
        let policies = make_test_applied_policies();
        let original_state = create_state(&config, policies).unwrap();

        // Test serialization manually
        let json = serde_json::to_string_pretty(&original_state).unwrap();
        std::fs::write(&state_path, json).unwrap();

        let content = std::fs::read_to_string(&state_path).unwrap();
        let loaded_state: State = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded_state.version, original_state.version);
        assert_eq!(loaded_state.config_hash, original_state.config_hash);
    }

    #[test]
    fn state_serialization_includes_all_fields() {
        let config = make_test_config();
        let policies = make_test_applied_policies();
        let state = create_state(&config, policies).unwrap();

        let json = serde_json::to_string(&state).unwrap();

        assert!(json.contains("version"));
        assert!(json.contains("config_hash"));
        assert!(json.contains("last_updated"));
        assert!(json.contains("applied_policies"));
    }

    #[test]
    fn state_deserialization_handles_version_field() {
        let json = r#"{
            "version": "1.0",
            "config_hash": "sha256:test",
            "last_updated": "2025-01-01T00:00:00Z",
            "applied_policies": {}
        }"#;

        let state: State = serde_json::from_str(json).unwrap();
        assert_eq!(state.version, "1.0");
    }

    #[test]
    fn browser_state_serialization_skips_none_values() {
        let state = BrowserState {
            extensions: vec!["ext1".to_string()],
            disable_incognito: Some(true),
            disable_inprivate: None,
            disable_private_browsing: None,
            disable_guest_mode: None,
        };

        let json = serde_json::to_string(&state).unwrap();

        assert!(json.contains("extensions"));
        assert!(json.contains("disable_incognito"));
        assert!(!json.contains("disable_inprivate"));
        assert!(!json.contains("disable_private_browsing"));
        assert!(!json.contains("disable_guest_mode"));
    }

    // Hex Encoding Tests

    #[test]
    fn hex_encode_empty_bytes() {
        let bytes: &[u8] = &[];
        let encoded = hex::encode(bytes);
        assert_eq!(encoded, "");
    }

    #[test]
    fn hex_encode_single_byte() {
        let bytes: &[u8] = &[0xff];
        let encoded = hex::encode(bytes);
        assert_eq!(encoded, "ff");
    }

    #[test]
    fn hex_encode_multiple_bytes() {
        let bytes: &[u8] = &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
        let encoded = hex::encode(bytes);
        assert_eq!(encoded, "0123456789abcdef");
    }

    #[test]
    fn hex_encode_pads_single_digit() {
        let bytes: &[u8] = &[0x00, 0x01, 0x0f];
        let encoded = hex::encode(bytes);
        assert_eq!(encoded, "00010f");
    }

    // Integration-style tests

    #[test]
    fn state_with_all_browsers_roundtrips_correctly() {
        let config = Config {
            policies: vec![PolicyEntry {
                name: "Multi-browser Policy".to_string(),
                browsers: vec![Browser::Chrome, Browser::Firefox, Browser::Edge],
                disable_private_mode: Some(true),
                disable_guest_mode: Some(false),
                extensions: vec![],
            }],
        };

        let policies = AppliedPolicies {
            chrome: Some(BrowserState {
                extensions: vec![],
                disable_incognito: Some(true),
                disable_inprivate: None,
                disable_private_browsing: None,
                disable_guest_mode: None,
            }),
            firefox: Some(BrowserState {
                extensions: vec![],
                disable_incognito: None,
                disable_inprivate: None,
                disable_private_browsing: Some(true),
                disable_guest_mode: None,
            }),
            edge: Some(BrowserState {
                extensions: vec![],
                disable_incognito: None,
                disable_inprivate: Some(true),
                disable_private_browsing: None,
                disable_guest_mode: Some(false),
            }),
        };

        let state = create_state(&config, policies).unwrap();
        let json = serde_json::to_string(&state).unwrap();
        let loaded: State = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, state.version);
        assert_eq!(loaded.config_hash, state.config_hash);
    }
}
