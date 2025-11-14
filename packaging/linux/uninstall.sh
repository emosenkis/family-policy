#!/bin/bash
# Family Policy Agent - Linux Uninstallation Script
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

echo "Family Policy Agent - Uninstallation"
echo "====================================="
echo

# Stop and disable service
echo "Stopping service..."
if systemctl is-active --quiet family-policy-agent; then
    systemctl stop family-policy-agent
    echo -e "${GREEN}✓${NC} Service stopped"
else
    echo "Service not running"
fi

if systemctl is-enabled --quiet family-policy-agent 2>/dev/null; then
    systemctl disable family-policy-agent
    echo -e "${GREEN}✓${NC} Service disabled"
fi

# Remove service file
if [ -f /etc/systemd/system/family-policy-agent.service ]; then
    rm /etc/systemd/system/family-policy-agent.service
    systemctl daemon-reload
    echo -e "${GREEN}✓${NC} Service file removed"
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
    rm -rf /etc/family-policy
    rm -rf /var/lib/browser-extension-policy

    echo -e "${GREEN}✓${NC} Configuration and state files removed"
else
    echo "Configuration and state files preserved in:"
    echo "  - /etc/family-policy/"
    echo "  - /var/lib/browser-extension-policy/"
fi

echo
echo -e "${GREEN}Uninstallation complete!${NC}"
