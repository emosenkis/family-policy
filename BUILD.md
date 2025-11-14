# Building Family Policy

This document describes how to build the Family Policy binary for different platforms.

## Prerequisites

### All Platforms
- **Rust toolchain**: Install from https://rustup.rs
- **Git**: For cloning the repository

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Building for Your Current Platform

The simplest way to build is for your current platform:

```bash
# Clone the repository
git clone https://github.com/username/family-policy.git
cd family-policy

# Build in debug mode (faster compilation)
cargo build

# Build in release mode (optimized)
cargo build --release

# Binary location:
# Debug: target/debug/family-policy
# Release: target/release/family-policy (or .exe on Windows)
```

## Cross-Compilation

### Building for Linux

#### On Linux (native)
```bash
cargo build --release
```

#### On macOS or Windows
```bash
# Install Linux target
rustup target add x86_64-unknown-linux-gnu

# Build (requires cross-compilation toolchain)
cargo build --release --target x86_64-unknown-linux-gnu
```

### Building for macOS

#### On macOS (native)
```bash
# Build for Intel Macs
cargo build --release --target x86_64-apple-darwin

# Build for Apple Silicon
cargo build --release --target aarch64-apple-darwin

# Create universal binary (works on both)
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

lipo -create \
  target/x86_64-apple-darwin/release/family-policy \
  target/aarch64-apple-darwin/release/family-policy \
  -output target/universal-apple-darwin/release/family-policy
```

#### From Linux or Windows
Cross-compilation to macOS from other platforms requires the Xcode SDK and is complex.
**Recommendation**: Use GitHub Actions or build on macOS.

### Building for Windows

#### On Windows (native)
```bash
# MSVC toolchain (recommended on Windows)
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

#### From Linux (using Cross)
```bash
# Install cross (containerized cross-compilation)
cargo install cross --git https://github.com/cross-rs/cross

# Requires Docker to be installed and running
cross build --release --target x86_64-pc-windows-gnu
```

#### From Linux (using MinGW - alternative)
```bash
# Install MinGW toolchain
# Debian/Ubuntu:
sudo apt-get install gcc-mingw-w64-x86-64

# Fedora:
sudo dnf install mingw64-gcc

# Add Windows target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

#### From macOS
```bash
# Install MinGW via Homebrew
brew install mingw-w64

# Add Windows target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

## Automated Builds with GitHub Actions

The easiest way to build for all platforms is to use the included GitHub Actions workflow:

### Creating a Release

1. **Tag your release:**
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. **Create a GitHub release:**
   - Go to your repository on GitHub
   - Click "Releases" → "Create a new release"
   - Select your tag (v0.1.0)
   - Add release notes
   - Click "Publish release"

3. **GitHub Actions will automatically:**
   - Build for Linux (x86_64)
   - Build for macOS (Universal - Intel + Apple Silicon)
   - Build for Windows (x86_64)
   - Create DEB and RPM packages
   - Create macOS PKG installer
   - Create Windows ZIP archive
   - Upload all artifacts to the release

### Manual Workflow Trigger

You can also trigger builds manually without creating a release:

1. Go to Actions tab in your GitHub repository
2. Select "Release Builds" workflow
3. Click "Run workflow"
4. Enter a version tag (e.g., v0.1.0)
5. Click "Run workflow"

Artifacts will be available for download from the workflow run.

## Build Targets Reference

| Target                        | Platform              | Notes                          |
|-------------------------------|-----------------------|--------------------------------|
| `x86_64-unknown-linux-gnu`    | Linux (glibc)         | Most common Linux distros      |
| `x86_64-unknown-linux-musl`   | Linux (musl)          | Static binary, Alpine Linux    |
| `x86_64-apple-darwin`         | macOS Intel           | Intel Macs                     |
| `aarch64-apple-darwin`        | macOS Apple Silicon   | M1/M2/M3 Macs                  |
| `x86_64-pc-windows-msvc`      | Windows (MSVC)        | Recommended for Windows        |
| `x86_64-pc-windows-gnu`       | Windows (MinGW)       | Alternative, needs MinGW       |

## Testing Builds

After building, test the binary:

```bash
# Check version
./target/release/family-policy --version

