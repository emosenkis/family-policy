# Simple Remote Management Design

## Overview

A single Rust binary that works in two modes:
1. **Agent mode**: Lightweight daemon on client machines that listens for policy pushes
2. **Manager mode**: CLI tool on admin's machine that pushes policies to agents

No database, no web server, no complex infrastructure. Just secure peer-to-peer communication between an admin's machine and family computers.

---

## Architecture

```
Admin's Laptop/Desktop                    Family Computers
┌────────────────────┐                    ┌──────────────┐
│  family-policy     │  Push Policy →    │ Agent Daemon │
│  (manager mode)    │─────────────────→ │ (listening)  │
│                    │  ← Status Report  │              │
└────────────────────┘                    └──────────────┘
         │                                        │
         │ Local state:                          │ Local state:
         │ agents.json                           │ managers.json
         │ (known clients)                       │ (approved managers)
         │                                        │ Applied policy
         │                                        │ (same as current)
```

**Key Principles**:
- Single binary, dual-mode operation
- Push model: Admin initiates policy updates
- No always-running server component
- Home OS compatible (Windows Home, macOS, regular Linux)
- Simple, user-approved registration
- Lightweight: <10 MB binary, <20 MB RAM for agent

---

## Component 1: Single Binary with Dual Modes

### File Structure
```
src/
  main.rs                    # Entry point, mode dispatcher
  agent/
    mod.rs                   # Agent mode orchestration
    listener.rs              # Network listener
    policy_applier.rs        # Applies received policies
    manager_registry.rs      # Tracks approved managers
  manager/
    mod.rs                   # Manager mode orchestration
    client_registry.rs       # Tracks known agents
    policy_pusher.rs         # Pushes policies to agents
  crypto/
    mod.rs                   # Simple crypto primitives
    keypair.rs              # Ed25519 key generation
    auth.rs                 # Challenge-response auth
  protocol/
    mod.rs                   # Wire protocol definitions
    messages.rs             # Request/response types
  config.rs                  # (existing) Policy config
  policy/                    # (existing) Policy application
  platform/                  # (existing) Platform-specific
  state.rs                   # (existing) State management
```

### CLI Interface

#### Agent Commands
```bash
# Start agent daemon (runs in background)
sudo family-policy agent start

# Stop agent daemon
sudo family-policy agent stop

# Get registration token for manager to use
family-policy agent get-token

# List approved managers
family-policy agent list-managers

# Revoke a manager's access
sudo family-policy agent revoke --manager <id>

# Check agent status
family-policy agent status
```

#### Manager Commands
```bash
# Add new client (interactive approval on client side)
family-policy manager add-client --host 192.168.1.100 --name "Kids-PC"

# Add client using registration token (no interaction needed)
family-policy manager add-client --host 192.168.1.100 --token <token>

# List managed clients
family-policy manager list-clients

# Push policy to specific client
family-policy manager push --client "Kids-PC" --config policy.yaml

# Push policy to multiple clients
family-policy manager push --client "Kids-PC" --client "Living-Room-Mac" --config policy.yaml

# Push policy to all clients
family-policy manager push --all --config policy.yaml

# Get status from client
family-policy manager status --client "Kids-PC"

# Remove client from management
family-policy manager remove-client --client "Kids-PC"

# Uninstall policies from client (revert to no management)
family-policy manager uninstall --client "Kids-PC"
```

#### Backwards Compatibility
```bash
# Existing local-only mode still works
sudo family-policy --config policy.yaml  # Apply locally
sudo family-policy --uninstall            # Remove locally
```

---

## Component 2: Agent Daemon

### Responsibilities

1. **Listen for connections** on configurable port (default: 8745)
2. **Authenticate incoming requests** from approved managers
3. **Apply received policies** using existing policy logic
4. **Report status** back to manager
5. **Maintain approved manager list** in local file

### Agent State File

**Location**: Same as current state file location per platform
- Linux: `/var/lib/browser-extension-policy/agent-state.json`
- macOS: `/Library/Application Support/browser-extension-policy/agent-state.json`
- Windows: `C:\ProgramData\browser-extension-policy\agent-state.json`

