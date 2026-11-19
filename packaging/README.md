# Service definitions

The Rust binary embeds the systemd unit and launchd plist here. Use
`family-policy setup --url HTTPS_URL`, then `family-policy install-service`.
Windows uses the native Service Control Manager through the binary.
