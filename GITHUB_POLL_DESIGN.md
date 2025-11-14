# GitHub-Based Remote Management Design

## Overview

The simplest possible remote management: clients poll a GitHub repository for policy changes. No server, no complex auth, no firewall configuration needed.

```
┌─────────────────────────────────────────────────────────────┐
│                    GitHub Repository                         │
│                 (your-policies-repo)                         │
│                                                              │
│   policies/                                                  │
│     ├── kids-pc.yaml                                        │
│     ├── living-room-mac.yaml                                │
│     └── basement-linux.yaml                                 │
│                                                              │
└──────────────────┬──────────────────────────────────────────┘
                   │
      Every 5 minutes, check for changes
                   │
    ┌──────────────┼──────────────┬──────────────┐
    ↓              ↓              ↓              ↓
┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
│ Agent  │    │ Agent  │    │ Agent  │    │ Agent  │
│Kids-PC │    │Living  │    │Basement│    │Bedroom │
│        │    │Room Mac│    │ Linux  │    │Laptop  │
└────────┘    └────────────┘ └────────┘    └────────┘
```

**Admin workflow**:
1. Edit policy YAML in local git repo
2. Commit and push to GitHub
3. Agents automatically detect change and apply within minutes

---

## Architecture

### Single Binary, Two Modes

**Agent Mode** (runs on family computers):
- Polls GitHub repo for policy file
- Compares hash with last applied policy
- If changed, downloads and applies
- No listening port required!

**Setup Mode** (one-time configuration):
- Configures which GitHub repo/file to track
- Optionally configures authentication for private repos
- Generates unique machine ID

---

## Component 1: Agent Implementation

### Agent Configuration File

**Location**:
- Linux: `/etc/family-policy/agent.conf`
- macOS: `/Library/Application Support/family-policy/agent.conf`
- Windows: `C:\ProgramData\family-policy\agent.conf`

**Format** (TOML):
```toml
# GitHub repository settings
[github]
# Raw file URL to poll
policy_url = "https://raw.githubusercontent.com/username/family-policies/main/policies/kids-pc.yaml"

# For private repositories (optional)
# Create at: https://github.com/settings/tokens
# Needs 'repo' scope for private repos, or 'public_repo' for public repos
access_token = "ghp_xxxxxxxxxxxxxxxxxxxx"

# Polling interval
[agent]
# How often to check for changes (seconds)
poll_interval = 300  # 5 minutes

# Add random jitter to prevent thundering herd (seconds)
poll_jitter = 60  # ±60 seconds

# Retry on failure
retry_interval = 60  # 1 minute
max_retries = 3

# Logging
[logging]
level = "info"
file = "/var/log/family-policy-agent.log"

# Optional: File signature verification (advanced)
[security]
# Verify GPG signature on policy file (optional, for paranoid users)
# require_signature = true
# trusted_key = "ABCD1234..."
```

### Agent State File

**Location** (same as current state file):
- Linux: `/var/lib/browser-extension-policy/state.json`
- macOS: `/Library/Application Support/browser-extension-policy/state.json`
- Windows: `C:\ProgramData\browser-extension-policy\state.json`

**Schema** (extends existing state):
```json
{
  "version": "1.0",
  "machine_id": "uuid",
  "config_hash": "sha256-of-applied-config",
  "last_updated": "2025-10-29T15:45:00Z",
  "last_checked": "2025-10-29T15:50:00Z",
  "github_etag": "W/\"abc123...\"",
  "applied_policies": {
    "chrome": { /* ... existing format ... */ },
    "firefox": { /* ... */ },
    "edge": { /* ... */ }
  }
}
```

**New fields**:
- `machine_id`: Unique identifier for this machine (for tracking/debugging)
- `last_checked`: Last time agent polled GitHub (vs. `last_updated` = last applied)
- `github_etag`: GitHub's ETag header (for efficient HTTP caching)

### Agent Daemon Logic

