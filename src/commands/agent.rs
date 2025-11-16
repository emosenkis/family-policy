use anyhow::{Context, Result};

use crate::agent;
use crate::config;
use crate::platform;

use super::utils::{format_duration, init_logging, print_sudo_message};

/// Setup agent configuration
pub fn setup(url: String, token: Option<String>, poll_interval: u64, verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);
    // Check for admin privileges
    if let Err(e) = platform::ensure_admin_privileges() {
        eprintln!("Insufficient privileges: {:#}", e);
        print_sudo_message();
        std::process::exit(1);
    }

    println!("Browser Extension Policy Manager - Agent Setup");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    // Create configuration
    let config = agent::AgentConfig {
        github: agent::GitHubConfig {
            policy_url: url.clone(),
            access_token: token,
        },
        agent: agent::AgentSettings {
            poll_interval,
            poll_jitter: 60,
            retry_interval: 60,
            max_retries: 3,
        },
        logging: agent::LoggingConfig {
            level: "info".to_string(),
            file: None,
        },
        security: agent::SecurityConfig::default(),
    };

    // Validate configuration
    config.validate().context("Invalid configuration")?;

    println!("Testing connection to GitHub...");

    // Test connection synchronously
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let poller = agent::GitHubPoller::new(config.github.clone())?;
        match poller.fetch_policy(None).await {
            Ok(result) => {
                match result {
                    agent::PolicyFetchResult::Updated { content, .. } => {
                        // Parse to validate
                        let _policy: config::Config = serde_yaml::from_str(&content)
                            .context("Policy file is not valid YAML")?;
                        println!("✓ Policy file found and valid");
                    }
                    agent::PolicyFetchResult::NotModified => {
                        println!("✓ Policy file found");
                    }
                }
                Ok::<(), anyhow::Error>(())
            }
            Err(e) => {
                Err(e).context("Failed to fetch policy from GitHub")
            }
        }
    })?;

    // Save configuration
    let config_path = agent::get_agent_config_path()?;
    config.save(&config_path)?;
    println!("✓ Configuration saved to: {}", config_path.display());

    println!();
    println!("Agent configured successfully!");
    println!();
    println!("Next steps:");
    println!("  1. Start the agent:");
    println!("     sudo family-policy start");
    println!();
    println!("The agent will check for policy updates every {} seconds.", poll_interval);

    Ok(())
}

/// Install agent as a system service
pub fn install_service(verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);
    // Check for admin privileges
    if let Err(e) = platform::ensure_admin_privileges() {
        eprintln!("Insufficient privileges: {:#}", e);
        print_sudo_message();
        std::process::exit(1);
    }

    println!("Installing Family Policy Agent as a system service");
    println!();

    #[cfg(target_os = "linux")]
    {
        // For Linux, use systemctl
        println!("Enabling systemd service...");

        let output = std::process::Command::new("systemctl")
            .arg("enable")
            .arg("family-policy-agent")
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to enable service: {}", error);
        }

        println!("✓ Service enabled");
        println!();
        println!("Service installed successfully!");
        println!();
        println!("To start the service:");
        println!("  sudo systemctl start family-policy-agent");
        println!();
        println!("To check status:");
        println!("  sudo systemctl status family-policy-agent");
    }

    #[cfg(target_os = "macos")]
    {
        // For macOS, use launchctl
        println!("Loading LaunchDaemon...");

        let plist_path = "/Library/LaunchDaemons/com.family-policy.agent.plist";
        let output = std::process::Command::new("launchctl")
            .arg("load")
            .arg(plist_path)
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to load LaunchDaemon: {}", error);
        }

        println!("✓ LaunchDaemon loaded");
        println!();
        println!("Service installed and started successfully!");
        println!();
        println!("To check status:");
        println!("  sudo launchctl list | grep family-policy");
        println!("  sudo family-policy status");
    }

    #[cfg(target_os = "windows")]
    {
        println!("Windows Service installation is not yet implemented.");
        println!();
        println!("You can run the agent manually:");
        println!("  family-policy start --no-daemon");
        println!();
        println!("Or use Task Scheduler to run it at startup.");
    }

    Ok(())
}

