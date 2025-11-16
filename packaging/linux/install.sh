#!/bin/bash
# Family Policy Agent - Linux Installation Script
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

echo "Family Policy Agent - Installation"
echo "===================================="
echo

# Detect Linux distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    VER=$VERSION_ID
else
    echo -e "${RED}Error: Cannot detect Linux distribution${NC}"
    exit 1
fi

echo "Detected: $PRETTY_NAME"
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
mkdir -p /etc/family-policy
mkdir -p /var/lib/browser-extension-policy
echo -e "${GREEN}✓${NC} Directories created"

# Install systemd service
echo "Installing systemd service..."
SERVICE_FILE="/etc/systemd/system/family-policy-agent.service"

if [ -f "./family-policy-agent.service" ]; then
    cp ./family-policy-agent.service "$SERVICE_FILE"
    chmod 644 "$SERVICE_FILE"

    # Reload systemd
    systemctl daemon-reload
    echo -e "${GREEN}✓${NC} Systemd service installed"
else
    echo -e "${YELLOW}Warning: Service file not found, skipping service installation${NC}"
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
echo "  2. Enable and start the service:"
echo "     sudo systemctl enable family-policy-agent"
echo "     sudo systemctl start family-policy-agent"
echo
echo "  3. Check status:"
echo "     sudo systemctl status family-policy-agent"
echo "     sudo family-policy status"
echo