```rust
async fn run_agent_loop(config: AgentConfig) -> Result<()> {
    loop {
        // 1. Add jitter to prevent all agents polling simultaneously
        let jitter = rand::thread_rng().gen_range(-config.poll_jitter..=config.poll_jitter);
        let sleep_duration = Duration::from_secs(config.poll_interval as u64)
            + Duration::from_secs(jitter as u64);

        // 2. Sleep until next check
        tokio::time::sleep(sleep_duration).await;

        // 3. Check for policy updates
        match check_and_apply_policy(&config).await {
            Ok(applied) => {
                if applied {
                    log::info!("Policy updated and applied successfully");
                } else {
                    log::debug!("Policy unchanged");
                }
            }
            Err(e) => {
                log::error!("Failed to check/apply policy: {}", e);
                // Will retry on next iteration
            }
        }
    }
}

async fn check_and_apply_policy(config: &AgentConfig) -> Result<bool> {
    // 1. Load current state
    let mut state = state::load_state().unwrap_or_default();

    // 2. Fetch policy from GitHub (with ETag for caching)
    let client = reqwest::Client::new();
    let mut request = client.get(&config.github.policy_url);

    // Add auth token if configured
    if let Some(token) = &config.github.access_token {
        request = request.header("Authorization", format!("token {}", token));
    }

    // Add ETag for conditional request (saves bandwidth)
    if let Some(etag) = &state.github_etag {
        request = request.header("If-None-Match", etag);
    }

    let response = request.send().await?;

    // 3. Check if content changed
    if response.status() == StatusCode::NOT_MODIFIED {
        // No change, GitHub returned 304
        state.last_checked = Utc::now();
        state::save_state(&state)?;
        return Ok(false);
    }

    if !response.status().is_success() {
        return Err(anyhow!("GitHub returned {}", response.status()));
    }

    // 4. Get new ETag for future requests
    let new_etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // 5. Download policy content
    let policy_yaml = response.text().await?;

    // 6. Optional: Verify GPG signature (if configured)
    if config.security.require_signature {
        verify_signature(&policy_yaml, &config.security.trusted_key)?;
    }

    // 7. Parse policy
    let new_config = Config::from_yaml_str(&policy_yaml)?;

    // 8. Compute hash
    let new_hash = compute_hash(&policy_yaml);

    // 9. Check if policy actually changed
    if state.config_hash.as_ref() == Some(&new_hash) {
        // Same content, just update ETag
        state.github_etag = new_etag;
        state.last_checked = Utc::now();
        state::save_state(&state)?;
        return Ok(false);
    }

    // 10. Apply policy using existing logic
    log::info!("Applying new policy (hash: {})", new_hash);

    if let Some(chrome_config) = new_config.chrome {
        policy::chrome::apply(&chrome_config)?;
    }

    if let Some(firefox_config) = new_config.firefox {
        policy::firefox::apply(&firefox_config)?;
    }

    if let Some(edge_config) = new_config.edge {
        policy::edge::apply(&edge_config)?;
    }

    // 11. Update state
    state.config_hash = Some(new_hash);
    state.last_updated = Utc::now();
    state.last_checked = Utc::now();
    state.github_etag = new_etag;
    state.applied_policies = /* ... extract from applied policies ... */;
    state::save_state(&state)?;

    Ok(true)
}
```

### CLI Commands

```bash
# Initial setup (one-time)
sudo family-policy agent setup \
  --repo-url "https://github.com/username/family-policies" \
  --policy-file "policies/kids-pc.yaml"

# Or directly with raw URL
sudo family-policy agent setup \
  --url "https://raw.githubusercontent.com/username/family-policies/main/policies/kids-pc.yaml"

# Setup with private repo (requires GitHub Personal Access Token)
sudo family-policy agent setup \
  --url "https://raw.githubusercontent.com/username/family-policies/main/policies/kids-pc.yaml" \
  --token "ghp_xxxxxxxxxxxxxxxxxxxx"

# Start agent daemon
sudo family-policy agent start

# Stop agent daemon
sudo family-policy agent stop

# Check agent status
family-policy agent status
# Output:
# ✓ Agent running
# Policy URL: https://raw.githubusercontent.com/.../kids-pc.yaml
# Last checked: 2 minutes ago
# Last updated: 1 hour ago
# Current policy hash: a1b2c3d4...

# Force immediate check (don't wait for next poll)
sudo family-policy agent check-now
# Output:
# Checking for policy updates...
# ✓ Policy unchanged (hash: a1b2c3d4...)

# Or if changed:
# Checking for policy updates...
# ↓ Downloading new policy...
# ✓ Policy applied successfully (hash: e5f6g7h8...)

# View applied configuration
family-policy agent show-config
# Outputs the currently applied YAML

# Update configuration (e.g., change polling interval)
sudo family-policy agent config --poll-interval 600  # 10 minutes

# Uninstall (remove all policies and stop agent)
sudo family-policy agent uninstall
```

---

## Component 2: GitHub Repository Structure

### Recommended Layout

```
family-policies/
├── README.md
├── policies/
│   ├── kids-pc.yaml          # Windows PC in kids' room
│   ├── living-room-mac.yaml  # Family Mac in living room
│   ├── basement-linux.yaml   # Linux machine in basement
│   └── shared.yaml           # Shared base policy (optional)
├── templates/
│   ├── strict.yaml           # Template: Strict filtering
│   ├── moderate.yaml         # Template: Moderate filtering
│   └── minimal.yaml          # Template: Minimal filtering
└── docs/
    └── extension-guide.md    # Documentation for extensions
```

