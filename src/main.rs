use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod agent;
mod browser;
mod config;
mod platform;
mod policy;
mod state;

/// Browser Extension Policy Manager
///
/// Manages browser extension force-install policies and privacy controls
/// for Chrome, Firefox, and Edge across Windows, macOS, and Linux.
#[derive(Parser, Debug)]
#[command(name = "family-policy")]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to configuration file (for local mode)
    #[arg(short, long, default_value = "browser-policy.yaml", global = true)]
    config: PathBuf,

    /// Remove all policies created by this tool (for local mode)
    #[arg(short, long, global = true)]
    uninstall: bool,

    /// Show what would be done without making changes (for local mode)
    #[arg(short = 'n', long, global = true)]
    dry_run: bool,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Agent commands for remote policy management
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
}

#[derive(Subcommand, Debug)]
enum AgentCommands {
    /// Setup agent configuration
    Setup {
        /// Raw GitHub URL to policy file
        #[arg(long)]
        url: String,

        /// GitHub Personal Access Token (for private repos)
        #[arg(long)]
        token: Option<String>,

        /// Polling interval in seconds
        #[arg(long, default_value = "300")]
        poll_interval: u64,
    },
    /// Start agent daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        no_daemon: bool,
    },
    /// Check for policy updates now (don't wait for next poll)
    CheckNow,
    /// Show agent status
    Status,
    /// Show currently applied configuration
    ShowConfig,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands
    if let Some(command) = args.command {
        return match command {
            Commands::Agent { command } => run_agent_command(command, args.verbose),
        };
    }

    // No subcommand: run in local mode (backward compatibility)
    run_local_mode(args)
}

/// Run agent subcommands
fn run_agent_command(command: AgentCommands, verbose: bool) -> Result<()> {
    // Initialize logging
    init_logging(verbose);

    match command {
        AgentCommands::Setup { url, token, poll_interval } => {
            agent_setup(url, token, poll_interval)
        }
        AgentCommands::Start { no_daemon } => {
            agent_start(no_daemon)
        }
        AgentCommands::CheckNow => {
            agent_check_now()
        }
        AgentCommands::Status => {
            agent_status()
        }
        AgentCommands::ShowConfig => {
            agent_show_config()
        }
    }
}

/// Initialize logging
fn init_logging(verbose: bool) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let level = if verbose { "debug" } else { "info" };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(level)))
        .init();
}

/// Setup agent configuration
fn agent_setup(url: String, token: Option<String>, poll_interval: u64) -> Result<()> {
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
    println!("     sudo family-policy agent start");
    println!();
    println!("The agent will check for policy updates every {} seconds.", poll_interval);

    Ok(())
}

/// Start agent daemon
fn agent_start(no_daemon: bool) -> Result<()> {
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
            .context("Failed to load agent configuration. Run 'family-policy agent setup' first.")?;

        // Run agent
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            agent::run_agent_daemon(config).await
        })
    } else {
        // TODO: Implement proper daemonization for each platform
        anyhow::bail!("Daemon mode not yet implemented. Use --no-daemon to run in foreground.");
    }
}

