use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::time::sleep;

use super::{AgentConfig, AgentState, GitHubPoller, PolicyFetchResult, PollingScheduler};
use crate::config;
use crate::policy;
use crate::state::AppliedPolicies;

/// Run the agent daemon in a loop
pub async fn run_agent_daemon(config: AgentConfig) -> Result<()> {
    tracing::info!("Starting agent daemon");
    tracing::info!("Policy URL: {}", config.github.policy_url);
    tracing::info!(
        "Poll interval: {} seconds (±{} seconds jitter)",
        config.agent.poll_interval,
        config.agent.poll_jitter
    );

    let scheduler = PollingScheduler::new(config.agent.poll_interval, config.agent.poll_jitter);

    loop {
        // Check and apply policy
        match check_and_apply_with_retry(&config).await {
            Ok(applied) => {
                if applied {
                    tracing::info!("Policy updated and applied successfully");
                } else {
                    tracing::debug!("Policy unchanged");
                }
            }
            Err(e) => {
                tracing::error!("Failed to check/apply policy: {:#}", e);
                // Continue running even if this check failed
            }
        }

        // Sleep until next check
        let next_check = scheduler.next_poll_time();
        tracing::debug!("Next check at: {}", next_check.format("%Y-%m-%d %H:%M:%S %Z"));
        scheduler.sleep_until_next_poll().await;
    }
}

/// Check for policy updates and apply if changed (single execution)
pub async fn check_and_apply_once(config: &AgentConfig) -> Result<bool> {
    check_and_apply_policy(config).await
}

/// Check and apply policy with retry logic
async fn check_and_apply_with_retry(config: &AgentConfig) -> Result<bool> {
    let max_retries = config.agent.max_retries;
    let mut retries = 0;

    loop {
        match check_and_apply_policy(config).await {
            Ok(applied) => return Ok(applied),
            Err(e) if retries < max_retries => {
                retries += 1;
                let backoff = Duration::from_secs(config.agent.retry_interval * (2_u64.pow(retries - 1)));

                tracing::warn!(
                    "Failed to check/apply policy (attempt {}/{}): {}",
                    retries,
                    max_retries,
                    e
                );
                tracing::info!("Retrying in {} seconds...", backoff.as_secs());

                sleep(backoff).await;
            }
            Err(e) => {
                tracing::error!("Failed to check/apply policy after {} retries", retries);
                return Err(e);
            }
        }
    }
}

/// Check for policy updates and apply if changed
async fn check_and_apply_policy(config: &AgentConfig) -> Result<bool> {
    // 1. Load current state
    let mut state = AgentState::load()?.unwrap_or_default();

    // 2. Create GitHub poller
    let poller = GitHubPoller::new(config.github.clone())?;

    // 3. Fetch policy with ETag
    let result = poller
        .fetch_policy(state.github_etag.as_deref())
        .await?;

    // 4. Handle result
    match result {
        PolicyFetchResult::NotModified => {
            // No change, just update check time
            state.update_checked();
            state.save().context("Failed to save state")?;
            Ok(false)
        }
        PolicyFetchResult::Updated { content, etag } => {
            // Content changed, check if policy actually changed
            let new_hash = compute_yaml_hash(&content);

            if state.config_hash.as_ref() == Some(&new_hash) {
                // Same content (hash collision or ETag issue), just update ETag
                tracing::debug!("Content downloaded but hash unchanged");
                state.update_etag(etag);
                state.save().context("Failed to save state")?;
                return Ok(false);
            }

            // Policy changed, apply it
            tracing::info!("New policy detected (hash: {})", &new_hash[..16]);

            // Parse policy
            let policy_config = config::Config::from_yaml_str(&content)
                .context("Failed to parse policy YAML")?;

            // Apply policies using existing logic
            let applied_policies = apply_policy_config(&policy_config)
                .context("Failed to apply policies")?;

            // Update state
            state.update_applied(new_hash, etag, applied_policies);
            state.save().context("Failed to save state")?;

            tracing::info!("Policy applied successfully");
            Ok(true)
        }
    }
}

/// Apply policy configuration using existing policy modules
fn apply_policy_config(config: &config::Config) -> Result<AppliedPolicies> {
    // Convert to browser-specific configs
    let (chrome_config, firefox_config, edge_config) = config::to_browser_configs(config);

    let mut applied = AppliedPolicies::default();

    // Apply Chrome policies
    if let Some(chrome) = chrome_config {
        tracing::info!("Applying Chrome policies...");
        let state = policy::chrome::apply_chrome_policies(&chrome, false)?;
        applied.chrome = Some(state);
    }

    // Apply Firefox policies
    if let Some(firefox) = firefox_config {
        tracing::info!("Applying Firefox policies...");
        let state = policy::firefox::apply_firefox_policies(&firefox, false)?;
        applied.firefox = Some(state);
    }

    // Apply Edge policies
    if let Some(edge) = edge_config {
        tracing::info!("Applying Edge policies...");
        let state = policy::edge::apply_edge_policies(&edge, false)?;
        applied.edge = Some(state);
    }

    Ok(applied)
}

/// Compute SHA-256 hash of YAML content
fn compute_yaml_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();

    format!("sha256:{}", hex::encode(&result))
}

// Helper module for hex encoding
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

// Extension trait to add from_yaml_str to Config
impl config::Config {
    /// Parse config from YAML string
    pub fn from_yaml_str(content: &str) -> Result<Self> {
        let config: config::Config = serde_yaml::from_str(content)
            .context("Failed to parse YAML")?;

        // Validate the config
        config::validate_config(&config)?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_yaml_hash_is_deterministic() {
        let yaml = "chrome:\n  extensions:\n    - id: test123";
        let hash1 = compute_yaml_hash(yaml);
        let hash2 = compute_yaml_hash(yaml);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn compute_yaml_hash_different_for_different_content() {
        let yaml1 = "chrome:\n  extensions:\n    - id: test123";
        let yaml2 = "chrome:\n  extensions:\n    - id: test456";
        let hash1 = compute_yaml_hash(yaml1);
        let hash2 = compute_yaml_hash(yaml2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn compute_yaml_hash_has_correct_format() {
        let yaml = "chrome:\n  extensions:\n    - id: test123";
        let hash = compute_yaml_hash(yaml);
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71); // "sha256:" (7) + 64 hex chars
    }
}
