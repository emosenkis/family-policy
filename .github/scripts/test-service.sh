#!/usr/bin/env bash
set -euo pipefail
binary="$1"
# A closed local port exercises service startup without applying browser policies.
"$binary" setup --url https://127.0.0.1:9/policy.yaml
if "$binary" setup --url https://127.0.0.1:9/replacement.yaml; then
  echo 'setup overwrote existing configuration without --force' >&2
  exit 1
fi
"$binary" install-service
"$binary" start
if [[ $(uname) == Darwin ]]; then
  for attempt in {1..20}; do
    if launchctl print system/com.family-policy.agent | grep -q 'state = running'; then
      break
    fi
    sleep 1
  done
  if ! launchctl print system/com.family-policy.agent | grep 'state = running'; then
    launchctl print system/com.family-policy.agent
    cat /var/log/family-policy-agent.log
    exit 1
  fi
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
