// Re-export State and related functionality from the main state module
// Agent mode and local mode now use the same unified state type

pub use crate::state::State;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{BrowserState, AppliedPolicies};
    use chrono::Utc;

    #[test]
    fn agent_state_new_creates_valid_state() {
        let state = State::new_agent();
        assert_eq!(state.version, "1.0");
        assert!(!state.machine_id.is_empty());
        assert!(state.last_checked.is_none());
        assert!(state.etag.is_none());
    }

    #[test]
    fn agent_state_update_checked_sets_timestamp() {
        let mut state = State::new_agent();
        let before = Utc::now();

        state.update_checked();

        let after = Utc::now();
        assert!(state.last_checked.is_some());
        let checked = state.last_checked.unwrap();
        assert!(checked >= before && checked <= after);
    }

    #[test]
    fn agent_state_update_applied_sets_all_fields() {
        let mut state = State::new_agent();

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

        assert_eq!(state.config_hash, hash);
        assert_eq!(state.etag, etag);
        assert!(state.last_checked.is_some());

        let updated = state.last_updated;
        assert!(updated >= before && updated <= after);
    }

    #[test]
    fn agent_state_update_etag_updates_only_etag_and_checked() {
        let mut state = State::new_agent();
        let original_hash = "sha256:original".to_string();
        state.config_hash = original_hash.clone();

        state.update_etag(Some("W/\"new\"".to_string()));

        assert_eq!(state.config_hash, original_hash);
        assert_eq!(state.etag, Some("W/\"new\"".to_string()));
        assert!(state.last_checked.is_some());
    }
}
