#!/bin/bash
# Family Policy Agent - macOS Uninstallation Script
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This script must be run as root${NC}"
    echo "Please run: sudo $0"
    exit 1
fi

echo "Family Policy Agent - macOS Uninstallation"
echo "==========================================="
echo

# Stop and unload LaunchDaemon
PLIST_PATH="/Library/LaunchDaemons/com.family-policy.agent.plist"

echo "Stopping daemon..."
if launchctl list | grep -q com.family-policy.agent; then
    launchctl unload "$PLIST_PATH" 2>/dev/null || true
    echo -e "${GREEN}✓${NC} Daemon stopped and unloaded"
else
    echo "Daemon not running"
fi

# Remove plist
if [ -f "$PLIST_PATH" ]; then
    rm "$PLIST_PATH"
    echo -e "${GREEN}✓${NC} LaunchDaemon plist removed"
fi

# Remove binary
if [ -f /usr/local/bin/family-policy ]; then
    rm /usr/local/bin/family-policy
    echo -e "${GREEN}✓${NC} Binary removed"
fi

# Ask about configuration and data
echo
read -p "Remove configuration and state files? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Remove policies first
    if [ -f /usr/local/bin/family-policy ]; then
        family-policy --uninstall 2>/dev/null || true
    fi

    # Remove directories
    rm -rf "/Library/Application Support/family-policy"
    rm -rf "/Library/Application Support/browser-extension-policy"
    rm -f /var/log/family-policy-agent.log

    echo -e "${GREEN}✓${NC} Configuration and state files removed"
else
    echo "Configuration and state files preserved in:"
    echo "  - /Library/Application Support/family-policy/"
    echo "  - /Library/Application Support/browser-extension-policy/"
fi

echo
echo -e "${GREEN}Uninstallation complete!${NC}"
