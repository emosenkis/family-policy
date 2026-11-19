use anyhow::{Context, Result};
use plist::{Dictionary, Value};
use std::collections::HashMap;

#[cfg(target_os = "macos")]
mod system;
#[cfg(target_os = "macos")]
pub use system::{apply_firefox_preferences, apply_mcx, remove_firefox_preferences, remove_mcx};

/// Directory Service MCX uses metadata around each policy, including extension domains.
pub fn encode_mcx(domains: Vec<(String, HashMap<String, Value>)>) -> Dictionary {
    domains
        .into_iter()
        .filter(|(_, settings)| !settings.is_empty())
        .map(|(domain, settings)| {
            let entries = settings
                .into_iter()
                .map(|(key, value)| {
                    let mut entry = Dictionary::new();
                    entry.insert("state".into(), Value::String("always".into()));
                    entry.insert("value".into(), value);
                    (key, Value::Dictionary(entry))
                })
                .collect();
            (domain, Value::Dictionary(entries))
        })
        .collect()
}

/// Only values recorded by us can be updated or removed. Never claim identical external policies.
fn reconcile_owned(
    current: &Dictionary,
    owned: &Dictionary,
    desired: &Dictionary,
) -> Result<(Dictionary, Dictionary)> {
    let mut result = current.clone();
    for (domain, settings) in owned {
        let settings = settings
            .as_dictionary()
            .context("Invalid ownership receipt")?;
        if let Some(existing) = result.get_mut(domain) {
            let existing = existing
                .as_dictionary_mut()
                .context("Policy domain is not a dictionary")?;
            for (key, value) in settings {
                if existing.get(key) == Some(value) {
                    existing.remove(key);
                } else if existing.contains_key(key)
                    && desired
                        .get(domain)
                        .and_then(Value::as_dictionary)
                        .is_some_and(|d| d.contains_key(key))
                {
                    anyhow::bail!(
                        "Policy {domain}/{key} was changed outside family-policy; refusing to overwrite it"
                    );
                }
            }
            if existing.is_empty() {
                result.remove(domain);
            }
        }
    }
    let mut next_owned = Dictionary::new();
    for (domain, settings) in desired {
        let settings = settings
            .as_dictionary()
            .context("Policy domain is not a dictionary")?;
        if !result.contains_key(domain) {
            result.insert(domain.clone(), Value::Dictionary(Dictionary::new()));
        }
        let existing = result
            .get_mut(domain)
            .unwrap()
            .as_dictionary_mut()
            .context("Policy domain is not a dictionary")?;
        let mut inserted = Dictionary::new();
        for (key, value) in settings {
            if let Some(old) = existing.get(key) {
                if old != value {
                    anyhow::bail!(
                        "Policy {domain}/{key} already belongs to another manager; refusing to overwrite it"
                    );
                }
            } else {
                existing.insert(key.clone(), value.clone());
                inserted.insert(key.clone(), value.clone());
            }
        }
        if !inserted.is_empty() {
            next_owned.insert(domain.clone(), Value::Dictionary(inserted));
        }
    }
    Ok((result, next_owned))
}

/// Undo only keys touched by this update; preserve unrelated changes made meanwhile.
fn rollback_target(
    current: &Dictionary,
    before: &Dictionary,
    attempted: &Dictionary,
) -> Result<Dictionary> {
    let mut result = current.clone();
    let domains: std::collections::BTreeSet<_> = before.keys().chain(attempted.keys()).collect();
    for domain in domains {
        let old = before
            .get(domain)
            .and_then(Value::as_dictionary)
            .cloned()
            .unwrap_or_default();
        let new = attempted
            .get(domain)
            .and_then(Value::as_dictionary)
            .cloned()
            .unwrap_or_default();
        for key in old
            .keys()
            .chain(new.keys())
            .collect::<std::collections::BTreeSet<_>>()
        {
            if old.get(key) == new.get(key) {
                continue;
            }
            let observed = current
                .get(domain)
                .and_then(Value::as_dictionary)
                .and_then(|d| d.get(key));
            if observed.is_some() && observed != old.get(key) && observed != new.get(key) {
                anyhow::bail!("Cannot roll back {domain}/{key}: it was modified externally");
            }
            if !result.contains_key(domain) {
                result.insert(domain.clone(), Value::Dictionary(Dictionary::new()));
            }
            let values = result
                .get_mut(domain)
                .unwrap()
                .as_dictionary_mut()
                .context("Invalid policy domain")?;
            if let Some(value) = old.get(key) {
                values.insert(key.clone(), value.clone());
            } else {
                values.remove(key);
            }
            if values.is_empty() {
                result.remove(domain);
            }
        }
    }
    Ok(result)
}

