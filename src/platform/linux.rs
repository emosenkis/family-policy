#[cfg(target_os = "linux")]
use anyhow::{Context, Result};
#[cfg(target_os = "linux")]
use std::path::Path;

/// Write a JSON policy file
///
/// Creates a JSON file in the policy directory with the given data
/// Typically used for Chrome/Edge policies at /etc/opt/chrome/policies/managed
#[cfg(target_os = "linux")]
pub fn write_json_policy(
    policy_dir: &Path,
    policy_name: &str,
    data: serde_json::Value,
) -> Result<()> {
    // Ensure policy directory exists
    crate::platform::common::ensure_directory_exists(policy_dir)?;

    // Build file path
    let mut policy_path = policy_dir.to_path_buf();
    policy_path.push(format!("{}.json", policy_name));

    // Serialize JSON with pretty printing
    let content = serde_json::to_string_pretty(&data)
        .context("Failed to serialize JSON policy")?;

    // Write atomically
    crate::platform::common::atomic_write(&policy_path, content.as_bytes())
        .with_context(|| format!("Failed to write policy file: {}", policy_path.display()))?;

    // Set permissions (readable by all)
    crate::platform::common::set_permissions_readable_all(&policy_path)?;

    Ok(())
}

/// Read a JSON policy file
///
/// Returns None if the file doesn't exist
#[cfg(target_os = "linux")]
pub fn read_json_policy(
    policy_dir: &Path,
    policy_name: &str,
) -> Result<Option<serde_json::Value>> {
    let mut policy_path = policy_dir.to_path_buf();
    policy_path.push(format!("{}.json", policy_name));

    if !policy_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&policy_path)
        .with_context(|| format!("Failed to read policy file: {}", policy_path.display()))?;

    let data: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse policy file: {}", policy_path.display()))?;

    Ok(Some(data))
}

/// Remove a JSON policy file
///
/// Also removes the policy directory if it becomes empty
#[cfg(target_os = "linux")]
pub fn remove_json_policy(policy_dir: &Path, policy_name: &str) -> Result<()> {
    let mut policy_path = policy_dir.to_path_buf();
    policy_path.push(format!("{}.json", policy_name));

    if policy_path.exists() {
        std::fs::remove_file(&policy_path)
            .with_context(|| format!("Failed to delete policy file: {}", policy_path.display()))?;
    }

    // Try to remove the directory if it's empty
    if policy_dir.exists() {
        if let Ok(mut entries) = std::fs::read_dir(policy_dir) {
            if entries.next().is_none() {
                // Directory is empty, remove it
                let _ = std::fs::remove_dir(policy_dir);
            }
        }
    }

    Ok(())
}

/// Get Chrome policy directory path
#[cfg(target_os = "linux")]
pub fn get_chrome_policy_dir() -> &'static Path {
    Path::new("/etc/opt/chrome/policies/managed")
}

/// Get Chromium policy directory path
#[cfg(target_os = "linux")]
pub fn get_chromium_policy_dir() -> &'static Path {
    Path::new("/etc/chromium/policies/managed")
}

/// Get Edge policy directory path
#[cfg(target_os = "linux")]
pub fn get_edge_policy_dir() -> &'static Path {
    Path::new("/etc/opt/microsoft/edge/policies/managed")
}

/// Get Firefox policy directory path
#[cfg(target_os = "linux")]
pub fn get_firefox_policy_dir() -> &'static Path {
    Path::new("/etc/firefox/policies")
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_policy_dir_paths() {
        assert_eq!(
            get_chrome_policy_dir(),
            Path::new("/etc/opt/chrome/policies/managed")
        );
        assert_eq!(
            get_edge_policy_dir(),
            Path::new("/etc/opt/microsoft/edge/policies/managed")
        );
        assert_eq!(
            get_firefox_policy_dir(),
            Path::new("/etc/firefox/policies")
        );
    }

    #[test]
    #[ignore] // Requires root privileges
    fn test_write_read_remove_json_policy() {
        let test_dir = Path::new("/tmp/browser-policy-test");
        let policy_name = "test-policy";
        let data = json!({
            "TestKey": "TestValue",
            "TestNumber": 42
        });

        // Write
        write_json_policy(test_dir, policy_name, data.clone()).unwrap();

        // Read
        let read_data = read_json_policy(test_dir, policy_name).unwrap();
        assert_eq!(Some(data), read_data);

        // Remove
        remove_json_policy(test_dir, policy_name).unwrap();

        // Verify removed
        let after_remove = read_json_policy(test_dir, policy_name).unwrap();
        assert_eq!(None, after_remove);
    }
}