### Example Policy File

**policies/kids-pc.yaml**:
```yaml
# Browser Extension Policy for Kids' PC
# Last updated: 2025-10-29
# Applied to: Kids-PC (Windows 11)

chrome:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite

    - id: gcbommkclmclpchllfjekcdonpmejbdp
      name: HTTPS Everywhere

  disable_incognito: true
  disable_guest_mode: true

firefox:
  extensions:
    - id: uBOLite@raymondhill.net
      name: uBlock Origin Lite
      install_url: https://addons.mozilla.org/firefox/downloads/latest/ublock-origin-lite/latest.xpi

  disable_private_browsing: true

edge:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite

  disable_inprivate: true
  disable_guest_mode: true
```

### Repository Setup Script

Create a helper script to initialize the repository:

```bash
#!/bin/bash
# setup-policy-repo.sh

echo "Setting up family-policies repository..."

# 1. Create directory structure
mkdir -p family-policies/{policies,templates,docs}
cd family-policies

# 2. Create README
cat > README.md <<'EOF'
# Family Browser Policies

This repository contains browser extension policies for family computers.

## How It Works

Each computer runs the `family-policy` agent which polls this repository for changes.
When you update a policy file, agents automatically apply the changes within minutes.

## Policy Files

- `policies/kids-pc.yaml` - Kids' Windows PC
- `policies/living-room-mac.yaml` - Family Mac
- `policies/basement-linux.yaml` - Linux workstation

## Making Changes

1. Edit the policy file for the target computer
2. Commit and push to GitHub
3. Changes apply automatically within 5 minutes

## Security

- **Private Repository**: Keep this repo private to prevent unauthorized access
- **Access Tokens**: Each agent uses a Personal Access Token with read-only access
- **HTTPS**: All communication uses HTTPS (verified by OS certificate store)
EOF

# 3. Create template
cat > templates/moderate.yaml <<'EOF'
# Moderate policy template
chrome:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite
  disable_incognito: false
  disable_guest_mode: true

firefox:
  extensions:
    - id: uBOLite@raymondhill.net
      name: uBlock Origin Lite
      install_url: https://addons.mozilla.org/firefox/downloads/latest/ublock-origin-lite/latest.xpi
  disable_private_browsing: false
EOF

# 4. Initialize git
git init
git add .
git commit -m "Initial commit: Policy repository structure"

echo ""
echo "✓ Repository initialized!"
echo ""
echo "Next steps:"
echo "1. Create a GitHub repository (public or private)"
echo "2. Push this repository:"
echo "   git remote add origin https://github.com/YOUR_USERNAME/family-policies"
echo "   git push -u origin main"
echo ""
echo "3. If using private repo, create a Personal Access Token:"
echo "   https://github.com/settings/tokens/new"
echo "   - Note: 'Family Policy Access'"
echo "   - Expiration: No expiration (or 1 year)"
echo "   - Scope: 'repo' (for private) or 'public_repo' (for public)"
echo ""
echo "4. On each computer, run:"
echo "   sudo family-policy agent setup \\"
echo "     --url 'https://raw.githubusercontent.com/YOUR_USERNAME/family-policies/main/policies/MACHINE.yaml' \\"
echo "     --token 'ghp_xxxxxxxxxxxx'"  # Only for private repos
```

---

## Component 3: Security Model

### Security Through HTTPS + GitHub

**What we get for free**:
- ✅ **Transport security**: HTTPS prevents MITM attacks
- ✅ **Authentication**: GitHub Personal Access Tokens
- ✅ **Authorization**: GitHub repo permissions
- ✅ **Audit trail**: Git commit history
- ✅ **Version control**: Easy rollback via git revert

**Threat Model**:

| Threat | Mitigation |
|--------|-----------|
| **Attacker intercepts traffic** | HTTPS with certificate pinning to GitHub |
| **Attacker modifies policy in transit** | HTTPS prevents tampering |
| **Attacker gains repo access** | Private repo + limited scope PAT |
| **Attacker steals PAT from agent** | PAT has read-only access, limit blast radius |
| **Compromised GitHub account** | Use 2FA on GitHub, monitor commit history |
| **Malicious policy pushed to repo** | Git history provides audit trail, easy revert |

### Access Token Security

**Creating Limited-Scope PAT**:

1. Go to https://github.com/settings/tokens/new
2. Note: "Family Policy Agent - Read Only"
3. Expiration: 1 year (or no expiration for convenience)
4. Scopes:
   - **Private repo**: `repo` (full control needed to read)
   - **Public repo**: `public_repo` (or no scopes needed!)
