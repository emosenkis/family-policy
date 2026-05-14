use crate::config::Config;
use crate::policy;
use crate::state::{
    BrowserState, compute_config_hash, create_state, delete_state, load_state, save_state,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Result of applying policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    /// Whether policies were actually changed
    pub changed: bool,
    /// Number of extensions applied per browser
    pub extensions_applied: BrowserCounts,
    /// Privacy settings applied per browser
    pub privacy_settings_applied: BrowserCounts,
    /// Errors encountered (if any)
    pub errors: Vec<String>,
    /// Warnings (if any)
    pub warnings: Vec<String>,
}

/// Result of removing policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovalResult {
    /// Number of extensions removed per browser
    pub extensions_removed: BrowserCounts,
    /// Number of privacy settings removed per browser
    pub privacy_settings_removed: BrowserCounts,
    /// Errors encountered (if any)
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserCounts {
    pub chrome: usize,
    pub firefox: usize,
    pub edge: usize,
}

/// Options that control how policy application runs.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyOptions {
    /// If true, only show what would be done.
    pub dry_run: bool,
    /// If true, rewrite policies even when the saved state hash matches.
    pub force: bool,
}

/// Apply policies from a configuration
///
/// # Arguments
/// * `config` - Policy configuration to apply
/// * `dry_run` - If true, only show what would be done
///
/// # Returns
/// * `ApplyResult` with details of what was applied
pub fn apply_policies_from_config(config: &Config, dry_run: bool) -> Result<ApplyResult> {
    apply_policies_from_config_with_options(
        config,
        ApplyOptions {
            dry_run,
            force: false,
        },
    )
}

/// Apply policies from a configuration using explicit application options.
pub fn apply_policies_from_config_with_options(
    config: &Config,
    options: ApplyOptions,
) -> Result<ApplyResult> {
    // Compute hash of new config
    let config_hash = compute_config_hash(config)?;
    tracing::debug!("Computed configuration hash: {}", config_hash);

    match crate::state::get_state_path() {
        Ok(path) => tracing::debug!("State file path: {}", path.display()),
        Err(e) => tracing::debug!("Could not resolve state file path: {}", e),
    }

    // Load current state
    let current_state = load_state().ok().flatten();
    match &current_state {
        Some(state) => tracing::debug!(
            "Loaded existing state: state_hash={}, state_version={}, applied_chrome={}, applied_firefox={}, applied_edge={}",
            state.config_hash,
            state.version,
            state.applied_policies.chrome.is_some(),
            state.applied_policies.firefox.is_some(),
            state.applied_policies.edge.is_some()
        ),
        None => tracing::debug!("No existing state file found; policies will be written"),
    }

    // Check if config has changed
    let changed = current_state
        .as_ref()
        .map(|s| s.config_hash != config_hash)
        .unwrap_or(true);
    tracing::debug!(
        "Policy change detection result: changed={}, dry_run={}, force={}",
        changed,
        options.dry_run,
        options.force
    );

    if !changed && !options.dry_run && !options.force {
        tracing::debug!(
            "Skipping policy writes because configuration hash matches saved state. Use --force-apply, remove the state file, or change the config to force a rewrite."
        );
        return Ok(ApplyResult {
            changed: false,
            extensions_applied: BrowserCounts::default(),
            privacy_settings_applied: BrowserCounts::default(),
            errors: vec![],
            warnings: vec!["No changes detected, policies already applied".to_string()],
        });
    }

    let mut result = ApplyResult {
        changed: changed || options.force,
        extensions_applied: BrowserCounts::default(),
        privacy_settings_applied: BrowserCounts::default(),
        errors: vec![],
        warnings: vec![],
    };

    // Apply policies using existing policy module
    tracing::debug!("Invoking platform policy writers");
    if options.force && !changed {
        tracing::debug!(
            "Force apply requested while configuration hash matches saved state; rewriting policy outputs anyway"
        );
    }

    let applied_policies = policy::apply_policies(config, current_state.as_ref(), options.dry_run)
        .context("Failed to apply policies")?;

    // Count what was applied
    if let Some(ref chrome) = applied_policies.chrome {
        result.extensions_applied.chrome = chrome.extensions.len();
        result.privacy_settings_applied.chrome = count_privacy_in_state(chrome);
    }
    if let Some(ref firefox) = applied_policies.firefox {
        result.extensions_applied.firefox = firefox.extensions.len();
        result.privacy_settings_applied.firefox = count_privacy_in_state(firefox);
    }
    if let Some(ref edge) = applied_policies.edge {
        result.extensions_applied.edge = edge.extensions.len();
        result.privacy_settings_applied.edge = count_privacy_in_state(edge);
    }

    if options.dry_run {
        return Ok(result);
    }

    // Save new state
    let new_state = create_state(config, applied_policies).context("Failed to create state")?;

    tracing::debug!("Saving applied state after successful policy writes");
    save_state(&new_state).context("Failed to save state")?;

    Ok(result)
}

