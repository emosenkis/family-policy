use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const RECORD: &str = "/Computers/local_computer";
const RECEIPTS: &str = "/Library/Application Support/browser-extension-policy/macos-ownership";
const FIREFOX: &str = "org.mozilla.firefox";

fn command(program: &str, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run {program}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "{program} {} failed ({}): stdout: {}; stderr: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output.stdout)
}
fn dscl(args: &[&str]) -> Result<Vec<u8>> {
    let mut all = vec!["/Local/Default"];
    all.extend_from_slice(args);
    command("/usr/bin/dscl", &all)
}
fn parse_dictionary(bytes: &[u8]) -> Result<Dictionary> {
    Value::from_reader(std::io::Cursor::new(bytes))?
        .into_dictionary()
        .context("Expected a plist dictionary")
}
fn read_dictionary(path: &Path) -> Result<Dictionary> {
    if !path.exists() {
        return Ok(Dictionary::new());
    }
    parse_dictionary(&fs::read(path)?).with_context(|| {
        format!(
            "Invalid plist at {}; refusing to replace it",
            path.display()
        )
    })
}
fn write_dictionary(path: &Path, dict: &Dictionary) -> Result<()> {
    let mut data = Vec::new();
    Value::Dictionary(dict.clone()).to_writer_xml(&mut data)?;
    // The general atomic writer makes its parent public; ownership receipts stay private.
    let temporary = path.with_extension("tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&temporary)?;
    crate::platform::set_file_permissions(&temporary, 0o600)?;
    file.set_len(0)?;
    file.write_all(&data)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    Ok(())
}
fn lock() -> Result<File> {
    // Serialize CLI and daemon updates so their receipts cannot race.
    fs::create_dir_all(RECEIPTS)?;
    crate::platform::set_file_permissions(Path::new(RECEIPTS), 0o700)?;
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(Path::new(RECEIPTS).join("lock"))?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(std::io::Error::last_os_error()).context("Failed to lock macOS policy updates");
    }
    Ok(file)
}
fn record_exists() -> Result<bool> {
    let names = dscl(&["-list", "/Computers"])?;
    Ok(String::from_utf8(names)?
        .lines()
        .any(|line| line.trim() == "local_computer"))
}
fn record_attributes() -> Result<Dictionary> {
    parse_dictionary(&command(
        "/usr/bin/dscl",
        &["-plist", "/Local/Default", "-read", RECORD],
    )?)
}
fn attribute<'a>(dict: &'a Dictionary, key: &str) -> Option<&'a str> {
    dict.get(&format!("dsAttrTypeStandard:{key}"))?
        .as_array()?
        .first()?
        .as_string()
}
fn ensure_record() -> Result<()> {
    let interface = String::from_utf8(command("/sbin/ifconfig", &["en0"])?)?;
    let mac = interface
        .lines()
        .find_map(|line| line.trim().strip_prefix("ether "))
        .and_then(|line| line.split_whitespace().next())
        .context("No en0 hardware address found")?;
    if mac.split(':').count() != 6
        || !mac
            .split(':')
            .all(|part| part.len() == 2 && u8::from_str_radix(part, 16).is_ok())
    {
        anyhow::bail!("Invalid en0 hardware address: {mac}");
    }
    if record_exists()? {
        let attrs = record_attributes()?;
        uuid::Uuid::parse_str(attribute(&attrs, "GeneratedUID").context("Existing local_computer record has no GeneratedUID; repair it before applying policies")?)?;
        let addresses = attrs
            .get("dsAttrTypeStandard:ENetAddress")
            .and_then(Value::as_array);
        if !addresses.is_some_and(|a| {
            a.iter()
                .any(|v| v.as_string().is_some_and(|s| s.eq_ignore_ascii_case(mac)))
        }) {
            anyhow::bail!(
                "Existing local_computer record does not match en0 address {mac}; refusing to alter another computer record"
            );
        }
        return Ok(());
    }
    let result = (|| -> Result<()> {
        dscl(&["-create", RECORD])?;
        dscl(&[
            "-create",
            RECORD,
            "GeneratedUID",
            &uuid::Uuid::new_v4().to_string(),
        ])?;
        dscl(&["-create", RECORD, "ENetAddress", mac])?;
        dscl(&["-create", RECORD, "RealName", "Local Computer"])?;
        write_dictionary(
            &Path::new(RECEIPTS).join("created-record.plist"),
            &record_attributes()?,
        )
    })();
    if let Err(error) = result {
        let rollback = dscl(&["-delete", RECORD]);
        anyhow::bail!("Failed to create local computer record: {error:#}; cleanup: {rollback:?}");
    }
    Ok(())
}
fn read_mcx() -> Result<Dictionary> {
    if !record_exists()? {
        return Ok(Dictionary::new());
    }
    if !record_attributes()?.contains_key("dsAttrTypeStandard:MCXSettings") {
        return Ok(Dictionary::new());
    }
    let output = dscl(&["-mcxexport", RECORD])?;
    parse_dictionary(&output)
        .context("Could not export existing MCXSettings; refusing to replace them")
}
fn refresh() -> Result<()> {
    let uid = String::from_utf8(command("/usr/bin/stat", &["-f", "%u", "/dev/console"])?)?;
    let uid: u32 = uid.trim().parse().context("Invalid console user ID")?;
    command("/usr/bin/mcxrefresh", &["-u", &uid.to_string()])?;
    Ok(())
}
fn write_mcx(before: &Dictionary, after: &Dictionary) -> Result<()> {
    // mcximport merges keys; delete only keys removed from our transaction.
    for (domain, settings) in before {
        for key in settings
            .as_dictionary()
            .context("Invalid MCX domain")?
            .keys()
        {
            if after
                .get(domain)
                .and_then(Value::as_dictionary)
                .and_then(|d| d.get(key))
                .is_none()
            {
                dscl(&["-mcxdelete", RECORD, domain, key])?;
            }
        }
    }
    let mut changed = Dictionary::new();
    for (domain, settings) in after {
        let mut values = Dictionary::new();
        for (key, value) in settings.as_dictionary().context("Invalid MCX domain")? {
            if before
                .get(domain)
                .and_then(Value::as_dictionary)
                .and_then(|d| d.get(key))
                != Some(value)
            {
                values.insert(key.clone(), value.clone());
            }
        }
        if !values.is_empty() {
            changed.insert(domain.clone(), Value::Dictionary(values));
        }
    }
    if !changed.is_empty() {
        let path = Path::new(RECEIPTS).join("import.plist");
        write_dictionary(&path, &changed)?;
        dscl(&[
            "-mcximport",
            RECORD,
            path.to_str().context("Invalid import path")?,
        ])?;
        fs::remove_file(path)?;
    }
    Ok(())
}
fn cleanup_record() -> Result<()> {
    let marker = Path::new(RECEIPTS).join("created-record.plist");
    if !marker.exists() || !record_exists()? || !read_mcx()?.is_empty() {
        return Ok(());
    }
    let mut original = read_dictionary(&marker)?;
    let mut current = record_attributes()?;
    for key in [
        "dsAttrTypeStandard:MCXSettings",
        "dsAttrTypeStandard:MCXFlags",
    ] {
        original.remove(key);
        current.remove(key);
    }
    if original == current {
        dscl(&["-delete", RECORD])?;
        fs::remove_file(marker)?;
    }
    Ok(())
}

