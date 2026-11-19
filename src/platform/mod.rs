/// Common cross-platform utilities
pub mod common;

/// Windows-specific operations (registry)
#[cfg(target_os = "windows")]
pub mod windows;

/// macOS-specific operations (plist)
#[cfg(any(target_os = "macos", test))]
pub mod macos;

/// Linux-specific operations (JSON policies)
#[cfg(target_os = "linux")]
pub mod linux;

// Re-export common utilities for convenience
pub use common::*;
