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
    /// Time limits management
    TimeLimits {
        #[command(subcommand)]
        command: TimeLimitsCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum TimeLimitsCommands {
    /// Initialize time limits configuration
    Init {
        /// Output path for the configuration file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite existing file if it exists
        #[arg(short, long)]
        force: bool,
    },
    /// Add a child profile
    AddChild {
        /// Child ID (unique identifier)
        #[arg(long)]
        id: String,

        /// Child's name
        #[arg(long)]
        name: String,

        /// OS usernames for this child (comma-separated)
        #[arg(long)]
        os_users: Option<String>,

        /// Weekday time limit in hours
        #[arg(long)]
        weekday_hours: u32,

        /// Weekend time limit in hours
        #[arg(long)]
        weekend_hours: u32,
    },
    /// Show current time limits status
    Status,
    /// Grant time extension (admin)
    GrantExtension {
        /// Child ID
        child_id: String,

        /// Additional minutes to grant
        minutes: u32,

        /// Admin password
        #[arg(long)]
        password: String,

        /// Reason for extension
        #[arg(long)]
        reason: Option<String>,
    },
    /// Reset a child's time for today (admin)
    ResetTime {
        /// Child ID
        child_id: String,

        /// Admin password
        #[arg(long)]
        password: String,
    },
    /// Set admin password
    SetPassword {
        /// New admin password
        password: String,
    },
    /// Show usage history
    History {
        /// Child ID
        child_id: String,

        /// Number of days to show
        #[arg(short, long, default_value = "7")]
        days: u32,
    },
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