**Schema**:
```json
{
  "version": "1.0",
  "agent_keypair": {
    "public_key": "base64-encoded-ed25519-public-key",
    "private_key_encrypted": "base64-encoded-encrypted-private-key"
  },
  "approved_managers": [
    {
      "id": "uuid",
      "name": "Admin's Laptop",
      "public_key": "base64-encoded-ed25519-public-key",
      "added_at": "2025-10-29T10:30:00Z",
      "last_seen": "2025-10-29T15:45:00Z"
    }
  ],
  "registration_tokens": [
    {
      "token": "xxxx-xxxx-xxxx",
      "created_at": "2025-10-29T10:00:00Z",
      "expires_at": "2025-10-29T10:15:00Z",
      "used": false
    }
  ],
  "current_policy": {
    "hash": "sha256-of-applied-config",
    "applied_at": "2025-10-29T15:45:00Z",
    "applied_by_manager": "uuid"
  }
}
```

### Agent Daemon Implementation

**Platform-Specific Service Management**:

**Linux (systemd)**:
```ini
# /etc/systemd/system/family-policy-agent.service
[Unit]
Description=Family Policy Agent
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/family-policy agent start --no-daemon
Restart=on-failure
User=root

[Install]
WantedBy=multi-user.target
```

**macOS (LaunchDaemon)**:
```xml
<!-- /Library/LaunchDaemons/com.example.family-policy-agent.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" ...>
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.family-policy-agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/family-policy</string>
        <string>agent</string>
        <string>start</string>
        <string>--no-daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

**Windows (Windows Service)**:
- Uses `windows-service` crate
- Registered as service: `FamilyPolicyAgent`
- Runs under SYSTEM account

### Configuration File

**Location**:
- Linux: `/etc/family-policy/agent.conf`
- macOS: `/Library/Application Support/family-policy/agent.conf`
- Windows: `C:\ProgramData\family-policy\agent.conf`

**Format** (TOML):
```toml
# Network settings
[network]
listen_address = "0.0.0.0"
listen_port = 8745

# Security settings
[security]
# Require interactive approval for new managers (true = show prompt, false = require token)
require_approval = true

# Automatically approve requests from local network (192.168.0.0/16, 10.0.0.0/8)
auto_approve_local = false

# Registration token expiry (minutes)
token_expiry = 15

