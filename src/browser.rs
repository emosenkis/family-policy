use serde::{Deserialize, Serialize};

/// Supported browsers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    Chrome,
    Firefox,
    Edge,
}

impl Browser {
    /// Get lowercase string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Browser::Chrome => "chrome",
            Browser::Firefox => "firefox",
            Browser::Edge => "edge",
        }
    }
}

/// Supported platforms
///
/// Note: On each platform, only the current platform variant is constructed,
/// but all variants are needed for match expressions in policy modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Not all variants constructed on every platform
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

impl Platform {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Platform::Windows => "Windows",
            Platform::MacOS => "macOS",
            Platform::Linux => "Linux",
        }
    }
}

/// Get the current platform
pub fn current_platform() -> Platform {
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }

    #[cfg(target_os = "macos")]
    {
        Platform::MacOS
    }

    #[cfg(target_os = "linux")]
    {
        Platform::Linux
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        compile_error!("Unsupported operating system");
    }
}

#[cfg(test)]
mod test_helpers {
    use super::*;
    use std::path::PathBuf;

/// Check if a browser is available on the system
pub fn is_browser_available(browser: Browser) -> bool {
    match browser {
        Browser::Chrome => is_chrome_available(),
        Browser::Firefox => is_firefox_available(),
        Browser::Edge => is_edge_available(),
    }
}

/// Check if Chrome is installed
fn is_chrome_available() -> bool {
    get_chrome_paths().iter().any(|p| p.exists())
}

/// Check if Firefox is installed
fn is_firefox_available() -> bool {
    get_firefox_paths().iter().any(|p| p.exists())
}

/// Check if Edge is installed
fn is_edge_available() -> bool {
    get_edge_paths().iter().any(|p| p.exists())
}

/// Get possible Chrome installation paths
pub fn get_chrome_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            PathBuf::from(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"),
        ]
    }

    #[cfg(target_os = "macos")]
    {
        vec![PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        )]
    }

    #[cfg(target_os = "linux")]
    {
        vec![
            PathBuf::from("/usr/bin/google-chrome"),
            PathBuf::from("/usr/bin/google-chrome-stable"),
            PathBuf::from("/usr/bin/chromium"),
            PathBuf::from("/usr/bin/chromium-browser"),
        ]
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        vec![]
    }
}

/// Get possible Firefox installation paths
pub fn get_firefox_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            PathBuf::from(r"C:\Program Files\Mozilla Firefox\firefox.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe"),
        ]
    }

    #[cfg(target_os = "macos")]
    {
        vec![PathBuf::from(
            "/Applications/Firefox.app/Contents/MacOS/firefox",
        )]
    }

    #[cfg(target_os = "linux")]
    {
        vec![
            PathBuf::from("/usr/bin/firefox"),
            PathBuf::from("/usr/bin/firefox-esr"),
        ]
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        vec![]
    }
}