/// Uninstall agent system service
pub fn uninstall_service(verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);
    // Check for admin privileges
    if let Err(e) = platform::ensure_admin_privileges() {
        eprintln!("Insufficient privileges: {:#}", e);
        print_sudo_message();
        std::process::exit(1);
    }

    println!("Uninstalling Family Policy Agent service");
    println!();

    #[cfg(target_os = "linux")]
    {
        // Stop service first
        let _ = std::process::Command::new("systemctl")
            .arg("stop")
            .arg("family-policy-agent")
            .output();

        // Disable service
        println!("Disabling systemd service...");
        let output = std::process::Command::new("systemctl")
            .arg("disable")
            .arg("family-policy-agent")
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("Warning: Failed to disable service: {}", error);
        } else {
            println!("✓ Service disabled");
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Unload LaunchDaemon
        println!("Unloading LaunchDaemon...");
        let plist_path = "/Library/LaunchDaemons/com.family-policy.agent.plist";
        let output = std::process::Command::new("launchctl")
            .arg("unload")
            .arg(plist_path)
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("Warning: Failed to unload LaunchDaemon: {}", error);
        } else {
            println!("✓ LaunchDaemon unloaded");
        }
    }

    #[cfg(target_os = "windows")]
    {
        println!("Windows Service is not installed.");
    }

    println!();
    println!("Service uninstalled successfully!");

    Ok(())
}

/// Start agent daemon
pub fn start(no_daemon: bool, verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);
    // Check for admin privileges
    if let Err(e) = platform::ensure_admin_privileges() {
        eprintln!("Insufficient privileges: {:#}", e);
        print_sudo_message();
        std::process::exit(1);
    }

    if no_daemon {
        // Run in foreground
        println!("Starting agent in foreground mode...");
        println!("Press Ctrl+C to stop");
        println!();

        let config_path = agent::get_agent_config_path()?;
        let config = agent::AgentConfig::load(&config_path)
            .context("Failed to load agent configuration. Run 'family-policy setup' first.")?;

        // Run agent
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            agent::run_agent_daemon(config).await
        })
    } else {
        // Use system service instead of manual daemonization
        #[cfg(target_os = "linux")]
        {
            println!("Starting systemd service...");
            let output = std::process::Command::new("systemctl")
                .arg("start")
                .arg("family-policy-agent")
                .output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Failed to start service: {}\n\nHint: Have you run 'sudo family-policy install-service'?", error);
            }

            println!("✓ Service started successfully");
            println!();
            println!("To check status:");
            println!("  sudo systemctl status family-policy-agent");
            println!("  sudo family-policy status");

            Ok(())
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, the LaunchDaemon should auto-start when loaded
            println!("The agent runs as a LaunchDaemon on macOS.");
            println!();
            println!("If the daemon is not running, load it with:");
            println!("  sudo launchctl load /Library/LaunchDaemons/com.family-policy.agent.plist");
            println!();
            println!("To check status:");
            println!("  sudo launchctl list | grep family-policy");
            println!("  sudo family-policy status");

            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            anyhow::bail!("Daemon mode not yet implemented on Windows. Use --no-daemon to run in foreground.");
        }
    }
}

/// Stop agent daemon
pub fn stop(verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);
    // Check for admin privileges
    if let Err(e) = platform::ensure_admin_privileges() {
        eprintln!("Insufficient privileges: {:#}", e);
        print_sudo_message();
        std::process::exit(1);
    }

    println!("Stopping Family Policy Agent");
    println!();

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("systemctl")
            .arg("stop")
            .arg("family-policy-agent")
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to stop service: {}", error);
        }

        println!("✓ Service stopped successfully");
    }

    #[cfg(target_os = "macos")]
    {
        let plist_path = "/Library/LaunchDaemons/com.family-policy.agent.plist";
        let output = std::process::Command::new("launchctl")
            .arg("unload")
            .arg(plist_path)
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to stop LaunchDaemon: {}", error);
        }

        println!("✓ LaunchDaemon stopped successfully");
        println!();
        println!("To start it again:");
        println!("  sudo launchctl load {}", plist_path);
    }

    #[cfg(target_os = "windows")]
    {
        println!("No Windows Service is running.");
        println!("If you started the agent manually, press Ctrl+C to stop it.");
    }

    Ok(())
}

/// Check for policy updates now
pub fn check_now(verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);
    // Check for admin privileges
    if let Err(e) = platform::ensure_admin_privileges() {
        eprintln!("Insufficient privileges: {:#}", e);
        print_sudo_message();
        std::process::exit(1);
    }

    println!("Checking for policy updates...");

    let config_path = agent::get_agent_config_path()?;
    let config = agent::AgentConfig::load(&config_path)
        .context("Failed to load agent configuration. Run 'family-policy setup' first.")?;

    let runtime = tokio::runtime::Runtime::new()?;
    let applied = runtime.block_on(async {
        agent::check_and_apply_once(&config).await
    })?;

    if applied {
        println!("✓ Policy updated and applied successfully");
    } else {
        println!("✓ Policy unchanged");
    }

    Ok(())
}