# Logging
[logging]
level = "info"  # debug, info, warn, error
file = "/var/log/family-policy-agent.log"
```

---

## Component 3: Manager CLI

### Manager State File

**Location** (user-local, not system-wide):
- Linux: `~/.config/family-policy/manager-state.json`
- macOS: `~/Library/Application Support/family-policy/manager-state.json`
- Windows: `%APPDATA%\family-policy\manager-state.json`

**Schema**:
```json
{
  "version": "1.0",
  "manager_keypair": {
    "public_key": "base64-encoded-ed25519-public-key",
    "private_key_encrypted": "base64-encoded-encrypted-private-key"
  },
  "clients": [
    {
      "id": "uuid",
      "name": "Kids-PC",
      "host": "192.168.1.100",
      "port": 8745,
      "public_key": "base64-encoded-ed25519-public-key",
      "added_at": "2025-10-29T10:30:00Z",
      "last_contact": "2025-10-29T15:45:00Z",
      "os": "windows",
      "last_policy_hash": "sha256-of-last-pushed-config"
    }
  ]
}
```

### Push Operation Flow

```rust
// Pseudo-code for push operation
async fn push_policy(client_id: &str, config_path: &Path) -> Result<()> {
    // 1. Load manager state (our keypair, client info)
    let manager_state = load_manager_state()?;
    let client = manager_state.get_client(client_id)?;

    // 2. Parse and validate config
    let config = Config::from_file(config_path)?;
    let config_hash = compute_hash(&config);

    // 3. Check if policy changed (skip if same)
    if client.last_policy_hash == Some(config_hash.clone()) {
        println!("Policy unchanged, skipping");
        return Ok(());
    }

    // 4. Connect to agent
    let mut connection = connect_to_agent(&client.host, client.port).await?;

    // 5. Authenticate (challenge-response with our keypair)
    authenticate(&mut connection, &manager_state.keypair, &client.public_key).await?;

    // 6. Send policy
    let response = connection.send_policy(&config).await?;

    // 7. Update our state with result
    if response.success {
        manager_state.update_client_policy(client_id, config_hash);
        println!("✓ Policy applied successfully on {}", client.name);
    } else {
        eprintln!("✗ Policy application failed: {}", response.error);
    }

    Ok(())
}
```

---

## Component 4: Security Model

### Key Management (Simple and Secure)

**Cryptographic Approach**: Ed25519 key pairs (same as SSH)
- Public key: Shared with other party during registration
- Private key: Stored locally, never transmitted

**Key Generation**:
- First run of agent/manager: Generate keypair automatically
- Store in state file with basic encryption (platform keyring if available)

### Registration Flow Options

#### Option A: Interactive Approval (Default, Most Secure)

```
┌─────────────┐                           ┌──────────────┐
│   Manager   │                           │    Agent     │
└──────┬──────┘                           └──────┬───────┘
       │                                         │
       │ 1. family-policy manager add-client    │
       │    --host 192.168.1.100                │
       │                                         │
       │ 2. TCP connection + registration req   │
       ├────────────────────────────────────────>│
       │    (includes manager's public key,     │
       │     name, timestamp)                   │
       │                                         │
       │                                         │ 3. Agent shows
       │                                         │    notification:
       │                                         │    "Manager 'Admin's
       │                                         │    Laptop' wants to
       │                                         │    connect. Approve?"
       │                                         │    [Approve] [Deny]
       │                                         │
       │                                         │ 4. User clicks Approve
       │                                         │
       │ 5. Registration approved               │
       │    (includes agent's public key)       │
       │<────────────────────────────────────────┤
       │                                         │
       │ 6. Both sides save public keys         │
       │    Future connections authenticated    │
       │                                         │
```

**Notification Mechanism**:
- **Linux**: Use `notify-send` or write to system log (user sees with `journalctl -f`)
- **macOS**: Use `osascript` to show native dialog
- **Windows**: Use `msg` command or Windows notification API

**Fallback**: If no interactive session, write to log file with instruction to manually approve:
```bash
sudo family-policy agent approve --request-id <id>
```

#### Option B: Registration Token (Convenient, Still Secure)

```
┌─────────────┐                           ┌──────────────┐
│   Manager   │                           │    Agent     │
└──────┬──────┘                           └──────┬───────┘
       │                                         │
       │                                         │ 1. family-policy agent
       │                                         │    get-token
       │                                         │
       │                                         │ Displays:
       │                                         │ "Token: ABCD-EFGH-IJKL"
       │                                         │ "Expires in 15 minutes"
       │                                         │
       │ 2. User copies token                   │
       │                                         │
       │ 3. family-policy manager add-client    │
       │    --host 192.168.1.100                │
       │    --token ABCD-EFGH-IJKL              │
       │                                         │
       │ 4. Connection + registration           │
       ├────────────────────────────────────────>│
       │    (includes token + manager pubkey)   │
       │                                         │
       │                                         │ 5. Agent validates token
       │                                         │    (not expired, not used)
       │                                         │
       │ 6. Auto-approved, returns agent pubkey │
       │<────────────────────────────────────────┤
       │                                         │
       │ 7. Both sides save public keys         │
       │                                         │
```

**Token Format**:
- 4 words from BIP39 word list (easy to type, memorable)
- Example: `laptop-coffee-sunset-piano`
- 128 bits of entropy (same as UUID)
- Expires after 15 minutes (configurable)
- Single-use only

### Authentication Protocol (Per-Request)

Every policy push is authenticated using challenge-response:

```
┌─────────────┐                           ┌──────────────┐
│   Manager   │                           │    Agent     │
└──────┬──────┘                           └──────┬───────┘
       │                                         │
       │ 1. Connect to agent                    │
       ├────────────────────────────────────────>│
       │                                         │
       │ 2. Challenge (random 32 bytes)         │
       │<────────────────────────────────────────┤
       │                                         │
       │ 3. Sign challenge with private key     │
       │    + send manager's public key ID      │
       ├────────────────────────────────────────>│
       │                                         │
       │                                         │ 4. Verify signature
       │                                         │    using stored pubkey
       │                                         │
       │ 5. Session token (valid for 5 min)     │
       │<────────────────────────────────────────┤
       │                                         │
       │ 6. Send policy with session token      │
       ├────────────────────────────────────────>│
       │                                         │
       │                                         │ 7. Apply policy
       │                                         │
       │ 8. Status response                     │
       │<────────────────────────────────────────┤
       │                                         │
```

**Why This Works**:
- Challenge prevents replay attacks
- Signature proves manager owns private key
- No passwords or tokens transmitted
- Same model as SSH authentication

### Transport Security

**TLS Without Certificates**:
- Use `rustls` with self-signed certificates
- Pin certificates during registration (like SSH known_hosts)
- No need for Certificate Authority

**Alternatively**: Noise Protocol Framework
- Modern, simple cryptographic protocol
- Used by WireGuard, Lightning Network
- Rust implementation: `snow` crate
- Built-in perfect forward secrecy

**Recommendation**: Start with TLS + certificate pinning (simpler), consider Noise for v2.

### Network Security

**Firewall Considerations**:
- Agent listens on port 8745 (configurable)
- Recommend restricting to local network only
- Document how to configure firewall per OS

**Connection Limits**:
- Rate limiting: Max 10 connection attempts per minute per IP
- Connection timeout: 30 seconds
- Max message size: 1 MB (config files shouldn't be huge)

---

## Component 5: Wire Protocol

### Message Format (JSON over TLS)

All messages are JSON with this envelope:
```json
{
  "version": 1,
  "message_type": "register_request|register_response|auth_challenge|...",
  "payload": { /* type-specific payload */ }
}
```

### Message Types

#### Registration Messages

**RegisterRequest** (Manager → Agent):
```json
{
  "version": 1,
  "message_type": "register_request",
  "payload": {
    "manager_public_key": "base64...",
    "manager_name": "Admin's Laptop",
    "token": "laptop-coffee-sunset-piano",  // Optional
    "timestamp": "2025-10-29T15:45:00Z"
  }
}
```

**RegisterResponse** (Agent → Manager):
```json
{
  "version": 1,
  "message_type": "register_response",
  "payload": {
    "success": true,
    "agent_public_key": "base64...",
    "agent_name": "Kids-PC",
    "agent_id": "uuid",
    "requires_approval": false  // If true, manager should poll
  }
}
```

**RegistrationApproval** (Agent → Manager, async):
```json
{
  "version": 1,
  "message_type": "registration_approved",
  "payload": {
    "agent_public_key": "base64...",
    "agent_name": "Kids-PC",
    "agent_id": "uuid"
  }
}
```

#### Authentication Messages

**AuthChallenge** (Agent → Manager):
```json
{
  "version": 1,
  "message_type": "auth_challenge",
  "payload": {
    "challenge": "base64-encoded-random-bytes",
    "timestamp": "2025-10-29T15:45:00Z"
  }
}
```

**AuthResponse** (Manager → Agent):
```json
{
  "version": 1,
  "message_type": "auth_response",
  "payload": {
    "manager_id": "uuid",
    "signature": "base64-encoded-signed-challenge"
  }
}
```

**SessionToken** (Agent → Manager):
```json
{
  "version": 1,
  "message_type": "session_token",
  "payload": {
    "token": "base64...",
    "expires_at": "2025-10-29T15:50:00Z"
  }
}
```

#### Policy Messages

**PushPolicy** (Manager → Agent):
```json
{
  "version": 1,
  "message_type": "push_policy",
  "payload": {
    "session_token": "base64...",
    "config_yaml": "chrome:\n  extensions:\n    ...",
    "config_hash": "sha256-of-yaml",
    "dry_run": false  // If true, validate but don't apply
  }
}
```

**PolicyResponse** (Agent → Manager):
```json
{
  "version": 1,
  "message_type": "policy_response",
  "payload": {
    "success": true,
    "applied_at": "2025-10-29T15:45:00Z",
    "config_hash": "sha256-of-yaml",
    "error": null,  // Or error message if success=false
    "applied_state": {
      "chrome": {
        "extensions": ["id1", "id2"],
        "disable_incognito": true
      },
      // ... (same as current state.json format)
    }
  }
}
```

#### Status Messages

**StatusRequest** (Manager → Agent):
```json
{
  "version": 1,
  "message_type": "status_request",
  "payload": {
    "session_token": "base64..."
  }
}
```

**StatusResponse** (Agent → Manager):
```json
{
  "version": 1,
  "message_type": "status_response",
  "payload": {
    "agent_version": "1.0.0",
    "os": "windows",
    "os_version": "Windows 11 Home",
    "uptime_seconds": 86400,
    "current_policy_hash": "sha256...",
    "last_policy_applied": "2025-10-29T10:30:00Z",
    "applied_by_manager": "uuid",
    "applied_state": { /* same as policy_response */ }
  }
}
```

#### Uninstall Message

**UninstallRequest** (Manager → Agent):
```json
{
  "version": 1,
  "message_type": "uninstall_request",
  "payload": {
    "session_token": "base64..."
  }
}
```

**UninstallResponse** (Agent → Manager):
```json
{
  "version": 1,
  "message_type": "uninstall_response",
  "payload": {
    "success": true,
    "removed_policies": ["chrome", "firefox", "edge"]
  }
}
```

---

## Component 6: Implementation Details

### Rust Crates

```toml
[dependencies]
# Core
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Crypto
ed25519-dalek = "2"  # Ed25519 signatures
rand = "0.8"         # Random number generation
sha2 = "0.10"        # SHA-256 hashing
base64 = "0.21"      # Encoding

# Network
tokio-rustls = "0.25"  # TLS support
rustls = "0.22"
rustls-pemfile = "2"

# Alternative: Noise protocol
# snow = "0.9"  # If using Noise instead of TLS

# CLI
clap = { version = "4", features = ["derive"] }
colored = "2"  # Colored terminal output

# Platform-specific (existing)
[target.'cfg(windows)'.dependencies]
winreg = "0.52"
windows-service = "0.7"

[target.'cfg(target_os = "macos")'.dependencies]
plist = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Not registered: {0}")]
    NotRegistered(String),

    #[error("Registration denied by user")]
    RegistrationDenied,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Policy application failed: {0}")]
    PolicyApplicationFailed(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] std::io::Error),

    #[error("Crypto error: {0}")]
    CryptoError(String),
}
```

### Configuration Validation

Before pushing policy, manager should validate:
```rust
fn validate_before_push(config: &Config) -> Result<()> {
    // 1. YAML syntax valid (already parsed)

    // 2. Extension IDs valid format
    for browser in [&config.chrome, &config.firefox, &config.edge] {
        if let Some(browser_config) = browser {
            for ext in &browser_config.extensions {
                validate_extension_id(&ext.id)?;
            }
        }
    }

    // 3. URLs valid (Firefox install URLs)

    // 4. No obviously wrong values

    Ok(())
}
```

### Logging

**Agent Logs**:
- Connection attempts (IP, manager ID, result)
- Policy applications (hash, timestamp, success/failure)
- Registration events (new manager approved/denied)
- Errors and warnings

**Manager Logs**:
- Push attempts (client, policy hash, result)
- Connection failures
- Authentication errors

**Log Locations**:
- Linux: `/var/log/family-policy-agent.log` (agent), `~/.local/share/family-policy/manager.log` (manager)
- macOS: `/Library/Logs/family-policy-agent.log`, `~/Library/Logs/family-policy-manager.log`
- Windows: `C:\ProgramData\family-policy\agent.log`, `%APPDATA%\family-policy\manager.log`

---

## Component 7: User Experience

### Installation Experience

#### Agent Installation (on family computers)

**Linux**:
```bash
# Using installer script
curl -sSL https://example.com/install.sh | sudo bash