enum Backend {
    Mcx,
    Firefox,
}
impl Backend {
    fn read(&self) -> Result<Dictionary> {
        match self {
            Self::Mcx => read_mcx(),
            Self::Firefox => {
                let mut domains = Dictionary::new();
                let path = Path::new("/Library/Preferences/org.mozilla.firefox.plist");
                if path.exists() {
                    let content = command(
                        "/usr/bin/defaults",
                        &["export", "/Library/Preferences/org.mozilla.firefox", "-"],
                    )?;
                    let values = parse_dictionary(&content)?;
                    if !values.is_empty() {
                        domains.insert(FIREFOX.into(), Value::Dictionary(values));
                    }
                }
                Ok(domains)
            }
        }
    }
    fn write(&self, before: &Dictionary, after: &Dictionary) -> Result<()> {
        match self {
            Self::Mcx => write_mcx(before, after),
            Self::Firefox => {
                let values = after
                    .get(FIREFOX)
                    .and_then(Value::as_dictionary)
                    .cloned()
                    .unwrap_or_default();
                // defaults import merges; remove only owned keys omitted by this update.
                if let Some(previous) = before.get(FIREFOX).and_then(Value::as_dictionary) {
                    for key in previous.keys().filter(|key| !values.contains_key(*key)) {
                        command(
                            "/usr/bin/defaults",
                            &["delete", "/Library/Preferences/org.mozilla.firefox", key],
                        )?;
                    }
                }
                let path = Path::new(RECEIPTS).join("firefox-import.plist");
                write_dictionary(&path, &values)?;
                command(
                    "/usr/bin/defaults",
                    &[
                        "import",
                        "/Library/Preferences/org.mozilla.firefox",
                        path.to_str().unwrap(),
                    ],
                )?;
                fs::remove_file(path)?;
                let preferences = Path::new("/Library/Preferences/org.mozilla.firefox.plist");
                if preferences.exists() {
                    crate::platform::set_file_permissions(preferences, 0o644)?;
                }
                Ok(())
            }
        }
    }
    fn refresh(&self) -> Result<()> {
        if matches!(self, Self::Mcx) {
            refresh()?;
        }
        Ok(())
    }
}

