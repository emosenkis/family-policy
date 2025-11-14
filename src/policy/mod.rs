use anyhow::{Context, Result};

use crate::config::Config;
use crate::state::{AppliedPolicies, State};

mod chromium_common;
pub mod chrome;
pub mod edge;
pub mod firefox;

/// Apply policies for all configured browsers
pub fn apply_policies(config: &Config, _current_state: Option<&State>) -> Result<AppliedPolicies> {
    let mut applied = AppliedPolicies::default();

    // Convert new config format to browser-specific configs
    let (chrome_config, firefox_config, edge_config) = crate::config::to_browser_configs(config);

    // Apply Chrome policies
    if let Some(chrome_config) = chrome_config {
        println!("Applying Chrome policies...");
        let state = chrome::apply_chrome_policies(&chrome_config)
            .context("Failed to apply Chrome policies")?;

        if !state.is_empty() {
            applied.chrome = Some(state);
            println!("✓ Chrome policies applied successfully");
        }
    }

    // Apply Firefox policies
    if let Some(firefox_config) = firefox_config {
        println!("Applying Firefox policies...");
        let state = firefox::apply_firefox_policies(&firefox_config)
            .context("Failed to apply Firefox policies")?;

        if !state.is_empty() {
            applied.firefox = Some(state);
            println!("✓ Firefox policies applied successfully");
        }
    }

    // Apply Edge policies
    if let Some(edge_config) = edge_config {
        println!("Applying Edge policies...");
        let state = edge::apply_edge_policies(&edge_config)
            .context("Failed to apply Edge policies")?;

        if !state.is_empty() {
            applied.edge = Some(state);
            println!("✓ Edge policies applied successfully");
        }
    }

    Ok(applied)
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
    fn test_apply_policies_empty_config() {
        let config = Config {
            policies: vec![],
        };

        // This should fail because at least one policy must be configured
        assert!(crate::config::validate_config(&config).is_err());
    }
}