# Or manual
sudo dpkg -i family-policy_1.0.0_amd64.deb  # Debian/Ubuntu
sudo rpm -i family-policy-1.0.0.x86_64.rpm  # Fedora/RHEL

# Post-install: Service starts automatically
sudo systemctl status family-policy-agent
```

**macOS**:
```bash
# Using installer package
sudo installer -pkg family-policy-1.0.0.pkg -target /

# Or using Homebrew
brew install family-policy

# Post-install: Launch daemon loaded automatically
sudo launchctl list | grep family-policy
```

**Windows**:
```
1. Download family-policy-1.0.0-installer.exe
2. Run as Administrator
3. Installer:
   - Copies binary to Program Files
   - Registers Windows Service
   - Starts service
   - Opens firewall port (with user consent)
```

#### Manager Setup (on admin's computer)

**All Platforms**:
```bash
# Install same binary (doesn't start agent on admin's machine)
# Just makes CLI available

# Initialize manager
family-policy manager init

# Output:
# ✓ Manager initialized
# ✓ Keypair generated
# Configuration saved to ~/.config/family-policy/manager-state.json
```

### Registration Experience

#### Scenario 1: Adding First Client

**On agent machine (Kids-PC)**:
```bash
# Check agent is running
family-policy agent status
# Output: ✓ Agent running, listening on port 8745