pub fn string_vec_to_plist_array(strings: Vec<String>) -> Value {
    Value::Array(strings.into_iter().map(Value::String).collect())
}
pub fn integer_to_plist(val: i64) -> Value {
    Value::Integer(val.into())
}
pub fn bool_to_plist(val: bool) -> Value {
    Value::Boolean(val)
}

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
            let plist_arr: Vec<Value> = arr.iter().map(json_to_plist).collect::<Option<_>>()?;
            Some(Value::Array(plist_arr))
        }
        serde_json::Value::Object(obj) => {
            let mut plist_dict = plist::Dictionary::new();
            for (key, val) in obj {
                plist_dict.insert(key.clone(), json_to_plist(val)?);
            }
            Some(Value::Dictionary(plist_dict))
        }
        serde_json::Value::Null => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict(value: serde_json::Value) -> Dictionary {
        plist::to_value(&value).unwrap().into_dictionary().unwrap()
    }

    #[test]
    fn rollback_restores_partial_import_without_losing_unrelated_updates() {
        let before =
            dict(serde_json::json!({"browser":{"Owned":1,"Removed":true,"Unrelated":false}}));
        let attempted =
            dict(serde_json::json!({"browser":{"Owned":2,"New":true,"Unrelated":false}}));
        let partial = dict(serde_json::json!({"browser":{"Owned":2,"Unrelated":true}}));
        let restored = rollback_target(&partial, &before, &attempted).unwrap();
        assert_eq!(
            restored,
            dict(serde_json::json!({"browser":{"Owned":1,"Removed":true,"Unrelated":true}}))
        );
        let conflicting = dict(serde_json::json!({"browser":{"Owned":99}}));
        assert!(rollback_target(&conflicting, &before, &attempted).is_err());
    }

    #[test]
    fn mcx_encodes_browser_and_extension_domains() {
        let settings = vec![
            (
                "com.google.Chrome".into(),
                HashMap::from([
                    ("IncognitoModeAvailability".into(), Value::Integer(1.into())),
                    ("BrowserGuestModeEnabled".into(), Value::Boolean(false)),
                    ("AllowDeletingBrowserHistory".into(), Value::Boolean(false)),
                    (
                        "ExtensionInstallForcelist".into(),
                        Value::Array(vec![Value::String("extension;https://example.com".into())]),
                    ),
                ]),
            ),
            (
                "com.google.Chrome.extensions.extension".into(),
                HashMap::from([(
                    "rulesets".into(),
                    Value::Array(vec![Value::String("default".into())]),
                )]),
            ),
        ];
        let mcx = encode_mcx(settings);
        assert_eq!(
            mcx["com.google.Chrome"].as_dictionary().unwrap()["IncognitoModeAvailability"],
            Value::Dictionary(dict(serde_json::json!({"state":"always","value":1})))
        );
        let extension = mcx["com.google.Chrome.extensions.extension"]
            .as_dictionary()
            .unwrap();
        assert_eq!(
            extension["rulesets"].as_dictionary().unwrap()["state"].as_string(),
            Some("always")
        );
        assert!(
            extension["rulesets"].as_dictionary().unwrap()["value"]
                .as_array()
                .is_some()
        );
    }

    #[test]
    fn reapply_and_uninstall_preserve_unowned_values() {
        let existing = dict(serde_json::json!({"browser":{"Unrelated":true}}));
        let desired = dict(serde_json::json!({"browser":{"Owned":1}}));
        let (first, receipt) = reconcile_owned(&existing, &Dictionary::new(), &desired).unwrap();
        let (again, receipt2) = reconcile_owned(&first, &receipt, &desired).unwrap();
        assert_eq!(first, again);
        assert_eq!(receipt, receipt2);
        let (removed, _) = reconcile_owned(&again, &receipt, &Dictionary::new()).unwrap();
        assert_eq!(removed, existing);
    }

    #[test]
    fn refuses_to_replace_another_managers_policy() {
        let existing = dict(serde_json::json!({"browser":{"Key":true}}));
        let desired = dict(serde_json::json!({"browser":{"Key":false}}));
        assert!(reconcile_owned(&existing, &Dictionary::new(), &desired).is_err());
        // An identical value supplied by somebody else remains unowned.
        let (_, receipt) = reconcile_owned(&existing, &Dictionary::new(), &existing).unwrap();
        assert!(receipt.is_empty());
    }

    #[test]
    fn uninstall_does_not_remove_externally_modified_owned_keys() {
        let old = dict(serde_json::json!({"browser":{"Key":1}}));
        let modified = dict(serde_json::json!({"browser":{"Key":2}}));
        let (result, _) = reconcile_owned(&modified, &old, &Dictionary::new()).unwrap();
        assert_eq!(result, modified);
        assert!(reconcile_owned(&modified, &old, &old).is_err());
    }
}