/// Remove all applied policies
///
/// # Arguments
/// * `dry_run` - If true, only show what would be removed
///
/// # Returns
/// * `RemovalResult` with details of what was removed
pub fn remove_all_policies(dry_run: bool) -> Result<RemovalResult> {
    let current_state = load_state()
        .context("Failed to load state")?
        .ok_or_else(|| anyhow::anyhow!("No state file found, nothing to remove"))?;

    let mut result = RemovalResult {
        extensions_removed: BrowserCounts::default(),
        privacy_settings_removed: BrowserCounts::default(),
        errors: vec![],
    };

    // Count what will be removed
    if let Some(ref chrome) = current_state.applied_policies.chrome {
        result.extensions_removed.chrome = chrome.extensions.len();
        result.privacy_settings_removed.chrome = count_privacy_in_state(chrome);
    }
    if let Some(ref firefox) = current_state.applied_policies.firefox {
        result.extensions_removed.firefox = firefox.extensions.len();
        result.privacy_settings_removed.firefox = count_privacy_in_state(firefox);
    }
    if let Some(ref edge) = current_state.applied_policies.edge {
        result.extensions_removed.edge = edge.extensions.len();
        result.privacy_settings_removed.edge = count_privacy_in_state(edge);
    }

    if dry_run {
        return Ok(result);
    }

    // Actually remove policies using existing policy module
    if let Err(e) = policy::remove_policies(&current_state) {
        result
            .errors
            .push(format!("Failed to remove policies: {}", e));
    }

    // Remove state file
    if let Err(e) = delete_state() {
        result
            .errors
            .push(format!("Failed to delete state file: {}", e));
    }

    if !result.errors.is_empty() {
        anyhow::bail!("Some errors occurred during removal: {:?}", result.errors);
    }

    Ok(result)
}

// Helper functions

fn count_privacy_in_state(state: &BrowserState) -> usize {
    let mut count = 0;
    if state.disable_incognito.is_some() {
        count += 1;
    }
    if state.disable_inprivate.is_some() {
        count += 1;
    }
    if state.disable_private_browsing.is_some() {
        count += 1;
    }
    if state.disable_guest_mode.is_some() {
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::Browser;
    use crate::config::{BrowserIdMap, ExtensionEntry, PolicyEntry};

    #[test]
    fn test_browser_counts_default() {
        let counts = BrowserCounts::default();
        assert_eq!(counts.chrome, 0);
        assert_eq!(counts.firefox, 0);
        assert_eq!(counts.edge, 0);
    }

    #[test]
    fn test_apply_result_serialization() {
        let result = ApplyResult {
            changed: true,
            extensions_applied: BrowserCounts {
                chrome: 2,
                firefox: 1,
                edge: 3,
            },
            privacy_settings_applied: BrowserCounts {
                chrome: 1,
                firefox: 1,
                edge: 2,
            },
            errors: vec![],
            warnings: vec!["Test warning".to_string()],
        };

        // Test that it can be serialized and deserialized
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ApplyResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.changed, true);
        assert_eq!(deserialized.extensions_applied.chrome, 2);
        assert_eq!(deserialized.warnings.len(), 1);
    }

    #[test]
    fn test_removal_result_serialization() {
        let result = RemovalResult {
            extensions_removed: BrowserCounts::default(),
            privacy_settings_removed: BrowserCounts::default(),
            errors: vec![],
        };

        // Test that it can be serialized and deserialized
        let json = serde_json::to_string(&result).unwrap();
        let _deserialized: RemovalResult = serde_json::from_str(&json).unwrap();
    }
}