# Generate token (optional, for convenience)
family-policy agent get-token
# Output:
# Registration Token: laptop-coffee-sunset-piano
# Expires in: 15 minutes
```

**On manager machine (Admin's Laptop)**:
```bash
family-policy manager add-client \
  --host 192.168.1.100 \
  --name "Kids-PC" \
  --token laptop-coffee-sunset-piano

# Output:
# Connecting to 192.168.1.100:8745...
# ✓ Connected
# ✓ Registration accepted
# ✓ Client "Kids-PC" added successfully
#
# You can now push policies with:
#   family-policy manager push --client "Kids-PC" --config policy.yaml
```

#### Scenario 2: Interactive Approval

**On manager machine**:
```bash
family-policy manager add-client \
  --host 192.168.1.100 \
  --name "Kids-PC"

# Output:
# Connecting to 192.168.1.100:8745...
# ✓ Connected
# ⏳ Waiting for user approval on client...
#    (User must approve this connection on Kids-PC)
```

**On agent machine (notification shown)**:
```
┌─────────────────────────────────────────────┐
│  Family Policy Agent                        │
├─────────────────────────────────────────────┤
│  Manager "Admin's Laptop" wants to connect  │
│  IP: 192.168.1.50                           │
│                                             │
│  Allow this manager to push policies?       │
│                                             │
│      [Approve]          [Deny]              │
└─────────────────────────────────────────────┘
```

**After user clicks Approve**:
```
# Manager output continues:
# ✓ Registration approved by user
# ✓ Client "Kids-PC" added successfully
```

### Policy Push Experience

```bash
# Push to single client
family-policy manager push \
  --client "Kids-PC" \
  --config ./browser-policy.yaml