/// Show agent status
pub fn status(verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);
    println!("Family Policy Agent Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load configuration
    let config_path = agent::get_agent_config_path()?;
    let config = agent::AgentConfig::load(&config_path)
        .context("Agent not configured. Run 'family-policy setup' first.")?;

    println!("Policy URL:  {}", config.github.policy_url);
    println!("Poll Interval: {} seconds", config.agent.poll_interval);

    // Load state
    match agent::AgentState::load()? {
        Some(state) => {
            println!();
            if let Some(last_checked) = state.last_checked {
                let ago = chrono::Utc::now() - last_checked;
                println!("Last checked:  {} ({} ago)",
                    last_checked.format("%Y-%m-%d %H:%M:%S %Z"),
                    format_duration(ago));
            }

            if let Some(last_updated) = state.last_updated {
                let ago = chrono::Utc::now() - last_updated;
                println!("Last updated:  {} ({} ago)",
                    last_updated.format("%Y-%m-%d %H:%M:%S %Z"),
                    format_duration(ago));
            }

            if let Some(hash) = state.config_hash {
                println!("Current hash:  {}...", &hash[..16]);
            }

            // Show applied policies
            println!();
            println!("Applied Configuration:");
            if state.applied_policies.chrome.is_some() {
                let chrome = state.applied_policies.chrome.as_ref().unwrap();
                println!("  Chrome:     {} extensions", chrome.extensions.len());
            }
            if state.applied_policies.firefox.is_some() {
                let firefox = state.applied_policies.firefox.as_ref().unwrap();
                println!("  Firefox:    {} extensions", firefox.extensions.len());
            }
            if state.applied_policies.edge.is_some() {
                let edge = state.applied_policies.edge.as_ref().unwrap();
                println!("  Edge:       {} extensions", edge.extensions.len());
            }

            // Calculate next check time
            let scheduler = agent::PollingScheduler::new(
                config.agent.poll_interval,
                config.agent.poll_jitter
            );
            println!();
            println!("Next check:   ~{}", scheduler.next_poll_time().format("%Y-%m-%d %H:%M:%S %Z"));
        }
        None => {
            println!();
            println!("Status: Not yet run (no state file)");
        }
    }

    Ok(())
}

/// Show currently applied configuration
pub fn show_config(verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);
    println!("Current Policy Configuration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load state
    let state = agent::AgentState::load()?
        .context("No policy applied yet. Run 'family-policy check-now' to apply policy.")?;

    if let Some(last_updated) = state.last_updated {
        println!("Applied at: {}", last_updated.format("%Y-%m-%d %H:%M:%S %Z"));
    }

    if let Some(hash) = state.config_hash {
        println!("Hash: {}", hash);
    }

    println!();

    // Show applied policies
    let applied = state.applied_policies;

    if let Some(chrome) = applied.chrome {
        println!("Chrome:");
        println!("  Extensions:");
        for ext_id in &chrome.extensions {
            println!("    - {}", ext_id);
        }
        if let Some(disable) = chrome.disable_incognito {
            println!("  Incognito mode: {}", if disable { "DISABLED" } else { "enabled" });
        }
        if let Some(disable) = chrome.disable_guest_mode {
            println!("  Guest mode: {}", if disable { "DISABLED" } else { "enabled" });
        }
        println!();
    }

    if let Some(firefox) = applied.firefox {
        println!("Firefox:");
        println!("  Extensions:");
        for ext_id in &firefox.extensions {
            println!("    - {}", ext_id);
        }
        if let Some(disable) = firefox.disable_private_browsing {
            println!("  Private browsing: {}", if disable { "DISABLED" } else { "enabled" });
        }
        println!();
    }

    if let Some(edge) = applied.edge {
        println!("Edge:");
        println!("  Extensions:");
        for ext_id in &edge.extensions {
            println!("    - {}", ext_id);
        }
        if let Some(disable) = edge.disable_inprivate {
            println!("  InPrivate mode: {}", if disable { "DISABLED" } else { "enabled" });
        }
        if let Some(disable) = edge.disable_guest_mode {
            println!("  Guest mode: {}", if disable { "DISABLED" } else { "enabled" });
        }
        println!();
    }

    Ok(())
}