# Run tests
cargo test

# Test local mode (requires sudo)
sudo ./target/release/family-policy --config examples/policy.yaml --dry-run

# Test agent mode
./target/release/family-policy agent --help
```

## Creating Packages

See [packaging/README.md](packaging/README.md) for detailed instructions on creating platform-specific packages.

### Quick Reference

```bash
# Debian/Ubuntu (.deb)
cd packaging/debian && ./build-deb.sh

# Fedora/RHEL (.rpm)
cd packaging/rpm && ./build-rpm.sh

# macOS (.pkg) - requires macOS
cd packaging/macos && ./build-pkg.sh

# Windows (ZIP)
# Build on Windows, then package with PowerShell scripts
```

## Troubleshooting

### Linking Errors on Linux

If you encounter linking errors:

```bash
# Install build dependencies
# Debian/Ubuntu:
sudo apt-get install build-essential pkg-config libssl-dev

# Fedora:
sudo dnf install gcc pkg-config openssl-devel
```

### OpenSSL Errors

The project uses `rustls` for TLS, which doesn't require OpenSSL. If you still get errors:

```bash
# Ensure you're using the correct features
cargo clean
cargo build --release --no-default-features --features rustls-tls
```

### Cross-Compilation Issues

If cross-compilation fails:

1. Use `cross` instead of `cargo`:
   ```bash
   cross build --release --target <target-triple>
   ```

2. Or build on the target platform directly

3. Or use GitHub Actions (recommended for releases)

### Windows: Missing DLLs

If the Windows binary doesn't run due to missing DLLs:

- Use the MSVC target (`x86_64-pc-windows-msvc`) instead of GNU
- Or install the Visual C++ Redistributable on the target machine
- Or statically link: `RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-pc-windows-gnu`

## Development Builds

For faster iteration during development:

```bash
# Build in debug mode (much faster)
cargo build

# Run without building (builds if needed)
cargo run -- --help

# Run tests
cargo test

# Run with verbose output
cargo run -- --verbose agent status

# Check code without building
cargo check
```

## Release Checklist

Before creating a release:

1. ✅ Update version in `Cargo.toml`
2. ✅ Update version in packaging scripts:
   - `packaging/debian/DEBIAN/control`
   - `packaging/debian/build-deb.sh`
   - `packaging/rpm/family-policy.spec`
   - `packaging/rpm/build-rpm.sh`
   - `packaging/macos/build-pkg.sh`
3. ✅ Update CHANGELOG.md
4. ✅ Run full test suite: `cargo test`
5. ✅ Test builds on all platforms (or use GitHub Actions)
6. ✅ Test packages install correctly
7. ✅ Update documentation if needed
8. ✅ Commit all changes
9. ✅ Create and push tag
10. ✅ Create GitHub release

## Continuous Integration

The project uses GitHub Actions for CI/CD:

- **`.github/workflows/release.yml`**: Builds all platforms on release creation
- Artifacts are automatically attached to releases
- All builds are cached for faster subsequent runs

## Binary Size Optimization

To reduce binary size:

```bash
# Add to Cargo.toml:
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols

# Then build
cargo build --release
```

Current release binary sizes (approximate):
- Linux: ~8-10 MB
- macOS: ~9-11 MB (universal)
- Windows: ~8-10 MB

## Platform-Specific Notes

### Linux
- Uses system OpenSSL by default (via rustls - no OpenSSL needed)
- Binaries are dynamically linked to glibc
- For static binaries, use musl target

### macOS
- Universal binaries work on both Intel and Apple Silicon
- Requires Xcode Command Line Tools for building
- Binaries are code-signed automatically (ad-hoc signature)

### Windows
- MSVC is recommended for native builds
- MinGW works but may have compatibility issues
- Binaries include necessary CRT libraries

## Getting Help

- Build issues: Check GitHub Issues
- Cross-compilation: Consider using GitHub Actions
- Package creation: See packaging/README.md
