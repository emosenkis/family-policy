#[cfg(target_os = "windows")]
use anyhow::{Context, Result};

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

/// Registry value types
#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub enum RegistryValue {
    Dword(u32),
    String(String),
}

/// Write numbered registry values (for extension lists)
///
/// Opens or creates a registry key and writes numbered values (1, 2, 3, ...)
/// This is used for policies like ExtensionInstallForcelist
#[cfg(target_os = "windows")]
pub fn write_registry_policy(key_path: &str, values: Vec<String>) -> Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // Open or create the key
    let (key, _) = hklm
        .create_subkey(key_path)
        .with_context(|| format!("Failed to create registry key: HKLM\\{}", key_path))?;

    // First, delete all existing numbered values (for idempotency)
    let mut i = 1u32;
    loop {
        match key.delete_value(i.to_string()) {
            Ok(_) => i += 1,
            Err(_) => break, // No more values to delete
        }
    }

    // Write new values
    for (index, value) in values.iter().enumerate() {
        let value_name = (index + 1).to_string();
        key.set_value(&value_name, value).with_context(|| {
            format!(
                "Failed to set registry value: HKLM\\{}\\{}",
                key_path, value_name
            )
        })?;
    }

    Ok(())
}

/// Write a single named registry value
///
/// Used for privacy control policies like IncognitoModeAvailability
#[cfg(target_os = "windows")]
pub fn write_registry_value(
    key_path: &str,
    value_name: &str,
    value: RegistryValue,
) -> Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // Open or create the key
    let (key, _) = hklm
        .create_subkey(key_path)
        .with_context(|| format!("Failed to create registry key: HKLM\\{}", key_path))?;

    // Write the value
    match value {
        RegistryValue::Dword(val) => {
            key.set_value(value_name, &val).with_context(|| {
                format!(
                    "Failed to set DWORD value: HKLM\\{}\\{}",
                    key_path, value_name
                )
            })?;
        }
        RegistryValue::String(val) => {
            key.set_value(value_name, &val).with_context(|| {
                format!(
                    "Failed to set String value: HKLM\\{}\\{}",
                    key_path, value_name
                )
            })?;
        }
    }

    Ok(())
}

/// Remove a registry key and all its subkeys
#[cfg(target_os = "windows")]
pub fn remove_registry_policy(key_path: &str) -> Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    match hklm.delete_subkey_all(key_path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Key doesn't exist - this is fine (idempotent)
            Ok(())
        }
        Err(e) => Err(e).with_context(|| format!("Failed to delete registry key: HKLM\\{}", key_path)),
    }
}

/// Remove a single named value from a registry key
#[cfg(target_os = "windows")]
pub fn remove_registry_value(key_path: &str, value_name: &str) -> Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    match hklm.open_subkey_with_flags(key_path, KEY_WRITE) {
        Ok(key) => {
            match key.delete_value(value_name) {
                Ok(_) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // Value doesn't exist - this is fine (idempotent)
                    Ok(())
                }
                Err(e) => Err(e).with_context(|| {
                    format!(
                        "Failed to delete registry value: HKLM\\{}\\{}",
                        key_path, value_name
                    )
                }),
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Key doesn't exist - this is fine (idempotent)
            Ok(())
        }
        Err(e) => Err(e).with_context(|| format!("Failed to open registry key: HKLM\\{}", key_path)),
    }
}

/// Read numbered registry values
#[cfg(target_os = "windows")]
pub fn read_registry_policy(key_path: &str) -> Result<Vec<String>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    match hklm.open_subkey(key_path) {
        Ok(key) => {
            let mut values = Vec::new();
            let mut i = 1u32;

            loop {
                match key.get_value::<String, _>(i.to_string()) {
                    Ok(value) => {
                        values.push(value);
                        i += 1;
                    }
                    Err(_) => break, // No more values
                }
            }

            Ok(values)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Key doesn't exist - return empty list
            Ok(Vec::new())
        }
        Err(e) => Err(e).with_context(|| format!("Failed to read registry key: HKLM\\{}", key_path)),
    }
}

/// Apply registry policy with dry-run support
/// Shows diff in dry-run mode, actually writes in normal mode
#[cfg(target_os = "windows")]
pub fn apply_registry_policy_with_preview(
    key_path: &str,
    values: Vec<String>,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        println!("Registry Key: HKLM\\{}", key_path);
        let current_values = read_registry_policy(key_path).unwrap_or_default();

        if current_values.is_empty() && !values.is_empty() {
            println!("  Action: CREATE new policy");
            for (i, ext) in values.iter().enumerate() {
                println!("  + [{}] {}", i + 1, ext);
            }
        } else if current_values != values {
            println!("  Action: UPDATE policy");
            for (i, value) in current_values.iter().enumerate() {
                if !values.contains(value) {
                    println!("  - [{}] {}", i + 1, value);
                }
            }
            for (i, value) in values.iter().enumerate() {
                if !current_values.contains(value) {
                    println!("  + [{}] {}", i + 1, value);
                } else if current_values.get(i) == Some(value) {
                    println!("    [{}] {}", i + 1, value);
                }
            }
        } else {
            println!("  Action: No changes needed");
        }
        println!();
        Ok(())
    } else {
        write_registry_policy(key_path, values)
    }
}

/// Apply registry value with dry-run support
/// Shows value in dry-run mode, actually writes in normal mode
#[cfg(target_os = "windows")]
pub fn apply_registry_value_with_preview(
    key_path: &str,
    value_name: &str,
    value: RegistryValue,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        println!("Registry Value: HKLM\\{}\\{}", key_path, value_name);
        println!("  Action: SET value");
        match &value {
            RegistryValue::Dword(val) => {
                println!("  + Type: DWORD");
                println!("  + Value: {}", val);
            }
            RegistryValue::String(val) => {
                println!("  + Type: String");
                println!("  + Value: {}", val);
            }
        }
        println!();
        Ok(())
    } else {
        write_registry_value(key_path, value_name, value)
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;

    // Note: These tests require admin privileges and will modify the registry
    // They should be run carefully in a test environment

    #[test]
    #[ignore] // Ignore by default to avoid modifying registry during normal test runs
    fn test_write_and_read_registry_policy() {
        let test_key = r"SOFTWARE\BrowserPolicyTest\TestKey";
        let values = vec![
            "test1;https://example.com".to_string(),
            "test2;https://example.org".to_string(),
        ];

        // Write
        write_registry_policy(test_key, values.clone()).unwrap();

        // Read
        let read_values = read_registry_policy(test_key).unwrap();
        assert_eq!(values, read_values);

        // Clean up
        remove_registry_policy(test_key).unwrap();
    }

    #[test]
    #[ignore]
    fn test_write_and_remove_registry_value() {
        let test_key = r"SOFTWARE\BrowserPolicyTest\TestValue";

        // Write DWORD
        write_registry_value(test_key, "TestDword", RegistryValue::Dword(1)).unwrap();

        // Write String
        write_registry_value(test_key, "TestString", RegistryValue::String("test".to_string())).unwrap();

        // Remove
        remove_registry_value(test_key, "TestDword").unwrap();
        remove_registry_value(test_key, "TestString").unwrap();
        remove_registry_policy(test_key).unwrap();
    }
}
