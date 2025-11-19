use anyhow::{Context, Result};
use std::path::PathBuf;
use std::fs;

use crate::cli::ConfigCommands;
use crate::config::EXAMPLE_CONFIG;

/// Initialize a new configuration file
pub fn init(output: PathBuf, force: bool, _verbose: bool) -> Result<()> {
    println!("Family Policy - Configuration Initialization");
    println!();

    // Check if file already exists
    if output.exists() && !force {
        anyhow::bail!(
            "Configuration file already exists at: {}\n\
             Use --force to overwrite, or specify a different --output path",
            output.display()
        );
    }

    // Write the example config to the file
    fs::write(&output, EXAMPLE_CONFIG)
        .with_context(|| format!("Failed to write configuration file to {}", output.display()))?;

    println!("✓ Created example configuration file: {}", output.display());
    println!();
    println!("Next steps:");
    println!("  1. Edit the configuration file to match your needs");
    println!("  2. Test your configuration:");
    println!("     family-policy --config {} --dry-run", output.display());
    println!("  3. Apply the configuration:");
    println!("     sudo family-policy --config {}", output.display());
    println!();

    Ok(())
}
