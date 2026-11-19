# Family Policy

A Rust CLI and system service for managing Chrome, Firefox, and Edge extension and privacy policies on Linux, macOS, and Windows. There is no GUI or Node.js dependency.

Download the archive for Linux or macOS from [Releases](https://github.com/emosenkis/family-policy/releases) and extract it with `tar -xzf family-policy-<target>.tar.gz`. It contains the executable `family-policy`, with execute permissions preserved. On Windows, download `family-policy.exe` directly. The macOS binary is native Apple Silicon (including M4).

Build from source with `cargo build --release`; the binary is in `target/release/`.

## CLI

```sh
# Generate a policy file using the required policies schema
family-policy config init --output browser-policy.yaml

# Preview or apply a local policy
family-policy --config browser-policy.yaml --dry-run
sudo family-policy --config browser-policy.yaml

# Preview or apply a policy downloaded over HTTPS
family-policy --config https://example.com/policy.yaml --dry-run
sudo family-policy --config https://example.com/policy.yaml

# Reapply even when the saved configuration hash matches
sudo family-policy --force-apply --verbose

# Remove policies tracked by this tool
sudo family-policy --uninstall
```

See [example-config.yaml](example-config.yaml) for the policy schema. Remote configurations must use HTTPS and pass the same validation as local files. Applying a URL downloads and applies once; use the service for ongoing polling.

## System service

Run the following as root on Unix, or in an elevated terminal on Windows (omit `sudo`):

```sh
sudo ./family-policy setup --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml
sudo ./family-policy install-service
sudo ./family-policy start
family-policy status
sudo ./family-policy check-now
sudo ./family-policy stop
sudo ./family-policy uninstall-service
```

`setup` writes the agent TOML configuration and refuses to overwrite an existing one unless `--force` is passed. Optional settings: `--token` for private repositories and `--poll-interval` (seconds, minimum 60). Avoid passing tokens on shared machines where command arguments/history are visible; instead edit the protected agent config file directly. See [agent.conf.example](agent.conf.example) for its schema.

Agent config locations:

- Linux: `/etc/family-policy/agent.conf`
- macOS: `/Library/Application Support/family-policy/agent.conf`
- Windows: `%ProgramData%\family-policy\agent.conf`

Installation copies the executable to `/usr/local/bin/family-policy` on Unix or `%ProgramFiles%\Family Policy\family-policy.exe` on Windows, then registers systemd, launchd, or a native Windows service. On macOS installation also starts the service. Uninstalling the service retains the binary, configuration, and browser policies; use `--uninstall` to remove tracked policies. `daemon` (or `start --no-daemon`) runs polling in the foreground.

## Development

```sh
cargo fmt --check
cargo test --locked
cargo build --locked --release
```

CI tests and builds native Linux x86_64, Windows x86_64, and macOS ARM64 binaries. A version tag publishes binaries and SHA-256 checksums only after all three platforms pass.

Browser policy changes require administrator privileges. Preview a policy before applying it and restart browsers after changes.

## macOS policy application

Chrome and Edge use machine-level MCX settings in Directory Service's
`/Computers/local_computer` record, including separate extension-managed domains.
Firefox uses enterprise preferences in `/Library/Preferences/org.mozilla.firefox.plist`.
No configuration profile is installed, and Firefox's app bundle is not modified.

The first apply after upgrading from the profile backend retries even if the policy
YAML is unchanged. Ownership receipts under `/Library/Application Support/browser-extension-policy/macos-ownership`
allow reapply and uninstall to preserve unrelated settings. Conflicting settings
from another manager cause an error rather than being overwritten. Existing profiles
and manually written managed-preferences caches are left intact.

Each configured browser is attempted even if another fails. Partial failures keep
successful browser tracking and clear the saved hash and ETag so the next run retries.
Dry runs do not modify policies or ownership receipts.

After applying, reload `chrome://policy` / `edge://policy` and check the configured
policies and extension-managed settings. Restart Firefox and check `about:policies`.
Native CI verifies Directory Service and preference writes, reapply, ownership,
rollback, and removal; it does not replace checking browser behavior on your Mac.