5. Generate and save token

**Storing Token Securely**:

```rust
// Store token in config file with restricted permissions
fn save_config_with_token(config: &AgentConfig) -> Result<()> {
    let config_path = get_config_path();

    // Create parent directory if needed
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write config
    let toml = toml::to_string_pretty(config)?;
    fs::write(&config_path, toml)?;

    // Set restrictive permissions (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&config_path)?.permissions();
        perms.set_mode(0o600);  // rw-------
        fs::set_permissions(&config_path, perms)?;
    }

    // Windows: Use ACLs to restrict to SYSTEM only
    #[cfg(windows)]
    {
        platform::windows::restrict_file_to_system(&config_path)?;
    }

    Ok(())
}
```

### Optional: GPG Signature Verification (Advanced)

For users who want cryptographic verification:

**Setup**:
1. Admin generates GPG key
2. Configures git to sign commits: `git config commit.gpgsign true`
3. Exports public key and adds to agent config
4. Agent verifies signature before applying policy

**Implementation**:
```rust
#[cfg(feature = "gpg-verify")]
fn verify_signature(content: &str, trusted_key: &str) -> Result<()> {
    // Extract signature from YAML comment block
    // Verify using gpgme or sequoia-pgp crate
    // Return error if signature invalid or not from trusted key
}
```

**Policy File with Signature**:
```yaml
# -----BEGIN PGP SIGNED MESSAGE-----
# Hash: SHA256

chrome:
  extensions:
    - id: ddkjiahejlhfcafbddmgiahcphecmpfh
      name: uBlock Origin Lite
# ... rest of config ...

# -----BEGIN PGP SIGNATURE-----
# iQIzBAEBCAAdFiEE...
# -----END PGP SIGNATURE-----
```

**Recommendation**: Skip this for v1. HTTPS to GitHub is secure enough for home use.

---

## Component 4: User Experience

### Initial Setup Experience

**Step 1: Create GitHub Repository**

```bash
# Download and run setup script
curl -sSL https://example.com/setup-policy-repo.sh | bash

cd family-policies

# Edit policy files
nano policies/kids-pc.yaml

# Create GitHub repo (via web or gh CLI)
gh repo create family-policies --private

# Push
git remote add origin https://github.com/username/family-policies
git push -u origin main
```

**Step 2: Generate Access Token (if private repo)**

1. Visit https://github.com/settings/tokens/new
2. Configure as described above
3. Copy token: `ghp_xxxxxxxxxxxxxxxxxxxx`

**Step 3: Configure Agent on Each Computer**

```bash
# On Kids-PC
sudo family-policy agent setup \
  --url "https://raw.githubusercontent.com/username/family-policies/main/policies/kids-pc.yaml" \
  --token "ghp_xxxxxxxxxxxxxxxxxxxx"

# Output:
# Configuring agent...
# ✓ Testing connection to GitHub...
# ✓ Policy file found and valid
# ✓ Configuration saved
#
# Starting agent...
# ✓ Agent started successfully
#
# The agent will now check for policy updates every 5 minutes.
# Next check: 2025-10-29 16:05:00
```

### Making Policy Changes

```bash
# On admin's computer
cd ~/family-policies

# Edit policy
nano policies/kids-pc.yaml

# Add new extension
cat >> policies/kids-pc.yaml <<EOF
    - id: cjpalhdlnbpafiamejdnhcphjbkeiagm
      name: Privacy Badger
EOF

# Commit and push
git add policies/kids-pc.yaml
git commit -m "Add Privacy Badger extension to Kids-PC"
git push

# Output:
# [main abc123] Add Privacy Badger extension to Kids-PC
#  1 file changed, 2 insertions(+)
#
# Policy will be applied within 5 minutes.
```

**On the agent side** (automatic):
```
# Agent log output:
[2025-10-29 16:02:15] INFO: Checking for policy updates...
[2025-10-29 16:02:16] INFO: New policy detected (hash: e5f6g7h8...)
[2025-10-29 16:02:16] INFO: Downloading policy...
[2025-10-29 16:02:17] INFO: Applying policy...
[2025-10-29 16:02:18] INFO: Chrome: Added extension cjpalhdlnbpafiamejdnhcphjbkeiagm
[2025-10-29 16:02:18] INFO: Policy applied successfully
```

### Checking Status