# Output:
# Validating policy...
# ✓ Policy syntax valid
# ✓ Extension IDs valid
#
# Connecting to Kids-PC (192.168.1.100)...
# ✓ Connected
# ✓ Authenticated
#
# Pushing policy (hash: a1b2c3...)
# ⏳ Applying policy on client...
#
# ✓ Policy applied successfully
#
# Applied configuration:
#   Chrome: 2 extensions, incognito disabled
#   Firefox: 1 extension, private browsing disabled
#   Edge: not configured

# Push to multiple clients
family-policy manager push \
  --client "Kids-PC" \
  --client "Living-Room-Mac" \
  --config ./browser-policy.yaml

# Output:
# Validating policy...
# ✓ Policy syntax valid
#
# Pushing to 2 clients...
#
# [1/2] Kids-PC (192.168.1.100)
#   ✓ Connected
#   ✓ Policy applied successfully
#
# [2/2] Living-Room-Mac (192.168.1.101)
#   ✓ Connected
#   ✓ Policy applied successfully
#
# Summary: 2 successful, 0 failed

# Push with dry-run
family-policy manager push \
  --client "Kids-PC" \
  --config ./browser-policy.yaml \
  --dry-run

# Output:
# ✓ Policy would be applied (validation passed)
# No changes made (dry-run mode)
```

### Status Check Experience

```bash
family-policy manager status --client "Kids-PC"

# Output:
# Client: Kids-PC
# Host: 192.168.1.100
# Status: ✓ Online
# OS: Windows 11 Home (22H2)
# Agent Version: 1.0.0
# Last Contact: 2 minutes ago
#
# Current Policy:
#   Hash: a1b2c3d4...
#   Applied: 2025-10-29 15:45:00
#   Applied By: Admin's Laptop
#
# Applied Configuration:
#   Chrome:
#     Extensions: 2 installed
#       - uBlock Origin Lite (ddkjiahejlhfcafbddmgiahcphecmpfh)
#       - HTTPS Everywhere (gcbommkclmclpchllfjekcdonpmejbdp)
#     Incognito Mode: Disabled
#     Guest Mode: Enabled
#
#   Firefox:
#     Extensions: 1 installed
#       - uBlock Origin Lite (uBOLite@raymondhill.net)
#     Private Browsing: Disabled
#
#   Edge:
#     Not configured

# Check all clients
family-policy manager status --all

# Output:
# Managed Clients (3):
#
# ✓ Kids-PC (192.168.1.100)
#   Last seen: 2 minutes ago
#   Policy: a1b2c3d4... (applied 2 hours ago)
#
# ✓ Living-Room-Mac (192.168.1.101)
#   Last seen: 5 minutes ago
#   Policy: a1b2c3d4... (applied 2 hours ago)
#
# ✗ Basement-Linux (192.168.1.102)
#   Last seen: 3 days ago
#   Status: Offline
#   Policy: e5f6g7h8... (applied 5 days ago)
```

### Uninstall Experience

```bash
# Remove policies but keep agent running
family-policy manager uninstall --client "Kids-PC"