/// Check for policy updates now
fn agent_check_now() -> Result<()> {
    // Check for admin privileges
    if let Err(e) = platform::ensure_admin_privileges() {
        eprintln!("Insufficient privileges: {:#}", e);
        print_sudo_message();
        std::process::exit(1);
    }

    println!("Checking for policy updates...");

    let config_path = agent::get_agent_config_path()?;
    let config = agent::AgentConfig::load(&config_path)
        .context("Failed to load agent configuration. Run 'family-policy agent setup' first.")?;

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
fn agent_status() -> Result<()> {
    println!("Family Policy Agent Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load configuration
    let config_path = agent::get_agent_config_path()?;
    let config = agent::AgentConfig::load(&config_path)
        .context("Agent not configured. Run 'family-policy agent setup' first.")?;

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
fn agent_show_config() -> Result<()> {
    println!("Current Policy Configuration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load state
    let state = agent::AgentState::load()?
        .context("No policy applied yet. Run 'family-policy agent check-now' to apply policy.")?;

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

/// Format duration for display
fn format_duration(duration: chrono::Duration) -> String {
    let secs = duration.num_seconds();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

/// Print sudo message based on OS
fn print_sudo_message() {
    #[cfg(unix)]
    eprintln!("Please run with sudo: sudo {}", std::env::args().next().unwrap());

    #[cfg(windows)]
    eprintln!("Please run this program as Administrator.");
}

/// Run in local mode (backward compatibility)
fn run_local_mode(args: Args) -> Result<()> {
    let args = LocalArgs {
        config: args.config,
        uninstall: args.uninstall,
        dry_run: args.dry_run,
        verbose: args.verbose,
    };

    run_local(args)
}

/// Local mode arguments
struct LocalArgs {
    config: PathBuf,
    uninstall: bool,
    dry_run: bool,
    verbose: bool,
}

fn run_local(args: LocalArgs) -> Result<()> {
    // Print header
    println!("Browser Extension Policy Manager v{}", env!("CARGO_PKG_VERSION"));
    println!("Platform: {}", browser::current_platform().name());
    println!();

    // Check for admin privileges
    if !args.dry_run {
        if let Err(e) = platform::ensure_admin_privileges() {
            eprintln!("Insufficient privileges: {:#}", e);
            eprintln!();
            eprintln!("This program requires administrator/root privileges to modify system policies.");

            #[cfg(unix)]
            eprintln!("Please run with sudo: sudo {}", std::env::args().next().unwrap());

            #[cfg(windows)]
            eprintln!("Please run this program as Administrator.");

            std::process::exit(1);
        }
    }

    if args.uninstall {
        // Uninstall mode: Remove all policies
        uninstall_policies(args.dry_run)?;
    } else {
        // Install mode: Apply policies from config
        install_policies(&args)?;
    }

    Ok(())
}

fn install_policies(args: &LocalArgs) -> Result<()> {
    // Load configuration
    println!("Loading configuration from: {}", args.config.display());

    let config = config::load_config(&args.config)
        .context("Failed to load configuration file")?;

    if args.verbose {
        println!("Configuration loaded successfully");
        println!("  - {} policies configured", config.policies.len());
        for policy in &config.policies {
            println!("    - {}: {} browsers", policy.name, policy.browsers.len());
        }
    }

    println!();

    // Load current state
    let current_state = state::load_state().context("Failed to load state")?;

    // Check if config has changed
    let config_hash = state::compute_config_hash(&config)?;
    let needs_update = match &current_state {
        Some(state) => state.config_hash != config_hash,
        None => true,
    };

    if !needs_update && !args.dry_run {
        println!("✓ No changes detected - configuration matches current state");
        println!("  All policies are already applied as configured.");
        return Ok(());
    }

    if args.dry_run {
        println!("DRY RUN MODE - No changes will be made");
        println!();
    }

    // Apply policies
    println!("Applying browser policies...");
    println!();

    if !args.dry_run {
        let applied_policies = policy::apply_policies(&config, current_state.as_ref())
            .context("Failed to apply policies")?;

        // Create new state
        let new_state = state::create_state(&config, applied_policies)
            .context("Failed to create state")?;

        // Save state
        state::save_state(&new_state).context("Failed to save state")?;

        println!();
        println!("✓ All policies applied successfully");
        println!("  State saved to: {}", state::get_state_path()?.display());
    } else {
        // Dry run: just show what would be done
        let (chrome, firefox, edge) = config::to_browser_configs(&config);
        if chrome.is_some() {
            println!("[DRY RUN] Would apply Chrome policies");
        }
        if firefox.is_some() {
            println!("[DRY RUN] Would apply Firefox policies");
        }
        if edge.is_some() {
            println!("[DRY RUN] Would apply Edge policies");
        }
    }

    println!();
    println!("Summary:");
    print_summary(&config);

    Ok(())
}

fn uninstall_policies(dry_run: bool) -> Result<()> {
    println!("Uninstalling browser policies...");
    println!();

    // Load current state
    let state = match state::load_state().context("Failed to load state")? {
        Some(state) => state,
        None => {
            println!("No policies currently installed (state file not found)");
            return Ok(());
        }
    };

    if dry_run {
        println!("DRY RUN MODE - No changes will be made");
        println!();

        if state.applied_policies.chrome.is_some() {
            println!("[DRY RUN] Would remove Chrome policies");
        }
        if state.applied_policies.firefox.is_some() {
            println!("[DRY RUN] Would remove Firefox policies");
        }
        if state.applied_policies.edge.is_some() {
            println!("[DRY RUN] Would remove Edge policies");
        }

        return Ok(());
    }

    // Remove policies
    policy::remove_policies(&state).context("Failed to remove policies")?;

    // Delete state file
    state::delete_state().context("Failed to delete state file")?;

    println!();
    println!("✓ All policies removed successfully");

    Ok(())
}

fn print_summary(config: &config::Config) {
    let (chrome, firefox, edge) = config::to_browser_configs(config);

    if let Some(chrome) = chrome {
        println!("  Chrome:");
        println!("    Extensions: {}", chrome.extensions.len());
        if chrome.disable_incognito == Some(true) {
            println!("    Incognito mode: DISABLED");
        }
        if chrome.disable_guest_mode == Some(true) {
            println!("    Guest mode: DISABLED");
        }
    }

    if let Some(firefox) = firefox {
        println!("  Firefox:");
        println!("    Extensions: {}", firefox.extensions.len());
        if firefox.disable_private_browsing == Some(true) {
            println!("    Private browsing: DISABLED");
        }
    }

    if let Some(edge) = edge {
        println!("  Edge:");
        println!("    Extensions: {}", edge.extensions.len());
        if edge.disable_inprivate == Some(true) {
            println!("    InPrivate mode: DISABLED");
        }
        if edge.disable_guest_mode == Some(true) {
            println!("    Guest mode: DISABLED");
        }
    }
}