fn apply_owned(
    owner: &str,
    desired: Dictionary,
    backend: Backend,
    dry_run: bool,
    refresh: impl Fn(&Backend) -> Result<()>,
) -> Result<()> {
    if dry_run {
        println!("macOS {} policy values (dry run): {desired:#?}", owner);
        return Ok(());
    }
    let _lock = lock()?;
    let receipt_path: PathBuf = Path::new(RECEIPTS).join(format!("{owner}.plist"));
    let receipt = read_dictionary(&receipt_path)?;
    if matches!(backend, Backend::Mcx) && !desired.is_empty() {
        ensure_record()?;
    }
    let before = backend.read()?;
    let (after, next_receipt) = reconcile_owned(&before, &receipt, &desired)?;
    let result = (|| -> Result<()> {
        backend.write(&before, &after)?;
        refresh(&backend)?;
        if backend.read()? != after {
            anyhow::bail!("macOS policy readback did not match the requested update");
        }
        write_dictionary(&receipt_path, &next_receipt)?;
        Ok(())
    })();
    if let Err(error) = result {
        // Restore just the transaction's keys, never whole domains or Directory Service records.
        let rollback = backend
            .read()
            .and_then(|current| {
                rollback_target(&current, &before, &after)
                    .and_then(|target| backend.write(&current, &target))
            })
            .and_then(|_| refresh(&backend));
        let receipt_rollback = write_dictionary(&receipt_path, &receipt);
        anyhow::bail!(
            "macOS policy update failed: {error:#}; rollback: {rollback:?}; receipt rollback: {receipt_rollback:?}"
        );
    }
    if matches!(backend, Backend::Mcx) && desired.is_empty() {
        cleanup_record()?;
    }
    Ok(())
}

