use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Browser Extension Policy Manager
///
/// Manages browser extension force-install policies and privacy controls
/// for Chrome, Firefox, and Edge across Windows, macOS, and Linux.
///
/// If no subcommand is specified, runs 'apply' by default.
#[derive(Parser, Debug)]
#[command(name = "family-policy")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path or HTTPS URL to policy YAML
    #[arg(short, long, default_value = "browser-policy.yaml", global = true)]
    pub config: String,

    /// Remove all policies created by this tool
    #[arg(short, long, global = true)]
    pub uninstall: bool,

    /// Show what would be done without making changes
    #[arg(short = 'n', long, global = true)]
    pub dry_run: bool,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Reapply policies even when the saved state hash matches the config
    #[arg(long, global = true)]
    pub force_apply: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Apply policies from a configuration file or HTTPS URL (default command)
    Apply,
    /// Configuration file management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Run as daemon (foreground mode)
    Daemon,
    /// Install agent as a system service
    InstallService,
    /// Uninstall agent system service
    UninstallService,
    /// Start agent daemon (background)
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        no_daemon: bool,
    },
    /// Stop agent daemon
    Stop,
    /// Check for policy updates now (don't wait for next poll)
    CheckNow,
    /// Show agent status
    Status,
    /// Show currently applied configuration
    ShowConfig,
    /// Configure the polling agent
    Setup {
        #[arg(long)]
        url: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long, default_value_t = 300)]
        poll_interval: u64,
        /// Overwrite an existing agent configuration
        #[arg(long)]
        force: bool,
    },
    #[cfg(windows)]
    #[command(hide = true)]
    Service,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Initialize a new configuration file with examples
    Init {
        /// Output path for the configuration file
        #[arg(short, long, default_value = "family-policy.yaml")]
        output: PathBuf,

        /// Overwrite existing file if it exists
        #[arg(short, long)]
        force: bool,
    },
}