# Output:
# Connecting to Kids-PC...
# ✓ Connected
#
# This will remove all browser policies from Kids-PC.
# The agent will continue running and can receive new policies.
#
# Continue? [y/N]: y
#
# ⏳ Removing policies...
# ✓ Chrome policies removed
# ✓ Firefox policies removed
# ✓ Edge policies removed
#
# ✓ All policies removed successfully
# Agent is still running and can receive new policies.

# Remove client from manager's list
family-policy manager remove-client --client "Kids-PC"

# Output:
# This will remove Kids-PC from your managed clients list.
# It will NOT remove the agent or policies from the client.
#
# To remove policies first, run:
#   family-policy manager uninstall --client "Kids-PC"
#
# Continue? [y/N]: y
# ✓ Client "Kids-PC" removed from manager
```

---

## Component 8: Security Best Practices

### For Users

**Documented Recommendations**:

1. **Network Isolation**:
   - Run agent only on trusted local network
   - Consider using firewall to restrict agent port to local network only
   ```bash
   # Example: Linux iptables
   sudo iptables -A INPUT -p tcp --dport 8745 -s 192.168.1.0/24 -j ACCEPT
   sudo iptables -A INPUT -p tcp --dport 8745 -j DROP
   ```

2. **Registration Security**:
   - Always use tokens or interactive approval
   - Don't share registration tokens over insecure channels (email, SMS)
   - Revoke manager access if device is lost/stolen
   ```bash
   family-policy agent list-managers
   family-policy agent revoke --manager <id>
   ```

3. **Regular Reviews**:
   - Periodically check approved managers
   - Review applied policies
   - Check agent logs for suspicious activity

4. **Updates**:
   - Keep agent updated (check for updates monthly)
   - Subscribe to security announcements

### In Implementation

1. **Rate Limiting**:
   ```rust
   // Per-IP connection limits
   const MAX_CONNECTIONS_PER_MINUTE: u32 = 10;
   const MAX_AUTH_ATTEMPTS: u32 = 3;
   ```

2. **Input Validation**:
   - Validate all incoming messages against schema
   - Reject oversized messages (max 1 MB)
   - Sanitize all string inputs

3. **Timing Attack Prevention**:
   ```rust
   // Constant-time comparison for signatures
   use subtle::ConstantTimeEq;
   if signature.ct_eq(&expected).into() {
       // Valid
   }
   ```

4. **Secure Defaults**:
   - Agent requires approval by default
   - Registration tokens expire quickly (15 min)
   - Session tokens short-lived (5 min)
   - TLS 1.3 only, strong cipher suites

5. **Logging (No Secrets)**:
   - Log connection attempts, but not keys/tokens
   - Log authentication results, not challenge/response values
   - Rotate logs (max 10 MB)

---

## Component 9: Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_registration_token_generation() {
        let token = generate_registration_token();
        assert!(token.is_valid());
        assert!(token.expires_at > Utc::now());
    }

    #[test]
    fn test_signature_verification() {
        let keypair = generate_keypair();
        let challenge = generate_challenge();
        let signature = sign_challenge(&challenge, &keypair.private);
        assert!(verify_signature(&challenge, &signature, &keypair.public));
    }

    #[test]
    fn test_policy_hash_consistency() {
        let config = Config::from_file("test-policy.yaml").unwrap();
        let hash1 = compute_hash(&config);
        let hash2 = compute_hash(&config);
        assert_eq!(hash1, hash2);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_registration_flow() {
    // Start mock agent
    let agent = start_test_agent().await;

    // Manager attempts registration
    let result = register_client(&agent.host, &agent.port, Some(agent.token)).await;

    assert!(result.is_ok());
    let client_info = result.unwrap();
    assert_eq!(client_info.name, "test-agent");
}

#[tokio::test]
async fn test_push_policy_flow() {
    let (agent, manager) = setup_test_environment().await;

    let config = Config::from_file("test-policy.yaml").unwrap();
    let result = push_policy(&manager, &agent.id, &config).await;

    assert!(result.is_ok());

    // Verify policy applied on agent
    let state = agent.get_state().await;
    assert_eq!(state.current_policy.hash, compute_hash(&config));
}
```

