#!/bin/bash
# Build macOS PKG installer for family-policy
set -e

VERSION="0.1.0"
IDENTIFIER="com.family-policy.pkg"

# Accept binary path as first argument, default to target/release/family-policy
BINARY_PATH="${1:-target/release/family-policy}"

echo "Building macOS PKG installer: family-policy-${VERSION}.pkg"
echo "Using binary: $BINARY_PATH"
echo

# Check if running on macOS
if [ "$(uname)" != "Darwin" ]; then
    echo "Error: This script must be run on macOS"
    exit 1
fi

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    echo "Please build the binary first or provide the correct path as the first argument"
    exit 1
fi

# Create temporary installation root
echo "Creating installation structure..."
INSTALL_ROOT=$(mktemp -d)
trap "rm -rf $INSTALL_ROOT" EXIT

# Install binary
mkdir -p "$INSTALL_ROOT/usr/local/bin"
cp "$BINARY_PATH" "$INSTALL_ROOT/usr/local/bin/family-policy"
chmod 755 "$INSTALL_ROOT/usr/local/bin/family-policy"

# Install LaunchDaemon
mkdir -p "$INSTALL_ROOT/Library/LaunchDaemons"
cp packaging/macos/com.family-policy.agent.plist "$INSTALL_ROOT/Library/LaunchDaemons/"
chmod 644 "$INSTALL_ROOT/Library/LaunchDaemons/com.family-policy.agent.plist"

# Create directories
mkdir -p "$INSTALL_ROOT/Library/Application Support/family-policy"
mkdir -p "$INSTALL_ROOT/Library/Application Support/browser-extension-policy"

# Create scripts directory
SCRIPTS_DIR=$(mktemp -d)
trap "rm -rf $SCRIPTS_DIR" EXIT

# Create postinstall script
cat > "$SCRIPTS_DIR/postinstall" << 'EOF'
#!/bin/bash

# Create necessary directories
mkdir -p "/Library/Application Support/family-policy"
mkdir -p "/Library/Application Support/browser-extension-policy"
mkdir -p /var/log

# Set permissions
chmod 755 "/Library/Application Support/family-policy"
chmod 755 "/Library/Application Support/browser-extension-policy"

echo ""
echo "Family Policy Agent has been installed."
echo ""
echo "To configure and start the agent:"
echo "  1. sudo family-policy agent setup --url YOUR_GITHUB_POLICY_URL"
echo "  2. sudo family-policy agent install"
echo ""
echo "The agent will start automatically on the next boot."
echo "To start it now:"
echo "  sudo launchctl load /Library/LaunchDaemons/com.family-policy.agent.plist"
echo ""

exit 0
EOF

chmod 755 "$SCRIPTS_DIR/postinstall"

# Build the package
echo "Building PKG..."
mkdir -p dist

pkgbuild \
    --root "$INSTALL_ROOT" \
    --identifier "$IDENTIFIER" \
    --version "$VERSION" \
    --scripts "$SCRIPTS_DIR" \
    --install-location "/" \
    "dist/family-policy-${VERSION}.pkg"

echo
echo "✓ Package created: dist/family-policy-${VERSION}.pkg"
echo
echo "To install:"
echo "  sudo installer -pkg dist/family-policy-${VERSION}.pkg -target /"
echo
echo "To remove:"
echo "  Use packaging/macos/uninstall.sh"
