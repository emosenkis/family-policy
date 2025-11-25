use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::browser;
use crate::cli::Args;
use crate::config;
use crate::core;
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

    // Note: Privilege checking is now done in main.rs before this function is called

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

    // Load current state for diff comparison
    let current_state = state::load_state().context("Failed to load state")?;

    // Show diff preview in dry-run mode
    if args.dry_run {
        println!("DRY RUN MODE - No changes will be made");
        println!();

        let diff = core::diff::generate_diff(&config, current_state.as_ref());
        core::diff::print_diff(&diff);

        return Ok(());
    }

    // Apply policies using core module
    println!("Applying browser policies...");
    println!();

    let result = core::apply::apply_policies_from_config(&config, args.dry_run)
        .context("Failed to apply policies")?;

    // Display results
    if !result.changed {
        println!("✓ No changes detected - configuration matches current state");
        println!("  All policies are already applied as configured.");
    } else {
        println!();
        println!("✓ All policies applied successfully");
        println!("  State saved to: {}", state::get_state_path()?.display());

        println!();
        println!("Summary:");
        println!("  Chrome: {} extensions, {} privacy settings",
            result.extensions_applied.chrome,
            result.privacy_settings_applied.chrome);
        println!("  Firefox: {} extensions, {} privacy settings",
            result.extensions_applied.firefox,
            result.privacy_settings_applied.firefox);
        println!("  Edge: {} extensions, {} privacy settings",
            result.extensions_applied.edge,
            result.privacy_settings_applied.edge);
    }

    if !result.errors.is_empty() {
        println!();
        println!("Errors encountered:");
        for error in &result.errors {
            eprintln!("  - {}", error);
        }
    }

    if !result.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &result.warnings {
            println!("  - {}", warning);
        }
    }

    Ok(())
}

fn uninstall_policies(dry_run: bool) -> Result<()> {
    println!("Uninstalling browser policies...");
    println!();

    // Use core module for removal
    let result = core::apply::remove_all_policies(dry_run)
        .context("Failed to remove policies")?;

    if dry_run {
        println!("DRY RUN MODE - No changes will be made");
        println!();
        println!("Would remove:");
        println!("  Chrome: {} extensions, {} privacy settings",
            result.extensions_removed.chrome,
            result.privacy_settings_removed.chrome);
        println!("  Firefox: {} extensions, {} privacy settings",
            result.extensions_removed.firefox,
            result.privacy_settings_removed.firefox);
        println!("  Edge: {} extensions, {} privacy settings",
            result.extensions_removed.edge,
            result.privacy_settings_removed.edge);
    } else {
        println!();
        println!("✓ All policies removed successfully");
        println!();
        println!("Removed:");
        println!("  Chrome: {} extensions, {} privacy settings",
            result.extensions_removed.chrome,
            result.privacy_settings_removed.chrome);
        println!("  Firefox: {} extensions, {} privacy settings",
            result.extensions_removed.firefox,
            result.privacy_settings_removed.firefox);
        println!("  Edge: {} extensions, {} privacy settings",
            result.extensions_removed.edge,
            result.privacy_settings_removed.edge);
    }

    if !result.errors.is_empty() {
        println!();
        println!("Errors encountered:");
        for error in &result.errors {
            eprintln!("  - {}", error);
        }
    }

    Ok(())
}

// Note: print_summary function removed - now using diff output and apply result
