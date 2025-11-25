use anyhow::Result;

/// Run the User UI
///
/// # Arguments
/// * `systray_mode` - If true, run in system tray mode; if false, show window
///
/// Launches the Tauri UI application in User mode (no admin privileges required).
/// The UI defaults to showing the User Status view.
pub fn run(_systray_mode: bool) -> Result<()> {
    // TODO: Implement systray vs window mode distinction
    // For now, always launch the full UI with tray icon
    super::run()
}
