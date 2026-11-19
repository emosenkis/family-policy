use std::process::Command;

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_family-policy"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn cli_has_setup_and_no_gui() {
    let output = cli(&["--help"]);
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("setup"));
    assert!(!help.contains("user-ui"));
    assert!(!help.contains("admin-ui"));
    assert!(cli(&["setup", "--help"]).status.success());
}

#[test]
fn remote_config_rejects_insecure_transport() {
    let output = cli(&["--dry-run", "--config", "http://example.com/policy.yaml"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("HTTPS"));
}

#[test]
fn example_policy_can_be_previewed() {
    let output = cli(&["--dry-run", "--config", "example-config.yaml"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn service_definitions_run_the_installed_binary() {
    let unit = include_str!("../packaging/linux/family-policy-agent.service");
    assert!(unit.contains("ExecStart=/usr/local/bin/family-policy start --no-daemon"));
    assert!(!unit.contains("ProtectSystem=strict"));
    let plist: plist::Value = plist::from_bytes(include_bytes!(
        "../packaging/macos/com.family-policy.agent.plist"
    ))
    .unwrap();
    let args = plist.as_dictionary().unwrap()["ProgramArguments"]
        .as_array()
        .unwrap();
    assert_eq!(args[0].as_string(), Some("/usr/local/bin/family-policy"));
    assert_eq!(args[1].as_string(), Some("start"));
    assert_eq!(args[2].as_string(), Some("--no-daemon"));
}