```bash
# On agent machine
family-policy agent status

# Output:
# Family Policy Agent Status
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Status:      ✓ Running
# Policy URL:  https://raw.githubusercontent.com/.../kids-pc.yaml
# Repository:  username/family-policies
# Branch:      main
#
# Last checked:  30 seconds ago (2025-10-29 16:02:45)
# Last updated:  5 minutes ago (2025-10-29 15:58:17)
# Current hash:  e5f6g7h8...
#
# Current Configuration:
#   Chrome:     3 extensions, incognito disabled, guest disabled
#   Firefox:    1 extension, private browsing disabled
#   Edge:       3 extensions, InPrivate disabled, guest disabled
#
# Next check:   In 4 minutes (2025-10-29 16:07:15)

# Force immediate check
sudo family-policy agent check-now

# Output:
# Checking for policy updates...
# ✓ Policy unchanged (hash: e5f6g7h8...)
```

### Viewing Applied Config

```bash
family-policy agent show-config

# Output (pretty-printed YAML):
# Current Policy Configuration
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Applied at: 2025-10-29 15:58:17
# Hash: e5f6g7h8...
# Source: https://raw.githubusercontent.com/.../kids-pc.yaml
#
# chrome:
#   extensions:
#     - id: ddkjiahejlhfcafbddmgiahcphecmpfh
#       name: uBlock Origin Lite
#     - id: gcbommkclmclpchllfjekcdonpmejbdp
#       name: HTTPS Everywhere
#     - id: cjpalhdlnbpafiamejdnhcphjbkeiagm
#       name: Privacy Badger
#   disable_incognito: true
#   disable_guest_mode: true
#
# firefox:
#   extensions:
#     - id: uBOLite@raymondhill.net
#       name: uBlock Origin Lite
#       install_url: https://addons.mozilla.org/...
#   disable_private_browsing: true
#
# edge:
#   extensions:
#     - id: ddkjiahejlhfcafbddmgiahcphecmpfh
#       name: uBlock Origin Lite
#     - id: gcbommkclmclpchllfjekcdonpmejbdp
#       name: HTTPS Everywhere
#     - id: cjpalhdlnbpafiamejdnhcphjbkeiagm
#       name: Privacy Badger
#   disable_inprivate: true
#   disable_guest_mode: true
```

### Rolling Back Changes

```bash
# Admin realizes mistake in last commit
cd ~/family-policies

# View recent commits
git log --oneline -5

# Output:
# abc123 Add Privacy Badger extension to Kids-PC
# def456 Update uBlock Origin settings
# ghi789 Initial policy for Kids-PC

# Revert last commit
git revert abc123
git push

# Within 5 minutes, agent will detect and apply reverted policy
```

---

## Component 5: Implementation Details

### HTTP Client with ETag Support

```rust
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

pub struct GitHubPoller {
    client: Client,
    config: GitHubConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubConfig {
    pub policy_url: String,
    pub access_token: Option<String>,
}

impl GitHubPoller {
    pub fn new(config: GitHubConfig) -> Self {
        Self {
            client: Client::builder()
                .user_agent("family-policy-agent/1.0")
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            config,
        }
    }

    pub async fn fetch_policy(&self, etag: Option<&str>) -> Result<PolicyFetchResult> {
        let mut request = self.client.get(&self.config.policy_url);

        // Add authentication if configured
        if let Some(token) = &self.config.access_token {
            request = request.header("Authorization", format!("token {}", token));
        }

        // Add ETag for conditional request
        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag);
        }

        let response = request.send().await?;

        match response.status() {
            StatusCode::NOT_MODIFIED => {
                // Content hasn't changed
                Ok(PolicyFetchResult::NotModified)
            }
            StatusCode::OK => {
                let new_etag = response
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);

                let content = response.text().await?;

                Ok(PolicyFetchResult::Updated {
                    content,
                    etag: new_etag,
                })
            }
            StatusCode::NOT_FOUND => {
                Err(anyhow!("Policy file not found (404). Check URL and repository access."))
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(anyhow!(
                    "Access denied ({}). Check access token and repository permissions.",
                    response.status()
                ))
            }
            status => {
                Err(anyhow!("GitHub returned unexpected status: {}", status))
            }
        }
    }
}

pub enum PolicyFetchResult {
    NotModified,
    Updated { content: String, etag: Option<String> },
}
```

### Efficient Polling with Jitter

