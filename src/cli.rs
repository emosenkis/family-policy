use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Browser Extension Policy Manager
///
/// Manages browser extension force-install policies and privacy controls
/// for Chrome, Firefox, and Edge across Windows, macOS, and Linux.
#[derive(Parser, Debug)]
#[command(name = "family-policy")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to configuration file (for local mode)
    #[arg(short, long, default_value = "browser-policy.yaml", global = true)]
    pub config: PathBuf,

    /// Remove all policies created by this tool (for local mode)
    #[arg(short, long, global = true)]
    pub uninstall: bool,

    /// Show what would be done without making changes (for local mode)
    #[arg(short = 'n', long, global = true)]
    pub dry_run: bool,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Configuration file management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Install agent as a system service
    InstallService,
    /// Uninstall agent system service
    UninstallService,
    /// Start agent daemon
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