/// Get possible Edge installation paths
pub fn get_edge_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            PathBuf::from(r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"),
        ]
    }

    #[cfg(target_os = "macos")]
    {
        vec![PathBuf::from(
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        )]
    }

    #[cfg(target_os = "linux")]
    {
        vec![
            PathBuf::from("/usr/bin/microsoft-edge"),
            PathBuf::from("/usr/bin/microsoft-edge-stable"),
        ]
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        vec![]
    }
}
} // end of test_helpers module

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_helpers::*;

    #[test]
    fn test_browser_clone() {
        let browser = Browser::Chrome;
        let cloned = browser.clone();
        assert_eq!(browser, cloned);
    }

    #[test]
    fn test_browser_copy() {
        let browser = Browser::Firefox;
        let copied = browser;
        assert_eq!(browser, copied);
    }

    #[test]
    fn test_browser_equality() {
        assert_eq!(Browser::Chrome, Browser::Chrome);
        assert_ne!(Browser::Chrome, Browser::Firefox);
        assert_ne!(Browser::Firefox, Browser::Edge);
    }

    #[test]
    fn test_browser_debug() {
        let browser = Browser::Chrome;
        let debug_str = format!("{:?}", browser);
        assert!(debug_str.contains("Chrome"));
    }

    // Platform tests
    #[test]
    fn test_platform_name() {
        assert_eq!(Platform::Windows.name(), "Windows");
        assert_eq!(Platform::MacOS.name(), "macOS");
        assert_eq!(Platform::Linux.name(), "Linux");
    }

    #[test]
    fn test_current_platform() {
        let platform = current_platform();
        // Verify it returns one of the supported platforms
        assert!(matches!(
            platform,
            Platform::Windows | Platform::MacOS | Platform::Linux
        ));
    }

    #[test]
    fn test_current_platform_matches_compile_target() {
        let platform = current_platform();

        #[cfg(target_os = "windows")]
        assert_eq!(platform, Platform::Windows);

        #[cfg(target_os = "macos")]
        assert_eq!(platform, Platform::MacOS);

        #[cfg(target_os = "linux")]
        assert_eq!(platform, Platform::Linux);
    }

    #[test]
    fn test_platform_clone() {
        let platform = Platform::Linux;
        let cloned = platform.clone();
        assert_eq!(platform, cloned);
    }

    #[test]
    fn test_platform_equality() {
        assert_eq!(Platform::Windows, Platform::Windows);
        assert_ne!(Platform::Windows, Platform::Linux);
        assert_ne!(Platform::MacOS, Platform::Linux);
    }

    #[test]
    fn test_platform_debug() {
        let platform = Platform::MacOS;
        let debug_str = format!("{:?}", platform);
        assert!(debug_str.contains("MacOS"));
    }

    // Browser availability tests
    #[test]
    fn test_is_browser_available_returns_bool() {
        // Just verify the function is callable and returns a bool
        let _ = is_browser_available(Browser::Chrome);
        let _ = is_browser_available(Browser::Firefox);
        let _ = is_browser_available(Browser::Edge);
    }

    #[test]
    fn test_get_chrome_paths_not_empty() {
        let paths = get_chrome_paths();
        assert!(!paths.is_empty(), "Chrome paths should not be empty");
    }

    #[test]
    fn test_get_firefox_paths_not_empty() {
        let paths = get_firefox_paths();
        assert!(!paths.is_empty(), "Firefox paths should not be empty");
    }

    #[test]
    fn test_get_edge_paths_not_empty() {
        let paths = get_edge_paths();
        assert!(!paths.is_empty(), "Edge paths should not be empty");
    }

    #[test]
    fn test_chrome_paths_are_absolute() {
        let paths = get_chrome_paths();
        for path in paths {
            assert!(path.is_absolute(), "Chrome path should be absolute: {:?}", path);
        }
    }

    #[test]
    fn test_firefox_paths_are_absolute() {
        let paths = get_firefox_paths();
        for path in paths {
            assert!(path.is_absolute(), "Firefox path should be absolute: {:?}", path);
        }
    }

    #[test]
    fn test_edge_paths_are_absolute() {
        let paths = get_edge_paths();
        for path in paths {
            assert!(path.is_absolute(), "Edge path should be absolute: {:?}", path);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_chrome_paths_include_common_locations() {
        let paths = get_chrome_paths();
        let path_strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

        assert!(path_strs.iter().any(|p| p.contains("google-chrome")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_firefox_paths_include_firefox() {
        let paths = get_firefox_paths();
        let path_strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

        assert!(path_strs.iter().any(|p| p.contains("firefox")));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_paths_include_program_files() {
        let chrome_paths = get_chrome_paths();
        let chrome_strs: Vec<String> = chrome_paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

        assert!(chrome_strs.iter().any(|p| p.contains("Program Files")));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_paths_include_applications() {
        let chrome_paths = get_chrome_paths();
        let chrome_strs: Vec<String> = chrome_paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

        assert!(chrome_strs.iter().any(|p| p.contains("/Applications/")));
    }
}