```rust
use rand::Rng;
use tokio::time::{sleep, Duration};

pub struct PollingScheduler {
    base_interval: Duration,
    jitter_range: Duration,
}

impl PollingScheduler {
    pub fn new(interval_secs: u64, jitter_secs: u64) -> Self {
        Self {
            base_interval: Duration::from_secs(interval_secs),
            jitter_range: Duration::from_secs(jitter_secs),
        }
    }

    pub async fn sleep_until_next_poll(&self) {
        let jitter = rand::thread_rng().gen_range(0..self.jitter_range.as_secs());
        let total_sleep = self.base_interval + Duration::from_secs(jitter);

        tracing::debug!("Sleeping for {} seconds until next poll", total_sleep.as_secs());
        sleep(total_sleep).await;
    }

    pub fn next_poll_time(&self) -> chrono::DateTime<chrono::Utc> {
        let jitter = rand::thread_rng().gen_range(0..self.jitter_range.as_secs());
        let total_secs = self.base_interval.as_secs() + jitter;

        chrono::Utc::now() + chrono::Duration::seconds(total_secs as i64)
    }
}
```

### Error Handling with Retry

```rust
pub async fn check_and_apply_with_retry(
    config: &AgentConfig,
    max_retries: u32,
) -> Result<bool> {
    let mut retries = 0;

    loop {
        match check_and_apply_policy(config).await {
            Ok(applied) => return Ok(applied),
            Err(e) if retries < max_retries => {
                retries += 1;
                tracing::warn!(
                    "Failed to check/apply policy (attempt {}/{}): {}",
                    retries,
                    max_retries,
                    e
                );

                // Exponential backoff: 60s, 120s, 240s
                let backoff = Duration::from_secs(60 * (2_u64.pow(retries - 1)));
                tracing::info!("Retrying in {} seconds...", backoff.as_secs());
                tokio::time::sleep(backoff).await;
            }
            Err(e) => {
                tracing::error!("Failed to check/apply policy after {} retries: {}", retries, e);
                return Err(e);
            }
        }
    }
}
```

### Configuration Parsing

```rust
use config::{Config as ConfigLoader, File, FileFormat};

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentConfig {
    pub github: GitHubConfig,
    pub agent: AgentSettings,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentSettings {
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,  // seconds

    #[serde(default = "default_jitter")]
    pub poll_jitter: u64,    // seconds

    #[serde(default = "default_retry_interval")]
    pub retry_interval: u64, // seconds

    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_poll_interval() -> u64 { 300 }  // 5 minutes
fn default_jitter() -> u64 { 60 }           // ±1 minute
fn default_retry_interval() -> u64 { 60 }  // 1 minute
fn default_max_retries() -> u32 { 3 }

impl AgentConfig {
    pub fn load() -> Result<Self> {
        let config_path = platform::get_config_path()?;

        ConfigLoader::builder()
            .add_source(File::new(
                config_path.to_str().unwrap(),
                FileFormat::Toml,
            ))
            .build()?
            .try_deserialize()
            .map_err(Into::into)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = platform::get_config_path()?;
        let toml = toml::to_string_pretty(self)?;

        // Create parent directory
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&config_path, toml)?;

        // Set restrictive permissions
        platform::restrict_file_permissions(&config_path)?;

        Ok(())
    }
}
```

---

## Component 6: Advanced Features (Optional)

### Multiple Policy Sources

Support checking multiple repos or files:

```toml
# agent.conf
[[github.policies]]
url = "https://raw.githubusercontent.com/org/shared-policies/main/base.yaml"
priority = 1  # Applied first

[[github.policies]]
url = "https://raw.githubusercontent.com/username/my-policies/main/kids-pc.yaml"
priority = 2  # Applied second (overwrites conflicts)

# Merge strategy: later policies override earlier ones
```

### Webhook-Triggered Updates

For near-instant updates instead of polling:

```toml
[webhook]
enabled = true
listen_address = "127.0.0.1"
listen_port = 8746
secret = "webhook-secret-shared-with-github"
```

**GitHub Configuration**:
1. Repository Settings → Webhooks → Add webhook
2. Payload URL: `http://your-public-ip:8746/webhook` (requires port forwarding)
3. Content type: `application/json`
4. Secret: (same as config)
5. Events: "Just the push event"

**Agent behavior**:
- Still polls on regular interval (fallback)
- Also listens for webhook POST requests
- On webhook received: verify signature, trigger immediate check

**Note**: Requires exposing agent to internet. Not recommended for most home users. Polling is simpler and more secure.

### Policy Templating

Support for variables in policy files:

**policies/kids-pc.yaml**:
```yaml
# Variables (substituted by agent)
vars:
  machine_name: "Kids-PC"
  os_type: "windows"

chrome:
  extensions:
    # Include shared extensions
    !include ../templates/standard-extensions.yaml

    # Add machine-specific extensions
    - id: custom-extension-id
      name: Custom Extension
```

**Implementation**: Use a YAML preprocessor before parsing.

### Conditional Policies

Apply different policies based on time or other conditions:

```yaml
# Apply different policies during school hours
schedules:
  - name: school_hours
    time: "08:00-15:00"
    weekdays: [mon, tue, wed, thu, fri]
    policy:
      chrome:
        extensions:
          - id: extension-for-school
        disable_incognito: true

  - name: after_school
    time: "15:00-20:00"
    policy:
      chrome:
        extensions:
          - id: extension-for-homework
        disable_incognito: false
```

**Implementation**: Agent evaluates conditions before applying policy.

---

## Component 7: Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_hash_consistency() {
        let yaml = "chrome:\n  extensions:\n    - id: test123";
        let hash1 = compute_hash(yaml);
        let hash2 = compute_hash(yaml);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_etag_parsing() {
        let response = create_mock_response_with_etag("W/\"abc123\"");
        let etag = extract_etag(&response);
        assert_eq!(etag, Some("W/\"abc123\"".to_string()));
    }

    #[tokio::test]
    async fn test_polling_scheduler_jitter() {
        let scheduler = PollingScheduler::new(300, 60);
        let next1 = scheduler.next_poll_time();
        let next2 = scheduler.next_poll_time();

        // Should be different due to jitter
        assert_ne!(next1, next2);

        // But within expected range
        let diff = (next1 - next2).num_seconds().abs();
        assert!(diff <= 60);
    }
}
```

### Integration Tests (with Mock GitHub)

```rust
#[tokio::test]
async fn test_fetch_policy_from_mock_server() {
    // Start mock HTTP server
    let server = MockServer::start().await;

    // Mock the GitHub API
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path("/test-policy.yaml");
        then.status(200)
            .header("etag", "W/\"abc123\"")
            .body("chrome:\n  extensions:\n    - id: test123");
    });

    // Test fetching
    let config = GitHubConfig {
        policy_url: format!("{}/test-policy.yaml", server.url()),
        access_token: None,
    };

    let poller = GitHubPoller::new(config);
    let result = poller.fetch_policy(None).await.unwrap();

    match result {
        PolicyFetchResult::Updated { content, etag } => {
            assert!(content.contains("test123"));
            assert_eq!(etag, Some("W/\"abc123\"".to_string()));
        }
        _ => panic!("Expected updated result"),
    }

    mock.assert();
}

#[tokio::test]
async fn test_not_modified_response() {
    let server = MockServer::start().await;

    let mock = server.mock(|when, then| {
        when.method("GET")
            .path("/test-policy.yaml")
            .header("if-none-match", "W/\"abc123\"");
        then.status(304);
    });

    let config = GitHubConfig {
        policy_url: format!("{}/test-policy.yaml", server.url()),
        access_token: None,
    };

    let poller = GitHubPoller::new(config);
    let result = poller.fetch_policy(Some("W/\"abc123\"")).await.unwrap();

    assert!(matches!(result, PolicyFetchResult::NotModified));
    mock.assert();
}
```

### Manual Testing Checklist

- [ ] Agent fetches policy on first run
- [ ] Agent applies policy correctly
- [ ] Agent detects changes and re-applies
- [ ] Agent doesn't re-apply unchanged policy (ETag works)
- [ ] Agent handles network errors gracefully
- [ ] Agent respects polling interval + jitter
- [ ] Agent logs useful information
- [ ] Agent works with public repos (no token)
- [ ] Agent works with private repos (with token)
- [ ] Agent handles 404 errors (file not found)
- [ ] Agent handles 401/403 errors (auth failure)
- [ ] `check-now` command works
- [ ] `show-config` displays current policy
- [ ] Status command shows accurate information

---

## Component 8: Documentation

### README.md for GitHub Repo

```markdown
# Family Browser Policies

Automated browser extension management for family computers.

## Quick Start

### 1. Install Agent on Each Computer

**Linux/macOS**:
```bash
curl -sSL https://example.com/install.sh | sudo bash
```

