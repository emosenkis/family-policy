use anyhow::Result;
use crate::core::privileges;

/// Run the Admin UI
///
/// TODO: Phase 4 - Implement full Admin UI functionality
pub fn run() -> Result<()> {
    // Verify admin privileges at startup
    if !privileges::is_admin() {
        return Err(anyhow::anyhow!(
            "Admin UI requires administrator privileges.\n\
             Please run with sudo (Linux/macOS) or as Administrator (Windows)."
        ));
    }

    eprintln!("Admin UI is not yet implemented.");
    eprintln!("This feature will be available in Phase 4 of the implementation.");
    eprintln!();
    eprintln!("For now, please use the existing CLI commands:");
    eprintln!("  - 'family-policy apply --config <file>' to apply policies");
    eprintln!("  - 'family-policy apply --config <file> --dry-run' to preview changes");
    eprintln!("  - 'family-policy --uninstall' to remove all policies");

    anyhow::bail!("Admin UI not yet implemented")
}
