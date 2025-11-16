use anyhow::{Context, Result};

use crate::agent;
use crate::platform;

use super::utils::{format_duration, init_logging, print_sudo_message};

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
        // For Windows, use sc.exe to create service
        println!("Creating Windows Service...");

        // Get binary path
        let current_exe = std::env::current_exe()?;
        let bin_path = current_exe.to_string_lossy().to_string();

        // Service configuration
        let service_name = "FamilyPolicyAgent";
        let display_name = "Family Policy Agent";
        let description = "Browser Extension Policy Management - Automatically manages browser policies via GitHub polling";

        // Create service with sc.exe
        // binPath must include the full command with arguments
        let bin_path_with_args = format!("\"{}\" start --no-daemon", bin_path);

        let output = std::process::Command::new("sc.exe")
            .args(&["create", service_name])
            .arg(format!("binPath= {}", bin_path_with_args))
            .arg("start= auto")
            .arg(format!("DisplayName= {}", display_name))
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!("Failed to create service:\n{}\n{}", stdout, error);
        }

        println!("✓ Service created");

        // Set service description
        let _ = std::process::Command::new("sc.exe")
            .args(&["description", service_name, description])
            .output();

        // Configure service recovery options (restart on failure)
        let _ = std::process::Command::new("sc.exe")
            .args(&["failure", service_name, "reset= 86400", "actions= restart/10000/restart/10000/restart/10000"])
            .output();

        println!("✓ Service recovery configured");
        println!();
        println!("Service installed successfully!");
        println!();
        println!("To start the service:");
        println!("  family-policy start");
        println!();
        println!("To check status:");
        println!("  family-policy status");
        println!("  sc.exe query {}", service_name);
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
        let service_name = "FamilyPolicyAgent";

        // Stop service first
        println!("Stopping service...");
        let output = std::process::Command::new("sc.exe")
            .args(&["stop", service_name])
            .output()?;

        // Don't fail if service is already stopped
        if output.status.success() {
            // Wait a moment for service to fully stop
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        // Delete the service
        println!("Removing service...");
        let output = std::process::Command::new("sc.exe")
            .args(&["delete", service_name])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Check if error is just "service doesn't exist"
            if stdout.contains("does not exist") || stdout.contains("1060") {
                println!("Service was not installed");
            } else {
                println!("Warning: Failed to remove service:\n{}\n{}", stdout, error);
            }
        } else {
            println!("✓ Service removed");
        }
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
            let service_name = "FamilyPolicyAgent";

            println!("Starting Windows Service...");
            let output = std::process::Command::new("sc.exe")
                .args(&["start", service_name])
                .output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Check if error is "service doesn't exist"
                if stdout.contains("does not exist") || stdout.contains("1060") {
                    anyhow::bail!("Service not installed. Run 'family-policy install-service' first.");
                } else if stdout.contains("already started") || stdout.contains("1056") {
                    println!("Service is already running");
                } else {
                    anyhow::bail!("Failed to start service:\n{}\n{}", stdout, error);
                }
            } else {
                println!("✓ Service started successfully");
            }

            println!();
            println!("To check status:");
            println!("  family-policy status");
            println!("  sc.exe query {}", service_name);

            Ok(())
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
        let service_name = "FamilyPolicyAgent";

        let output = std::process::Command::new("sc.exe")
            .args(&["stop", service_name])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Check if error is "service doesn't exist" or "not started"
            if stdout.contains("does not exist") || stdout.contains("1060") {
                println!("Service is not installed");
                println!();
                println!("If you started the agent manually, press Ctrl+C to stop it.");
            } else if stdout.contains("not started") || stdout.contains("1062") {
                println!("Service is not running");
            } else {
                anyhow::bail!("Failed to stop service:\n{}\n{}", stdout, error);
            }
        } else {
            println!("✓ Service stopped successfully");
        }
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
