#!/bin/bash
# Family Policy Agent - macOS Installation Script
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

echo "Family Policy Agent - macOS Installation"
echo "========================================="
echo

# Install binary
echo "Installing binary..."
BINARY_PATH="/usr/local/bin/family-policy"

if [ ! -f "./family-policy" ]; then
    echo -e "${RED}Error: family-policy binary not found in current directory${NC}"
    echo "Please run this script from the directory containing the binary"
    exit 1
fi

cp ./family-policy "$BINARY_PATH"
chmod 755 "$BINARY_PATH"
echo -e "${GREEN}✓${NC} Binary installed to $BINARY_PATH"

# Create necessary directories
echo "Creating directories..."
mkdir -p "/Library/Application Support/family-policy"
mkdir -p "/Library/Application Support/browser-extension-policy"
mkdir -p /var/log
echo -e "${GREEN}✓${NC} Directories created"

# Install LaunchDaemon
echo "Installing LaunchDaemon..."
PLIST_PATH="/Library/LaunchDaemons/com.family-policy.agent.plist"

if [ -f "./com.family-policy.agent.plist" ]; then
    cp ./com.family-policy.agent.plist "$PLIST_PATH"
    chmod 644 "$PLIST_PATH"
    chown root:wheel "$PLIST_PATH"
    echo -e "${GREEN}✓${NC} LaunchDaemon installed"
else
    echo -e "${YELLOW}Warning: LaunchDaemon plist not found, skipping daemon installation${NC}"
fi

echo
echo -e "${GREEN}Installation complete!${NC}"
echo
echo "Next steps:"
echo "  1. Configure the agent:"
echo "     sudo family-policy setup \\"
echo "       --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml \\"
echo "       --token YOUR_GITHUB_TOKEN"
echo
echo "  2. Load and start the daemon:"
echo "     sudo launchctl load $PLIST_PATH"
echo
echo "  3. Check status:"
echo "     sudo launchctl list | grep family-policy"
echo "     sudo family-policy status"
echo
echo "  4. View logs:"
echo "     tail -f /var/log/family-policy-agent.log"
echo
