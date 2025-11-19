use chrono::Duration;

/// Initialize logging
pub fn init_logging(verbose: bool) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let level = if verbose { "debug" } else { "info" };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(level)))
        .init();
}

/// Format duration for display
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.num_seconds();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

/// Print sudo message based on OS
pub fn print_sudo_message() {
    #[cfg(unix)]
    eprintln!("Please run with sudo: sudo {}", std::env::args().next().unwrap());

    #[cfg(windows)]
    eprintln!("Please run this program as Administrator.");
}
