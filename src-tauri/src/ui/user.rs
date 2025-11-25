use anyhow::Result;

/// Run the User UI
///
/// # Arguments
/// * `systray_mode` - If true, run in system tray mode; if false, show window
///
/// TODO: Phase 3 - Implement full User UI functionality
pub fn run(_systray_mode: bool) -> Result<()> {
    eprintln!("User UI is not yet implemented.");
    eprintln!("This feature will be available in Phase 3 of the implementation.");
    eprintln!();
    eprintln!("For now, please use the existing CLI commands:");
    eprintln!("  - 'family-policy status' to view current status");
    eprintln!("  - 'family-policy show-config' to see applied configuration");

    anyhow::bail!("User UI not yet implemented")
}
