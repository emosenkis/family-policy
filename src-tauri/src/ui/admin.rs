use anyhow::Result;
use crate::core::privileges;

/// Run the Admin UI
///
/// Launches the Tauri UI application in Admin mode.
/// Requires administrator privileges to save configuration changes.
pub fn run() -> Result<()> {
    // Note: Admin privileges are checked within individual Tauri commands
    // The UI will show warnings if not running as admin
    if !privileges::is_admin() {
        eprintln!("Warning: Not running with administrator privileges.");
        eprintln!("You can view settings but cannot save changes.");
        eprintln!("To enable saving, run with sudo (Linux/macOS) or as Administrator (Windows).");
        eprintln!();
    }

    super::run()
}
