#!/bin/bash
# Build Debian package for family-policy
set -e

VERSION="0.1.0"
ARCH="amd64"
PACKAGE_NAME="family-policy_${VERSION}_${ARCH}"

echo "Building Debian package: $PACKAGE_NAME"
echo

# Build the binary in release mode
echo "Building release binary..."
cargo build --release

# Create package directory structure
echo "Creating package structure..."
PACKAGE_DIR="packaging/debian/$PACKAGE_NAME"
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"

# Copy DEBIAN control files
mkdir -p "$PACKAGE_DIR/DEBIAN"
cp packaging/debian/DEBIAN/* "$PACKAGE_DIR/DEBIAN/"

# Install binary
mkdir -p "$PACKAGE_DIR/usr/local/bin"
cp target/release/family-policy "$PACKAGE_DIR/usr/local/bin/"
chmod 755 "$PACKAGE_DIR/usr/local/bin/family-policy"

# Install systemd service
mkdir -p "$PACKAGE_DIR/etc/systemd/system"
cp packaging/linux/family-policy-agent.service "$PACKAGE_DIR/etc/systemd/system/"
chmod 644 "$PACKAGE_DIR/etc/systemd/system/family-policy-agent.service"

# Create directories
mkdir -p "$PACKAGE_DIR/etc/family-policy"
mkdir -p "$PACKAGE_DIR/var/lib/browser-extension-policy"

# Build the package
echo "Building .deb package..."
dpkg-deb --build --root-owner-group "$PACKAGE_DIR"

# Move to output directory
mkdir -p dist
mv "$PACKAGE_DIR.deb" "dist/${PACKAGE_NAME}.deb"

echo
echo "✓ Package created: dist/${PACKAGE_NAME}.deb"
echo
echo "To install:"
echo "  sudo dpkg -i dist/${PACKAGE_NAME}.deb"
echo
echo "To remove:"
echo "  sudo dpkg -r family-policy"
