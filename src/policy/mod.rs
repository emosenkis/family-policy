use anyhow::{Context, Result};

use crate::config::Config;
use crate::state::{AppliedPolicies, State};

pub mod chrome;
mod chromium_common;
pub mod edge;
pub mod firefox;

/// A failed batch still reports successful browsers for cleanup and retry tracking.
#[derive(Debug)]
pub struct PolicyApplyError {
    pub applied: AppliedPolicies,
    errors: Vec<String>,
}
impl std::fmt::Display for PolicyApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Some browser policies failed:\n{}",
            self.errors.join("\n")
        )
    }
}
impl std::error::Error for PolicyApplyError {}

fn apply_browser(
    name: &str,
    apply: impl FnOnce() -> Result<crate::state::BrowserState>,
    slot: &mut Option<crate::state::BrowserState>,
    errors: &mut Vec<String>,
) {
    match apply() {
        Ok(state) => {
            *slot = Some(state);
            println!("✓ {name} policy operation succeeded");
        }
        Err(error) => {
            let message = format!("{name}: {error:#}");
            eprintln!("✗ {message}");
            errors.push(message);
        }
    }
}

/// Attempt every configured browser, even if an earlier one fails.
pub fn apply_policies(
    config: &Config,
    _current_state: Option<&State>,
    dry_run: bool,
) -> Result<AppliedPolicies> {
    let mut applied = AppliedPolicies::default();
    let mut errors = Vec::new();
    let (chrome, firefox, edge) = crate::config::to_browser_configs(config);
    if let Some(config) = chrome {
        apply_browser(
            "Chrome",
            || chrome::apply_chrome_policies(&config, dry_run),
            &mut applied.chrome,
            &mut errors,
        );
    }
    if let Some(config) = firefox {
        apply_browser(
            "Firefox",
            || firefox::apply_firefox_policies(&config, dry_run),
            &mut applied.firefox,
            &mut errors,
        );
    }
    if let Some(config) = edge {
        apply_browser(
            "Edge",
            || edge::apply_edge_policies(&config, dry_run),
            &mut applied.edge,
            &mut errors,
        );
    }
    if !errors.is_empty() {
        return Err(PolicyApplyError { applied, errors }.into());
    }
    Ok(applied)
}

/// Persist successes without a hash or ETag that could suppress a retry.
pub fn persist_partial_failure(
    error: &anyhow::Error,
    current: Option<&State>,
    dry_run: bool,
) -> Result<()> {
    if !dry_run {
        if let Some(failure) = error.downcast_ref::<PolicyApplyError>() {
            let mut state = current.cloned().unwrap_or_else(State::new_agent);
            state.mark_partial_application(&failure.applied);
            crate::state::save_state(&state).with_context(|| {
                format!("{error:#}; additionally failed to save partial application state")
            })?;
        }
    }
    Ok(())
}

/// Remove all policies for browsers tracked in the state
pub fn remove_policies(state: &State) -> Result<()> {
    let mut any_errors = false;

    // Remove Chrome policies
    if state.applied_policies.chrome.is_some() {
        println!("Removing Chrome policies...");
        match chrome::remove_chrome_policies() {
            Ok(_) => println!("✓ Chrome policies removed successfully"),
            Err(e) => {
                eprintln!("✗ Failed to remove Chrome policies: {:#}", e);
                any_errors = true;
            }
        }
    }

    // Remove Firefox policies
    if state.applied_policies.firefox.is_some() {
        println!("Removing Firefox policies...");
        match firefox::remove_firefox_policies() {
            Ok(_) => println!("✓ Firefox policies removed successfully"),
            Err(e) => {
                eprintln!("✗ Failed to remove Firefox policies: {:#}", e);
                any_errors = true;
            }
        }
    }

    // Remove Edge policies
    if state.applied_policies.edge.is_some() {
        println!("Removing Edge policies...");
        match edge::remove_edge_policies() {
            Ok(_) => println!("✓ Edge policies removed successfully"),
            Err(e) => {
                eprintln!("✗ Failed to remove Edge policies: {:#}", e);
                any_errors = true;
            }
        }
    }

    if any_errors {
        anyhow::bail!("Some policies could not be removed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_reports_all_failures_and_tracks_only_successes() {
        let mut applied = AppliedPolicies::default();
        let mut errors = Vec::new();
        apply_browser(
            "Chrome",
            || anyhow::bail!("import failed"),
            &mut applied.chrome,
            &mut errors,
        );
        apply_browser(
            "Firefox",
            || Ok(crate::state::BrowserState::new()),
            &mut applied.firefox,
            &mut errors,
        );
        apply_browser(
            "Edge",
            || anyhow::bail!("refresh failed"),
            &mut applied.edge,
            &mut errors,
        );
        assert!(applied.chrome.is_none());
        assert!(applied.firefox.is_some());
        assert!(applied.edge.is_none());
        let error = PolicyApplyError { applied, errors };
        assert!(error.to_string().contains("Chrome: import failed"));
        assert!(error.to_string().contains("Edge: refresh failed"));
    }

    #[test]
    fn test_apply_policies_empty_config() {
        let config = Config { policies: vec![] };

        // This should fail because at least one policy must be configured
        assert!(crate::config::validate_config(&config).is_err());
    }
}
