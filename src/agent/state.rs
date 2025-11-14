use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::state::{AppliedPolicies, get_state_path};

/// Agent-specific state extending the base state
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentState {
    /// State version
    pub version: String,

    /// Unique identifier for this machine
    pub machine_id: String,

    /// Hash of currently applied configuration
    pub config_hash: Option<String>,

    /// Last time configuration was updated/applied
    pub last_updated: Option<DateTime<Utc>>,

    /// Last time agent checked for updates
    pub last_checked: Option<DateTime<Utc>>,

    /// GitHub ETag from last request (for efficient HTTP caching)
    pub github_etag: Option<String>,

    /// Applied policies (from base state)
    #[serde(default)]
    pub applied_policies: AppliedPolicies,
}

impl AgentState {
    /// Create a new agent state
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            machine_id: Uuid::new_v4().to_string(),
            config_hash: None,
            last_updated: None,
            last_checked: None,
            github_etag: None,
            applied_policies: AppliedPolicies::default(),
        }
    }

    /// Load agent state from file
    pub fn load() -> Result<Option<Self>> {
        let state_path = get_agent_state_path()?;

        if !state_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&state_path)
            .with_context(|| format!("Failed to read state file: {}", state_path.display()))?;

        let state: AgentState = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse state file: {}", state_path.display()))?;

        // Validate state version
        if state.version != "1.0" {
            eprintln!(
                "Warning: State file version mismatch (expected 1.0, got {}). Treating as new state.",
                state.version
            );
            return Ok(None);
        }

        Ok(Some(state))
    }

    /// Save agent state to file
    pub fn save(&self) -> Result<()> {
        let state_path = get_agent_state_path()?;

        // Ensure parent directory exists
        if let Some(parent) = state_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create state directory: {}", parent.display())
            })?;
        }

        // Serialize state to JSON
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize state")?;

        // Write atomically
        crate::platform::common::atomic_write(&state_path, content.as_bytes())
            .with_context(|| format!("Failed to write state file: {}", state_path.display()))?;

        Ok(())
    }

    /// Update state after checking for policy
    pub fn update_checked(&mut self) {
        self.last_checked = Some(Utc::now());
    }

    /// Update state after applying policy
    pub fn update_applied(&mut self, config_hash: String, etag: Option<String>, applied_policies: AppliedPolicies) {
        self.config_hash = Some(config_hash);
        self.last_updated = Some(Utc::now());
        self.last_checked = Some(Utc::now());
        self.github_etag = etag;
        self.applied_policies = applied_policies;
    }

    /// Update ETag without applying policy
    pub fn update_etag(&mut self, etag: Option<String>) {
        self.github_etag = etag;
        self.last_checked = Some(Utc::now());
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the platform-specific agent state file path
pub fn get_agent_state_path() -> Result<PathBuf> {
    // Use the same path as regular state
    get_state_path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BrowserState;

    #[test]
    fn agent_state_new_creates_valid_state() {
        let state = AgentState::new();
        assert_eq!(state.version, "1.0");
        assert!(!state.machine_id.is_empty());
        assert!(state.config_hash.is_none());
        assert!(state.last_updated.is_none());
        assert!(state.last_checked.is_none());
        assert!(state.github_etag.is_none());
    }

    #[test]
    fn agent_state_update_checked_sets_timestamp() {
        let mut state = AgentState::new();
        let before = Utc::now();

        state.update_checked();

        let after = Utc::now();
        assert!(state.last_checked.is_some());
        let checked = state.last_checked.unwrap();
        assert!(checked >= before && checked <= after);
    }

    #[test]
    fn agent_state_update_applied_sets_all_fields() {
        let mut state = AgentState::new();

        let hash = "sha256:test".to_string();
        let etag = Some("W/\"abc123\"".to_string());
        let policies = AppliedPolicies {
            chrome: Some(BrowserState::new()),
            firefox: None,
            edge: None,
        };

        let before = Utc::now();
        state.update_applied(hash.clone(), etag.clone(), policies);
        let after = Utc::now();

        assert_eq!(state.config_hash, Some(hash));
        assert_eq!(state.github_etag, etag);
        assert!(state.last_updated.is_some());
        assert!(state.last_checked.is_some());

        let updated = state.last_updated.unwrap();
        assert!(updated >= before && updated <= after);
    }

    #[test]
    fn agent_state_update_etag_updates_only_etag_and_checked() {
        let mut state = AgentState::new();
        let original_hash = Some("sha256:original".to_string());
        state.config_hash = original_hash.clone();

        state.update_etag(Some("W/\"new\"".to_string()));

        assert_eq!(state.config_hash, original_hash);
        assert_eq!(state.github_etag, Some("W/\"new\"".to_string()));
        assert!(state.last_checked.is_some());
        assert!(state.last_updated.is_none());
    }

    #[test]
    fn agent_state_serialization_roundtrip() {
        let state = AgentState {
            version: "1.0".to_string(),
            machine_id: "test-machine".to_string(),
            config_hash: Some("sha256:test".to_string()),
            last_updated: Some(Utc::now()),
            last_checked: Some(Utc::now()),
            github_etag: Some("W/\"abc\"".to_string()),
            applied_policies: AppliedPolicies::default(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let loaded: AgentState = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, state.version);
        assert_eq!(loaded.machine_id, state.machine_id);
        assert_eq!(loaded.config_hash, state.config_hash);
        assert_eq!(loaded.github_etag, state.github_etag);
    }
}
