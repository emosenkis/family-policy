#!/usr/bin/env bash
set -euo pipefail
binary="$1"
# A closed local port exercises service startup without applying browser policies.
"$binary" setup --url https://127.0.0.1:9/policy.yaml
"$binary" install-service
"$binary" start
if [[ $(uname) == Darwin ]]; then
  launchctl print system/com.family-policy.agent | grep 'state = running'
else
  systemctl is-enabled family-policy-agent
  systemctl is-active family-policy-agent
fi
"$binary" status
"$binary" stop
"$binary" start
"$binary" stop
"$binary" uninstall-service
if [[ $(uname) == Darwin ]]; then
  test ! -e /Library/LaunchDaemons/com.family-policy.agent.plist
else
  test ! -e /etc/systemd/system/family-policy-agent.service
fi