**Windows**:
Download and run [installer](https://example.com/family-policy-installer.exe)

### 2. Configure Agent

```bash
sudo family-policy agent setup \
  --url "https://raw.githubusercontent.com/YOUR_USERNAME/family-policies/main/policies/MACHINE.yaml" \
  --token "ghp_xxxxxxxxxxxx"  # Only if private repo
```

### 3. Edit Policies

Edit the YAML files in the `policies/` directory, commit, and push.
Changes apply automatically within 5 minutes.

## Policy File Format

See [Policy Documentation](docs/policy-format.md)

## Troubleshooting

**Agent not applying policies?**
```bash
# Check agent status
family-policy agent status

# Check logs
sudo tail -f /var/log/family-policy-agent.log

# Force immediate check
sudo family-policy agent check-now
```

**Authentication errors?**
- Verify your GitHub access token is valid
- For private repos, ensure token has `repo` scope
- Check repository permissions

## Security

- Keep this repository **private**
- Use GitHub Personal Access Tokens (PATs) with minimal scope
- Review commit history regularly
- Enable 2FA on your GitHub account
```

---

## Component 9: Comparison with Other Designs

| Aspect | GitHub Polling | Client-Server (SIMPLE_REMOTE_DESIGN.md) | Original (REMOTE_MANAGEMENT_RESEARCH.md) |
|--------|----------------|--------------------------|---------------------------|
| **Complexity** | 🟢 Very Low | 🟡 Medium | 🔴 High |
| **Setup Difficulty** | 🟢 Easy (GitHub + config) | 🟡 Medium (cert exchange) | 🔴 Complex (CA, DB, server) |
| **Infrastructure** | 🟢 GitHub only | 🟡 Admin's computer | 🔴 Dedicated server + DB |
| **Firewall Config** | 🟢 None needed | 🟡 Port on clients | 🟡 Port on server |
| **Security** | 🟢 HTTPS to GitHub | 🟢 Public key auth | 🟢 mTLS with CA |
| **Audit Trail** | 🟢 Git history | 🟡 Local logs | 🟢 Database logs |
| **Version Control** | 🟢 Built-in (git) | ❌ Manual | 🟡 Database versioning |
| **Rollback** | 🟢 `git revert` | 🟡 Re-push old policy | 🟡 Select old version |
| **Update Latency** | 🟡 5 min polling | 🟢 Immediate push | 🟢 Immediate (or poll) |
| **Scale** | 🟢 Hundreds of clients | 🟢 Dozens of clients | 🟢 Thousands of clients |
| **Offline Mode** | ❌ Requires internet | ❌ Requires network | ❌ Requires network |
| **Cost** | 🟢 Free (GitHub) | 🟢 Free | 🟡 Server hosting |
| **Best For** | 🏠 Families, small biz | 🏠 Families | 🏢 Enterprises |

---

## Component 10: Implementation Roadmap

### Phase 1: Core Polling Agent (2 weeks)
- [ ] HTTP client with ETag support
- [ ] GitHub authentication (PAT)
- [ ] Policy fetching logic
- [ ] Integration with existing policy application
- [ ] State file with hash/etag tracking
- [ ] Basic error handling

### Phase 2: Agent Daemon (1 week)
- [ ] Polling loop with jitter
- [ ] Retry logic with exponential backoff
- [ ] Platform-specific service installation
- [ ] Logging infrastructure
- [ ] Signal handling (graceful shutdown)

### Phase 3: CLI Commands (1 week)
- [ ] `agent setup` - Initial configuration
- [ ] `agent start/stop/status`
- [ ] `agent check-now` - Force immediate check
- [ ] `agent show-config` - Display current policy
- [ ] `agent uninstall`
- [ ] Colored terminal output

### Phase 4: Testing & Polish (1-2 weeks)
- [ ] Unit tests (hash, ETag, parsing)
- [ ] Integration tests (mock HTTP server)
- [ ] Cross-platform testing
- [ ] Error message improvements
- [ ] Documentation
- [ ] Setup scripts

### Phase 5: Packaging (1 week)
- [ ] DEB package (Debian/Ubuntu)
- [ ] RPM package (Fedora/RHEL)
- [ ] PKG installer (macOS)
- [ ] MSI installer (Windows)
- [ ] Install scripts
- [ ] Uninstall scripts

**Total Timeline**: 6-7 weeks (~1.5 months)

### Optional Phase 6: Advanced Features (2-3 weeks)
- [ ] GPG signature verification
- [ ] Multiple policy sources
- [ ] Policy templating
- [ ] Webhook support
- [ ] Conditional policies (time-based)

---

## Conclusion

This GitHub-based approach is **significantly simpler** than client-server architectures:

✅ **No server to run** - GitHub is the "server"
✅ **No listening ports** - Agents only make outbound HTTPS requests
✅ **No complex auth** - GitHub handles it
✅ **Built-in version control** - Git history
✅ **Easy rollback** - `git revert`
✅ **Audit trail** - Commit history
✅ **Free hosting** - GitHub is free for private repos
✅ **Reliable** - GitHub's uptime is better than home servers
✅ **Simple setup** - Configure URL + token, done

**Perfect for**:
- Families managing 3-10 computers
- Small businesses with <50 machines
- Anyone who wants "set it and forget it" simplicity

**Trade-offs**:
- 5-minute latency (vs. instant push)
- Requires internet access
- Relies on GitHub availability
- Not suitable for air-gapped environments

For home use, these trade-offs are well worth the massive simplification!