pub fn apply_mcx(
    owner: &str,
    domains: Vec<(String, HashMap<String, Value>)>,
    dry_run: bool,
) -> Result<()> {
    apply_owned(
        owner,
        encode_mcx(domains),
        Backend::Mcx,
        dry_run,
        Backend::refresh,
    )
}
pub fn remove_mcx(owner: &str) -> Result<()> {
    apply_owned(
        owner,
        Dictionary::new(),
        Backend::Mcx,
        false,
        Backend::refresh,
    )
}
pub fn apply_firefox_preferences(settings: HashMap<String, Value>, dry_run: bool) -> Result<()> {
    let mut desired = Dictionary::new();
    desired.insert(
        FIREFOX.into(),
        Value::Dictionary(settings.into_iter().collect()),
    );
    apply_owned(
        FIREFOX,
        desired,
        Backend::Firefox,
        dry_run,
        Backend::refresh,
    )
}
pub fn remove_firefox_preferences() -> Result<()> {
    apply_owned(
        FIREFOX,
        Dictionary::new(),
        Backend::Firefox,
        false,
        Backend::refresh,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires root on a disposable macOS machine"]
    fn native_mcx_and_firefox_roundtrip() {
        assert_eq!(unsafe { libc::geteuid() }, 0);
        let owner = "com.family-policy.ci";
        let domain = "com.family-policy.ci.browser";
        let extension_domain = "com.family-policy.ci.browser.extensions.test";
        {
            let _lock = lock().unwrap();
            ensure_record().unwrap();
        }
        let original = read_mcx().unwrap();
        let mut external = original.clone();
        let mut unowned = Dictionary::new();
        unowned.insert(
            "Unrelated".into(),
            Value::Dictionary(
                [
                    ("state".to_string(), Value::String("always".into())),
                    ("value".to_string(), Value::Boolean(true)),
                ]
                .into_iter()
                .collect(),
            ),
        );
        external.insert(domain.into(), Value::Dictionary(unowned));
        write_mcx(&original, &external).unwrap();
        let domains = || {
            vec![
                (
                    domain.into(),
                    HashMap::from([("Owned".into(), Value::Boolean(true))]),
                ),
                (
                    extension_domain.into(),
                    HashMap::from([(
                        "Rules".into(),
                        Value::Array(vec![Value::String("default".into())]),
                    )]),
                ),
            ]
        };
        apply_mcx(owner, domains(), false).unwrap();
        let applied = read_mcx().unwrap();
        assert!(
            applied[domain]
                .as_dictionary()
                .unwrap()
                .contains_key("Unrelated")
        );
        assert!(
            applied[extension_domain]
                .as_dictionary()
                .unwrap()
                .contains_key("Rules")
        );
        apply_mcx(owner, domains(), false).unwrap();
        assert_eq!(applied, read_mcx().unwrap());

        let fail_once = std::cell::Cell::new(true);
        let update = encode_mcx(vec![(
            domain.into(),
            HashMap::from([("Owned".into(), Value::Boolean(false))]),
        )]);
        let failed = apply_owned(owner, update, Backend::Mcx, false, |backend| {
            if fail_once.replace(false) {
                anyhow::bail!("simulated refresh failure");
            }
            backend.refresh()
        });
        assert!(
            failed
                .unwrap_err()
                .to_string()
                .contains("simulated refresh failure")
        );
        assert_eq!(applied, read_mcx().unwrap());

        // Force a failure after the import, then check the old working values survived.
        let blocked = Path::new(RECEIPTS).join(format!("{owner}.tmp"));
        fs::create_dir(&blocked).unwrap();
        let update = vec![(
            domain.into(),
            HashMap::from([("Owned".into(), Value::Boolean(false))]),
        )];
        assert!(apply_mcx(owner, update, false).is_err());
        fs::remove_dir(blocked).unwrap();
        assert_eq!(applied, read_mcx().unwrap());
        remove_mcx(owner).unwrap();
        assert_eq!(external, read_mcx().unwrap());
        remove_mcx(owner).unwrap();
        write_mcx(&external, &original).unwrap();
        refresh().unwrap();
        cleanup_record().unwrap();

        let backend = Backend::Firefox;
        let original = backend.read().unwrap();
        let settings = HashMap::from([
            ("EnterprisePoliciesEnabled".into(), Value::Boolean(true)),
            ("DisablePrivateBrowsing".into(), Value::Boolean(true)),
        ]);
        apply_firefox_preferences(settings.clone(), false).unwrap();
        let applied = backend.read().unwrap();
        assert_eq!(
            applied[FIREFOX].as_dictionary().unwrap()["DisablePrivateBrowsing"],
            Value::Boolean(true)
        );
        apply_firefox_preferences(settings, false).unwrap();
        assert_eq!(applied, backend.read().unwrap());
        remove_firefox_preferences().unwrap();
        assert_eq!(original, backend.read().unwrap());

        // Exercise actual browser dispatch, partial-state retries, and complete macOS keys.
        let state_path = crate::state::get_state_path().unwrap();
        let saved_state = fs::read(&state_path).ok();
        let original_mcx = read_mcx().unwrap();
        let mut legacy = crate::state::State::new_agent();
        legacy.config_hash = "legacy-profile-hash".into();
        legacy.etag = Some("legacy-profile-etag".into());
        legacy.macos_policy_backend.clear();
        crate::state::save_state(&legacy).unwrap();
        let migrated = crate::state::load_state().unwrap().unwrap();
        assert!(migrated.config_hash.is_empty());
        assert!(migrated.etag.is_none());
        let config = crate::config::Config::from_yaml_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/macos-policy.yaml"
        )))
        .unwrap();
        let conflict_owner = "com.family-policy.ci.conflict";
        apply_mcx(
            conflict_owner,
            vec![(
                "com.google.Chrome".into(),
                HashMap::from([("IncognitoModeAvailability".into(), Value::Integer(0.into()))]),
            )],
            false,
        )
        .unwrap();
        let previous = crate::state::load_state().unwrap();
        let error = crate::policy::apply_policies(&config, previous.as_ref(), false).unwrap_err();
        let failure = error
            .downcast_ref::<crate::policy::PolicyApplyError>()
            .unwrap();
        assert!(failure.applied.chrome.is_none());
        assert!(failure.applied.firefox.is_some());
        assert!(failure.applied.edge.is_some());
        crate::policy::persist_partial_failure(&error, previous.as_ref(), false).unwrap();
        let partial = crate::state::load_state().unwrap().unwrap();
        assert!(partial.config_hash.is_empty());
        assert!(partial.etag.is_none());
        assert!(partial.applied_policies.firefox.is_some());
        assert!(partial.applied_policies.edge.is_some());
        remove_mcx(conflict_owner).unwrap();
        crate::core::apply::apply_policies_from_config(&config, false).unwrap();
        let policies = read_mcx().unwrap();
        for (domain, private_key) in [
            ("com.google.Chrome", "IncognitoModeAvailability"),
            ("com.microsoft.Edge", "InPrivateModeAvailability"),
        ] {
            let values = policies[domain].as_dictionary().unwrap();
            assert_eq!(
                values[private_key].as_dictionary().unwrap()["value"],
                Value::Integer(1.into())
            );
            assert_eq!(
                values["BrowserGuestModeEnabled"].as_dictionary().unwrap()["value"],
                Value::Boolean(false)
            );
            assert_eq!(
                values["AllowDeletingBrowserHistory"]
                    .as_dictionary()
                    .unwrap()["value"],
                Value::Boolean(false)
            );
            assert_eq!(
                values["ExtensionInstallForcelist"].as_dictionary().unwrap()["value"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            let extension = policies
                [&format!("{domain}.extensions.ddkjiahejlhfcafbddmgiahcphecmpfh")]
                .as_dictionary()
                .unwrap();
            assert_eq!(
                extension["defaultFiltering"].as_dictionary().unwrap()["value"].as_string(),
                Some("optimal")
            );
            assert!(
                extension["rulesets"].as_dictionary().unwrap()["value"]
                    .as_array()
                    .is_some()
            );
        }
        let firefox = backend.read().unwrap();
        let firefox = firefox[FIREFOX].as_dictionary().unwrap();
        assert_eq!(firefox["EnterprisePoliciesEnabled"], Value::Boolean(true));
        assert_eq!(firefox["DisablePrivateBrowsing"], Value::Boolean(true));
        assert!(
            firefox["ExtensionSettings"]
                .as_dictionary()
                .unwrap()
                .contains_key("*")
        );
        assert!(
            firefox["ExtensionSettings"]
                .as_dictionary()
                .unwrap()
                .contains_key("uBOLiteRedux@raymondhill.net")
        );
        let state_before_dry_run = fs::read(&state_path).unwrap();
        crate::core::apply::apply_policies_from_config(&config, true).unwrap();
        assert_eq!(state_before_dry_run, fs::read(&state_path).unwrap());
        assert_eq!(policies, read_mcx().unwrap());
        crate::core::apply::apply_policies_from_config_with_options(
            &config,
            crate::core::apply::ApplyOptions {
                force: true,
                dry_run: false,
            },
        )
        .unwrap();
        assert_eq!(policies, read_mcx().unwrap());
        crate::core::apply::remove_all_policies(false).unwrap();
        assert_eq!(original_mcx, read_mcx().unwrap());
        assert_eq!(original, backend.read().unwrap());
        if let Some(content) = saved_state {
            crate::platform::atomic_write(&state_path, &content).unwrap();
        }
    }
}
