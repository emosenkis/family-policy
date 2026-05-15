#[cfg(target_os = "macos")]
use anyhow::{Context, Result};
#[cfg(target_os = "macos")]
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use plist::Value;

#[cfg(target_os = "macos")]
const PROFILE_OUTPUT_DIR: &str = "/Library/Application Support/browser-extension-policy/profiles";

#[cfg(target_os = "macos")]
fn read_plist_dictionary(plist_path: &Path) -> Result<plist::Dictionary> {
    if !plist_path.exists() {
        return Ok(plist::Dictionary::new());
    }

    let file = std::fs::File::open(plist_path)
        .with_context(|| format!("Failed to open plist file: {}", plist_path.display()))?;

    match plist::from_reader(file) {
        Ok(Value::Dictionary(dict)) => Ok(dict),
        Ok(_) => Ok(plist::Dictionary::new()),
        Err(e) => {
            eprintln!(
                "Warning: Failed to parse existing plist at {}: {}. Creating new plist.",
                plist_path.display(),
                e
            );
            Ok(plist::Dictionary::new())
        }
    }
}

#[cfg(target_os = "macos")]
fn write_plist_dictionary(plist_path: &Path, dict: plist::Dictionary) -> Result<()> {
    if let Some(parent) = plist_path.parent() {
        crate::platform::common::ensure_directory_exists(parent)?;
    }

    let value = Value::Dictionary(dict);
    let file = std::fs::File::create(plist_path)
        .with_context(|| format!("Failed to create plist file: {}", plist_path.display()))?;

    plist::to_writer_xml(file, &value)
        .with_context(|| format!("Failed to write plist file: {}", plist_path.display()))?;

    crate::platform::common::set_permissions_readable_all(plist_path)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn write_plist_updates(plist_path: &Path, updates: HashMap<String, Value>) -> Result<()> {
    let mut existing_dict = read_plist_dictionary(plist_path)?;

    for (key, value) in updates {
        existing_dict.insert(key, value);
    }

    write_plist_dictionary(plist_path, existing_dict)
}

/// Write or update a plist file with multiple key-value pairs
///
/// Creates or updates a plist at /Library/Managed Preferences/{bundle_id}.plist
/// Preserves existing keys that are not in the updates map
#[cfg(target_os = "macos")]
pub fn write_plist_policy(bundle_id: &str, updates: HashMap<String, Value>) -> Result<()> {
    let plist_path = get_plist_path(bundle_id)?;
    write_plist_updates(&plist_path, updates)
}

/// Write or update a plist in /Library/Preferences.
///
/// Some browsers (notably Firefox on macOS) read command-line/defaults managed
/// policy values from this preferences domain instead of the Managed Preferences
/// file path used by Chrome-family browsers.
#[cfg(target_os = "macos")]
pub fn write_library_preferences_plist(
    bundle_id: &str,
    updates: HashMap<String, Value>,
) -> Result<()> {
    let plist_path = get_library_preferences_plist_path(bundle_id);
    write_plist_updates(&plist_path, updates)
}

/// Install a managed preferences configuration profile for browsers that rely on
/// macOS profiles to surface mandatory policies in their about:policies pages.
#[cfg(target_os = "macos")]
pub fn install_managed_preferences_profile(
    bundle_id: &str,
    display_name: &str,
    policy_settings: HashMap<String, Value>,
    dry_run: bool,
) -> Result<()> {
    let profile_path = get_profile_path(bundle_id);

    if dry_run {
        println!("Configuration Profile: {}", profile_path.display());
        println!("  Payload domain: {}", bundle_id);
        println!("  Display name: {}", display_name);
        println!("  Action: CREATE_OR_REPLACE managed preferences profile");
        for key in policy_settings.keys() {
            println!("  + Policy key: {}", key);
        }
        println!();
        return Ok(());
    }

    let profile = create_managed_preferences_profile(bundle_id, display_name, policy_settings);
    if let Some(parent) = profile_path.parent() {
        crate::platform::common::ensure_directory_exists(parent)?;
    }

    let file = std::fs::File::create(&profile_path).with_context(|| {
        format!(
            "Failed to create configuration profile: {}",
            profile_path.display()
        )
    })?;
    plist::to_writer_xml(file, &profile).with_context(|| {
        format!(
            "Failed to write configuration profile: {}",
            profile_path.display()
        )
    })?;
    crate::platform::common::set_permissions_readable_all(&profile_path)?;

    tracing::debug!(
        "Installing macOS managed preferences profile for {} from {}",
        bundle_id,
        profile_path.display()
    );

    // Re-applying a profile with an existing PayloadIdentifier can fail on some
    // macOS releases, so remove our previous profile first. Ignore failures: the
    // profile may not exist yet, and the install below will report real errors.
    let _ = remove_managed_preferences_profile(bundle_id);

    match std::process::Command::new("/usr/bin/profiles")
        .args([
            "install",
            "-type",
            "configuration",
            "-path",
            profile_path.to_string_lossy().as_ref(),
        ])
        .output()
    {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            tracing::warn!(
                "profiles install failed for {} (status: {}). stderr: {}. Retrying with legacy -I -F syntax.",
                bundle_id,
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let legacy_output = std::process::Command::new("/usr/bin/profiles")
                .args(["-I", "-F", profile_path.to_string_lossy().as_ref()])
                .output()
                .with_context(|| "Failed to run legacy profiles command")?;

            if legacy_output.status.success() {
                Ok(())
            } else {
                anyhow::bail!(
                    "Failed to install macOS configuration profile for {}. profiles stderr: {}; legacy stderr: {}",
                    bundle_id,
                    String::from_utf8_lossy(&output.stderr),
                    String::from_utf8_lossy(&legacy_output.stderr)
                );
            }
        }
        Err(e) => Err(e).with_context(|| "Failed to run profiles command"),
    }
}

/// Remove the configuration profile installed by install_managed_preferences_profile.
#[cfg(target_os = "macos")]
pub fn remove_managed_preferences_profile(bundle_id: &str) -> Result<()> {
    let profile_identifier = format!("com.family-policy.browser.{}", bundle_id);
    let output = std::process::Command::new("/usr/bin/profiles")
        .args(["remove", "-identifier", profile_identifier.as_str()])
        .output()
        .with_context(|| "Failed to run profiles remove command")?;

    if output.status.success() {
        tracing::debug!(
            "Removed macOS managed preferences profile {}",
            profile_identifier
        );
    } else {
        tracing::debug!(
            "macOS managed preferences profile {} was not removed (it may not exist): {}",
            profile_identifier,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Remove keys from a /Library/Preferences plist.
#[cfg(target_os = "macos")]
pub fn remove_library_preferences_plist_keys(bundle_id: &str, keys: &[String]) -> Result<()> {
    let plist_path = get_library_preferences_plist_path(bundle_id);
    remove_plist_keys_at_path(&plist_path, keys)
}

#[cfg(target_os = "macos")]
fn remove_plist_keys_at_path(plist_path: &Path, keys: &[String]) -> Result<()> {
    if !plist_path.exists() {
        return Ok(());
    }

    let file = std::fs::File::open(plist_path)
        .with_context(|| format!("Failed to open plist file: {}", plist_path.display()))?;

    let mut dict = match plist::from_reader(file) {
        Ok(Value::Dictionary(dict)) => dict,
        Ok(_) => {
            std::fs::remove_file(plist_path)?;
            return Ok(());
        }
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Failed to parse plist file: {}", plist_path.display()));
        }
    };

    for key in keys {
        dict.remove(key);
    }

    if dict.is_empty() {
        std::fs::remove_file(plist_path)
            .with_context(|| format!("Failed to delete plist file: {}", plist_path.display()))?;
    } else {
        write_plist_dictionary(plist_path, dict)?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn create_managed_preferences_profile(
    bundle_id: &str,
    display_name: &str,
    policy_settings: HashMap<String, Value>,
) -> Value {
    let profile_identifier = format!("com.family-policy.browser.{}", bundle_id);
    let payload_identifier = format!("{}.managed-preferences", profile_identifier);

    let mut mcx_settings = plist::Dictionary::new();
    for (key, value) in policy_settings {
        mcx_settings.insert(key, value);
    }

    let mut forced_entry = plist::Dictionary::new();
    forced_entry.insert(
        "mcx_preference_settings".to_string(),
        Value::Dictionary(mcx_settings),
    );

    let mut domain_payload = plist::Dictionary::new();
    domain_payload.insert(
        "Forced".to_string(),
        Value::Array(vec![Value::Dictionary(forced_entry)]),
    );

    let mut managed_content = plist::Dictionary::new();
    managed_content.insert(bundle_id.to_string(), Value::Dictionary(domain_payload));

    let mut managed_payload = plist::Dictionary::new();
    managed_payload.insert(
        "PayloadType".to_string(),
        Value::String("com.apple.ManagedClient.preferences".to_string()),
    );
    managed_payload.insert("PayloadVersion".to_string(), Value::Integer(1.into()));
    managed_payload.insert(
        "PayloadIdentifier".to_string(),
        Value::String(payload_identifier),
    );
    managed_payload.insert(
        "PayloadUUID".to_string(),
        Value::String(uuid::Uuid::new_v4().to_string()),
    );
    managed_payload.insert("PayloadEnabled".to_string(), Value::Boolean(true));
    managed_payload.insert(
        "PayloadDisplayName".to_string(),
        Value::String(format!("{} Managed Preferences", display_name)),
    );
    managed_payload.insert(
        "PayloadContent".to_string(),
        Value::Dictionary(managed_content),
    );

    let mut profile = plist::Dictionary::new();
    profile.insert(
        "PayloadType".to_string(),
        Value::String("Configuration".to_string()),
    );
    profile.insert("PayloadVersion".to_string(), Value::Integer(1.into()));
    profile.insert(
        "PayloadIdentifier".to_string(),
        Value::String(profile_identifier),
    );
    profile.insert(
        "PayloadUUID".to_string(),
        Value::String(uuid::Uuid::new_v4().to_string()),
    );
    profile.insert(
        "PayloadDisplayName".to_string(),
        Value::String(format!("Family Policy - {}", display_name)),
    );
    profile.insert(
        "PayloadDescription".to_string(),
        Value::String(format!(
            "Family Policy managed browser policies for {}",
            display_name
        )),
    );
    profile.insert(
        "PayloadOrganization".to_string(),
        Value::String("Family Policy".to_string()),
    );
    profile.insert(
        "PayloadContent".to_string(),
        Value::Array(vec![Value::Dictionary(managed_payload)]),
    );

    Value::Dictionary(profile)
}

#[cfg(target_os = "macos")]
fn get_library_preferences_plist_path(bundle_id: &str) -> PathBuf {
    let mut path = PathBuf::from("/Library/Preferences");
    path.push(format!("{}.plist", bundle_id));
    path
}

#[cfg(target_os = "macos")]
fn get_profile_path(bundle_id: &str) -> PathBuf {
    let mut path = PathBuf::from(PROFILE_OUTPUT_DIR);
    path.push(format!("{}.mobileconfig", bundle_id));
    path
}

/// Remove specific keys from a plist
///
/// If all keys are removed and the plist becomes empty, delete the file
#[cfg(target_os = "macos")]
pub fn remove_plist_keys(bundle_id: &str, keys: &[String]) -> Result<()> {
    let plist_path = get_plist_path(bundle_id)?;
    remove_plist_keys_at_path(&plist_path, keys)
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
    Value::Array(strings.into_iter().map(Value::String).collect())
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

/// Convert serde_json::Value to plist::Value
#[cfg(target_os = "macos")]
pub fn json_to_plist(value: &serde_json::Value) -> Option<Value> {
    match value {
        serde_json::Value::Bool(b) => Some(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Value::Integer(i.into()))
            } else if let Some(f) = n.as_f64() {
                Some(Value::Real(f))
            } else {
                None
            }
        }
        serde_json::Value::String(s) => Some(Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let plist_arr: Vec<Value> = arr.iter().filter_map(json_to_plist).collect();
            Some(Value::Array(plist_arr))
        }
        serde_json::Value::Object(obj) => {
            let mut plist_dict = plist::Dictionary::new();
            for (key, val) in obj {
                if let Some(plist_val) = json_to_plist(val) {
                    plist_dict.insert(key.clone(), plist_val);
                }
            }
            Some(Value::Dictionary(plist_dict))
        }
        serde_json::Value::Null => None,
    }
}

/// Write extension settings to a separate plist file
/// Extension settings go in: /Library/Managed Preferences/com.{browser}.extensions.{extension_id}.plist
#[cfg(target_os = "macos")]
pub fn write_extension_settings_plist(
    browser_bundle_prefix: &str,
    extension_id: &str,
    settings: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<()> {
    let bundle_id = format!("{}.extensions.{}", browser_bundle_prefix, extension_id);

    let mut plist_updates = HashMap::new();
    for (key, value) in settings {
        if let Some(plist_value) = json_to_plist(value) {
            plist_updates.insert(key.clone(), plist_value);
        } else {
            tracing::warn!("Could not convert setting {} to plist value", key);
        }
    }

    write_plist_policy(&bundle_id, plist_updates)
}

/// Remove extension settings plist file
#[cfg(target_os = "macos")]
pub fn remove_extension_settings_plist(
    browser_bundle_prefix: &str,
    extension_id: &str,
) -> Result<()> {
    let bundle_id = format!("{}.extensions.{}", browser_bundle_prefix, extension_id);
    remove_plist(&bundle_id)
}

/// Remove all extension settings plists for a browser
/// Removes all plists matching: /Library/Managed Preferences/{browser_bundle_prefix}.extensions.*.plist
#[cfg(target_os = "macos")]
pub fn remove_all_extension_settings_plists(browser_bundle_prefix: &str) -> Result<()> {
    let managed_prefs_dir = Path::new("/Library/Managed Preferences");

    if !managed_prefs_dir.exists() {
        return Ok(());
    }

    let pattern = format!("{}.extensions.", browser_bundle_prefix);

    match std::fs::read_dir(managed_prefs_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.starts_with(&pattern) && filename.ends_with(".plist") {
                        if let Err(e) = std::fs::remove_file(entry.path()) {
                            tracing::warn!(
                                "Failed to remove extension settings plist {}: {}",
                                filename,
                                e
                            );
                        } else {
                            tracing::debug!("Removed extension settings plist: {}", filename);
                        }
                    }
                }
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| {
            format!(
                "Failed to read Managed Preferences directory: {}",
                managed_prefs_dir.display()
            )
        }),
    }
}

/// Apply plist policy with dry-run support
/// Shows what would change in dry-run mode, actually writes in normal mode
#[cfg(target_os = "macos")]
pub fn apply_plist_policy_with_preview(
    bundle_id: &str,
    updates: HashMap<String, Value>,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        let plist_path = format!("/Library/Managed Preferences/{}.plist", bundle_id);
        println!("Plist File: {}", plist_path);

        // Try to read existing plist
        let existing_plist = if std::path::Path::new(&plist_path).exists() {
            std::fs::File::open(&plist_path)
                .ok()
                .and_then(|f| plist::from_reader::<_, plist::Value>(f).ok())
                .and_then(|v| {
                    if let plist::Value::Dictionary(d) = v {
                        Some(d)
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        if existing_plist.is_none() {
            println!("  Action: CREATE new plist file");
        } else {
            println!("  Action: UPDATE plist file");
        }
        println!();

        // Show each key being added/updated
        for (key, value) in &updates {
            println!("  Key: {}", key);
            if let Some(ref dict) = existing_plist {
                if dict.contains_key(key) {
                    println!("    Action: UPDATE");
                } else {
                    println!("    Action: ADD");
                }
            } else {
                println!("    Action: ADD");
            }

            // Show type and value
            match value {
                Value::Array(_) => {
                    println!("    + Type: Array");
                    if let Value::Array(arr) = value {
                        for item in arr {
                            if let Value::String(s) = item {
                                println!("      + {}", s);
                            }
                        }
                    }
                }
                Value::Integer(i) => {
                    println!("    + Type: Integer");
                    println!("    + Value: {}", i);
                }
                Value::Boolean(b) => {
                    println!("    + Type: Boolean");
                    println!("    + Value: {}", b);
                }
                Value::String(s) => {
                    println!("    + Type: String");
                    println!("    + Value: {}", s);
                }
                _ => {
                    println!("    + Type: Other");
                }
            }
            println!();
        }

        Ok(())
    } else {
        write_plist_policy(bundle_id, updates)
    }
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
