use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::browser;
use crate::cli::Args;
use crate::config;
use crate::platform;
use crate::policy;
use crate::state;

/// Local mode arguments
pub struct LocalArgs {
    pub config: PathBuf,
    pub uninstall: bool,
    pub dry_run: bool,
    pub verbose: bool,
}

impl From<Args> for LocalArgs {
    fn from(args: Args) -> Self {
        Self {
            config: args.config,
            uninstall: args.uninstall,
            dry_run: args.dry_run,
            verbose: args.verbose,
        }
    }
}

/// Run in local mode (backward compatibility)
pub fn run_local_mode(args: Args) -> Result<()> {
    let args = LocalArgs::from(args);
    run_local(args)
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