### Manual Testing Checklist

- [ ] Agent installs and starts on all platforms
- [ ] Registration works with token
- [ ] Registration works with interactive approval
- [ ] Policy push succeeds and applies correctly
- [ ] Policy push is idempotent (same policy = no changes)
- [ ] Status check returns accurate information
- [ ] Uninstall removes all policies
- [ ] Manager can manage multiple clients
- [ ] Connection fails gracefully when agent offline
- [ ] Authentication fails when using revoked manager
- [ ] Logs contain useful information (no secrets)

---

## Component 10: Implementation Roadmap

### Phase 1: Core Infrastructure (2-3 weeks)
- [ ] Project restructuring (agent/manager modules)
- [ ] Crypto primitives (keypair generation, signing)
- [ ] State file management (agent-state.json, manager-state.json)
- [ ] Protocol message definitions
- [ ] Basic TLS connection handling

### Phase 2: Agent Implementation (2-3 weeks)
- [ ] Agent listener (TCP socket with TLS)
- [ ] Registration handler (token and interactive)
- [ ] Authentication (challenge-response)
- [ ] Policy application (reuse existing logic)
- [ ] Status reporting
- [ ] Platform-specific service installation
- [ ] Interactive approval notifications (per OS)

### Phase 3: Manager Implementation (2 weeks)
- [ ] CLI commands (add-client, push, status, etc.)
- [ ] Client registry management
- [ ] Policy validation before push
- [ ] Push operation with error handling
- [ ] Multi-client push support
- [ ] Colored terminal output

### Phase 4: Testing & Refinement (2 weeks)
- [ ] Unit tests for crypto/protocol
- [ ] Integration tests (agent ↔ manager)
- [ ] Cross-platform testing
- [ ] Error handling improvements
- [ ] User experience polish
- [ ] Performance testing (latency, memory)

### Phase 5: Documentation & Packaging (1-2 weeks)
- [ ] User guide (installation, registration, usage)
- [ ] Security guide (best practices)
- [ ] Troubleshooting guide
- [ ] Installer packages (DEB, RPM, PKG, MSI)
- [ ] Installation scripts

**Total Timeline**: 9-12 weeks (2-3 months)

---

## Component 11: Future Enhancements (Post-v1)

### v1.1: Convenience Features
- [ ] Auto-discovery of agents on local network (mDNS/Bonjour)
- [ ] `--host auto` to discover and show available agents
- [ ] Policy templates (pre-defined configs for common scenarios)
- [ ] Policy diff before push (show what will change)

### v1.2: Monitoring
- [ ] Agent reports periodic heartbeats (configurable)
- [ ] Manager can query "are policies still applied?" (detect drift)
- [ ] Email/webhook notifications for policy failures
- [ ] Simple web dashboard (optional, run locally)

### v1.3: Advanced Management
- [ ] Policy groups (assign same policy to multiple clients)
- [ ] Policy scheduling (apply at specific time)
- [ ] Rollback support (keep last N policies)
- [ ] Remote agent updates (push new binary)

---

## Comparison with Original Design

| Aspect | Original Design | Simplified Design |
|--------|----------------|-------------------|
| **Architecture** | Always-running server + polling agents | CLI push + listening agents |
| **Database** | PostgreSQL | Local JSON files |
| **Deployment** | Server infrastructure required | Just install binary on admin's laptop |
| **Communication** | Pull (agents poll server) | Push (admin pushes to agents) |
| **Auth** | mTLS with CA infrastructure | Simple public key exchange |
| **Complexity** | High (server, DB, web UI) | Low (single binary, CLI only) |
| **Scale** | Thousands of machines | Dozens to hundreds |
| **Enterprise Features** | Audit logs, RBAC, HA | Basic logs, single admin |
| **Target Audience** | Enterprise IT | Home users, small business |

---

## Conclusion

This design provides a **practical, secure, home-friendly** remote management solution:

✅ Single binary (agent + manager in one)
✅ No database or always-running server
✅ Push model (admin-initiated)
✅ Works on home editions (Windows Home, regular macOS/Linux)
✅ Simple security (public key exchange, like SSH)
✅ User-approved registration
✅ Lightweight and efficient
✅ Reuses existing policy application logic

Perfect for managing a family's computers or a small business with a few dozen machines.
