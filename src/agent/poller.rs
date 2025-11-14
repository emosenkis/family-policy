use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use std::time::Duration;

use super::config::GitHubConfig;

/// Result of fetching policy from GitHub
#[derive(Debug)]
pub enum PolicyFetchResult {
    /// Content hasn't changed (304 Not Modified)
    NotModified,
    /// Content was updated
    Updated {
        content: String,
        etag: Option<String>,
    },
}

/// GitHub poller with ETag support
pub struct GitHubPoller {
    client: Client,
    config: GitHubConfig,
}

impl GitHubPoller {
    /// Create a new GitHub poller
    pub fn new(config: GitHubConfig) -> Result<Self> {
        // Validate HTTPS
        let url = url::Url::parse(&config.policy_url)
            .context("Invalid policy URL")?;

        if url.scheme() != "https" {
            anyhow::bail!("Policy URL must use HTTPS for security (got: {})", url.scheme());
        }

        // Build HTTP client with rustls (HTTPS only)
        let client = Client::builder()
            .user_agent(format!("family-policy-agent/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .https_only(true) // Enforce HTTPS
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    /// Fetch policy from GitHub with ETag support
    ///
    /// # Arguments
    /// * `etag` - Optional ETag from previous request for conditional GET
    ///
    /// # Returns
    /// * `PolicyFetchResult::NotModified` if content unchanged (304)
    /// * `PolicyFetchResult::Updated` with new content and ETag if changed
    pub async fn fetch_policy(&self, etag: Option<&str>) -> Result<PolicyFetchResult> {
        tracing::debug!("Fetching policy from: {}", self.config.policy_url);

        let mut request = self.client.get(&self.config.policy_url);

        // Add authentication if configured
        if let Some(token) = &self.config.access_token {
            request = request.header("Authorization", format!("token {}", token));
        }

        // Add ETag for conditional request (saves bandwidth)
        if let Some(etag) = etag {
            tracing::debug!("Using ETag for conditional request: {}", etag);
            request = request.header("If-None-Match", etag);
        }

        // Send request
        let response = request.send().await
            .context("Failed to connect to GitHub")?;

        match response.status() {
            StatusCode::NOT_MODIFIED => {
                // Content hasn't changed
                tracing::info!("Policy unchanged (304 Not Modified)");
                Ok(PolicyFetchResult::NotModified)
            }
            StatusCode::OK => {
                // Content updated
                let new_etag = response
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);

                if let Some(ref etag) = new_etag {
                    tracing::debug!("New ETag: {}", etag);
                }

                let content = response.text().await
                    .context("Failed to read response body")?;

                tracing::info!("Policy downloaded ({} bytes)", content.len());

                Ok(PolicyFetchResult::Updated {
                    content,
                    etag: new_etag,
                })
            }
            StatusCode::NOT_FOUND => {
                anyhow::bail!(
                    "Policy file not found (404). Check URL and repository access.\nURL: {}",
                    self.config.policy_url
                )
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                anyhow::bail!(
                    "Access denied ({}). Check access token and repository permissions.\nURL: {}",
                    response.status(),
                    self.config.policy_url
                )
            }
            status => {
                anyhow::bail!("GitHub returned unexpected status: {} for URL: {}", status, self.config.policy_url)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_poller_rejects_http() {
        let config = GitHubConfig {
            policy_url: "http://example.com/policy.yaml".to_string(),
            access_token: None,
        };

        assert!(GitHubPoller::new(config).is_err());
    }

    #[test]
    fn github_poller_accepts_https() {
        let config = GitHubConfig {
            policy_url: "https://raw.githubusercontent.com/user/repo/main/policy.yaml".to_string(),
            access_token: None,
        };

        assert!(GitHubPoller::new(config).is_ok());
    }

    #[test]
    fn github_poller_validates_url() {
        let config = GitHubConfig {
            policy_url: "not-a-url".to_string(),
            access_token: None,
        };

        assert!(GitHubPoller::new(config).is_err());
    }
}
