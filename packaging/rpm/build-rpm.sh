#!/bin/bash
# Build RPM package for family-policy
set -e

VERSION="0.1.0"
RELEASE="1"
ARCH="x86_64"

echo "Building RPM package: family-policy-${VERSION}-${RELEASE}"
echo

# Build the binary in release mode
echo "Building release binary..."
cargo build --release

# Create RPM build directories
echo "Setting up RPM build environment..."
RPMBUILD_DIR="$HOME/rpmbuild"
mkdir -p "$RPMBUILD_DIR"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Create source tarball
echo "Creating source tarball..."
TEMP_DIR=$(mktemp -d)
SOURCE_DIR="$TEMP_DIR/family-policy-$VERSION"
mkdir -p "$SOURCE_DIR"

# Copy files to source directory
cp target/release/family-policy "$SOURCE_DIR/"
cp packaging/linux/family-policy-agent.service "$SOURCE_DIR/"

# Create tarball
cd "$TEMP_DIR"
tar czf "$RPMBUILD_DIR/SOURCES/family-policy-$VERSION.tar.gz" "family-policy-$VERSION/"
cd -

# Copy spec file
cp packaging/rpm/family-policy.spec "$RPMBUILD_DIR/SPECS/"

# Build the RPM
echo "Building RPM..."
rpmbuild -bb "$RPMBUILD_DIR/SPECS/family-policy.spec"

# Copy to dist directory
mkdir -p dist
cp "$RPMBUILD_DIR/RPMS/$ARCH/family-policy-${VERSION}-${RELEASE}."*".${ARCH}.rpm" dist/ 2>/dev/null || \
cp "$RPMBUILD_DIR/RPMS/$ARCH/family-policy-${VERSION}-${RELEASE}.${ARCH}.rpm" dist/ 2>/dev/null || \
cp "$RPMBUILD_DIR/RPMS/"*"/family-policy-${VERSION}-${RELEASE}."*".rpm" dist/

# Clean up
rm -rf "$TEMP_DIR"

echo
echo "✓ Package created in dist/ directory"
echo
echo "To install:"
echo "  sudo rpm -i dist/family-policy-*.rpm"
echo
echo "To remove:"
echo "  sudo rpm -e family-policy"
