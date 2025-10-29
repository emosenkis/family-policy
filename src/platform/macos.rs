#[cfg(target_os = "macos")]
use anyhow::{Context, Result};
#[cfg(target_os = "macos")]
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use plist::Value;

/// Write or update a plist file with multiple key-value pairs
///
/// Creates or updates a plist at /Library/Managed Preferences/{bundle_id}.plist
/// Preserves existing keys that are not in the updates map
#[cfg(target_os = "macos")]
pub fn write_plist_policy(bundle_id: &str, updates: HashMap<String, Value>) -> Result<()> {
    let plist_path = get_plist_path(bundle_id)?;

    // Read existing plist if it exists
    let mut existing_dict = if plist_path.exists() {
        let file = std::fs::File::open(&plist_path)
            .with_context(|| format!("Failed to open plist file: {}", plist_path.display()))?;

        match plist::from_reader(file) {
            Ok(Value::Dictionary(dict)) => dict,
            Ok(_) => {
                // Not a dictionary, start fresh
                plist::Dictionary::new()
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to parse existing plist at {}: {}. Creating new plist.",
                    plist_path.display(),
                    e
                );
                plist::Dictionary::new()
            }
        }
    } else {
        plist::Dictionary::new()
    };

    // Merge updates into existing dictionary
    for (key, value) in updates {
        existing_dict.insert(key, value);
    }

    // Ensure parent directory exists
    if let Some(parent) = plist_path.parent() {
        crate::platform::common::ensure_directory_exists(parent)?;
    }

    // Write plist
    let value = Value::Dictionary(existing_dict);
    let file = std::fs::File::create(&plist_path)
        .with_context(|| format!("Failed to create plist file: {}", plist_path.display()))?;

    plist::to_writer_xml(file, &value)
        .with_context(|| format!("Failed to write plist file: {}", plist_path.display()))?;

    // Set permissions
    crate::platform::common::set_permissions_readable_all(&plist_path)?;

    Ok(())
}

/// Remove specific keys from a plist
///
/// If all keys are removed and the plist becomes empty, delete the file
#[cfg(target_os = "macos")]
pub fn remove_plist_keys(bundle_id: &str, keys: &[String]) -> Result<()> {
    let plist_path = get_plist_path(bundle_id)?;

    if !plist_path.exists() {
        // Nothing to do (idempotent)
        return Ok(());
    }

    // Read existing plist
    let file = std::fs::File::open(&plist_path)
        .with_context(|| format!("Failed to open plist file: {}", plist_path.display()))?;

    let mut dict = match plist::from_reader(file) {
        Ok(Value::Dictionary(dict)) => dict,
        Ok(_) => {
            // Not a dictionary, just delete the file
            std::fs::remove_file(&plist_path)?;
            return Ok(());
        }
        Err(e) => {
            return Err(e).with_context(|| {
                format!("Failed to parse plist file: {}", plist_path.display())
            });
        }
    };

    // Remove specified keys
    for key in keys {
        dict.remove(key);
    }

    // If dictionary is now empty, delete the file
    if dict.is_empty() {
        std::fs::remove_file(&plist_path)
            .with_context(|| format!("Failed to delete plist file: {}", plist_path.display()))?;
    } else {
        // Write updated plist
        let value = Value::Dictionary(dict);
        let file = std::fs::File::create(&plist_path)
            .with_context(|| format!("Failed to create plist file: {}", plist_path.display()))?;

        plist::to_writer_xml(file, &value)
            .with_context(|| format!("Failed to write plist file: {}", plist_path.display()))?;
    }

    Ok(())
}

/// Delete an entire plist file
#[cfg(target_os = "macos")]
pub fn remove_plist(bundle_id: &str) -> Result<()> {
    let plist_path = get_plist_path(bundle_id)?;

    if plist_path.exists() {
        std::fs::remove_file(&plist_path)
            .with_context(|| format!("Failed to delete plist file: {}", plist_path.display()))?;
    }

    Ok(())
}

/// Get the path to a managed preferences plist
#[cfg(target_os = "macos")]
fn get_plist_path(bundle_id: &str) -> Result<PathBuf> {
    let mut path = PathBuf::from("/Library/Managed Preferences");
    path.push(format!("{}.plist", bundle_id));
    Ok(path)
}

/// Helper to create a plist array from a vector of strings
#[cfg(target_os = "macos")]
pub fn string_vec_to_plist_array(strings: Vec<String>) -> Value {
    Value::Array(
        strings
            .into_iter()
            .map(Value::String)
            .collect()
    )
}

/// Helper to create a plist integer
#[cfg(target_os = "macos")]
pub fn integer_to_plist(val: i64) -> Value {
    Value::Integer(val.into())
}

/// Helper to create a plist boolean
#[cfg(target_os = "macos")]
pub fn bool_to_plist(val: bool) -> Value {
    Value::Boolean(val)
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;

    #[test]
    fn test_string_vec_to_plist_array() {
        let strings = vec!["test1".to_string(), "test2".to_string()];
        let array = string_vec_to_plist_array(strings);

        match array {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 2);
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_plist_conversions() {
        let int_val = integer_to_plist(42);
        assert!(matches!(int_val, Value::Integer(_)));

        let bool_val = bool_to_plist(true);
        assert!(matches!(bool_val, Value::Boolean(true)));
    }

    #[test]
    #[ignore] // Requires root privileges
    fn test_write_and_remove_plist() {
        let bundle_id = "com.test.browser-policy";
        let mut updates = HashMap::new();
        updates.insert(
            "TestKey".to_string(),
            Value::String("TestValue".to_string()),
        );

        write_plist_policy(bundle_id, updates).unwrap();
        remove_plist(bundle_id).unwrap();
    }
}
