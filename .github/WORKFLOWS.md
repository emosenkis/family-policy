# GitHub Actions Workflows

This document explains the CI/CD workflows configured for this project.

## Workflows

### Release Builds (`release.yml`)

**Trigger**: Automatically runs when a new release is created on GitHub.

**Purpose**: Builds binaries and packages for all supported platforms and attaches them to the GitHub release.

#### What It Does

1. **Build Linux (x86_64)**
   - Builds release binary for Linux
   - Uploads binary artifact

2. **Build macOS (Universal)**
   - Builds for Intel (x86_64)
   - Builds for Apple Silicon (aarch64)
   - Creates universal binary (works on both)
   - Creates PKG installer
   - Uploads all artifacts

3. **Build Windows (x86_64)**
   - Builds release binary for Windows
   - Creates MSI installer package
   - Creates ZIP archive with installation scripts (legacy)
   - Uploads all artifacts

4. **Create Release Assets**
   - Downloads all artifacts from previous jobs
   - Generates SHA256 checksums
   - Attaches everything to the GitHub release

#### Platform Build Matrix

| Platform | Runner | Target | Output |
|----------|--------|--------|--------|
| Linux | ubuntu-latest | x86_64-unknown-linux-gnu | Binary |
| macOS | macos-latest | x86_64/aarch64-apple-darwin | Universal Binary, PKG |
| Windows | windows-latest | x86_64-pc-windows-msvc | Binary, MSI, ZIP |

#### Caching

All builds use cargo caching to speed up subsequent runs:
- Cargo registry
- Cargo git dependencies
- Cargo build artifacts

This significantly reduces build times for releases.

## Creating a Release

### Method 1: Via GitHub Web Interface

1. **Go to your repository** on GitHub
2. **Click "Releases"** in the right sidebar
3. **Click "Create a new release"**
4. **Choose a tag**: Enter a new tag (e.g., `v0.1.0`)
   - Use semantic versioning: `vMAJOR.MINOR.PATCH`
5. **Target**: Select branch (usually `main`)
6. **Release title**: e.g., "Family Policy v0.1.0"
7. **Description**: Use the template from `.github/RELEASE_TEMPLATE.md`
8. **Click "Publish release"**

The workflow will automatically:
- Build for all platforms
- Create packages
- Upload assets to the release

### Method 2: Via Git Command Line

```bash
# 1. Ensure you're on main and up to date
git checkout main
git pull

# 2. Update version numbers (see checklist below)
# Edit: Cargo.toml, packaging scripts, etc.

# 3. Commit version bump
git add -A
git commit -m "Bump version to 0.1.0"

# 4. Create and push tag
git tag -a v0.1.0 -m "Release version 0.1.0"
git push origin main
git push origin v0.1.0

# 5. Create release on GitHub
# Go to GitHub and create release from the v0.1.0 tag
# Or use GitHub CLI:
gh release create v0.1.0 \
  --title "Family Policy v0.1.0" \
  --notes-file .github/RELEASE_TEMPLATE.md
```

### Manual Workflow Trigger

You can also run the workflow manually without creating a release:

1. **Go to Actions tab** in your GitHub repository
2. **Select "Release Builds"** workflow
3. **Click "Run workflow"** dropdown
4. **Select branch** (usually `main`)
5. **Enter version** (e.g., `v0.1.0`)
6. **Click "Run workflow"**

This is useful for:
- Testing the workflow
- Creating pre-release builds
- Building specific versions

## Artifacts

### Available Artifacts

After a successful workflow run, these artifacts are created:

1. **Binaries** (standalone executables)
   - `family-policy-linux-x86_64`
   - `family-policy-macos-universal`
   - `family-policy-windows-x86_64.exe`

2. **macOS Package**
   - `family-policy-VERSION.pkg`

3. **Windows Packages**
   - `family-policy-VERSION-x86_64.msi` (Installer)
   - `family-policy-windows-x86_64.zip` (Legacy manual install)

4. **Checksums**
   - `SHA256SUMS` (checksums for all files)

### Downloading Artifacts

**From Release** (public):
```bash
# Latest release
wget https://github.com/USERNAME/family-policy/releases/latest/download/family-policy-linux-x86_64

# Specific version
wget https://github.com/USERNAME/family-policy/releases/download/v0.1.0/family-policy-linux-x86_64
```

**From Workflow Run** (requires authentication):
- Go to Actions tab
- Click on the workflow run
- Scroll to "Artifacts" section
- Click to download

## Pre-Release Checklist

Before creating a release, ensure:

- [ ] All tests pass: `cargo test`
- [ ] Code builds on all platforms (or trust CI)
- [ ] Version updated in:
  - [ ] `Cargo.toml`
  - [ ] `packaging/macos/build-pkg.sh`
  - [ ] WiX configuration (auto-synced from Cargo.toml)
- [ ] CHANGELOG.md updated (if you maintain one)
- [ ] Documentation is current
- [ ] All changes are committed
- [ ] Main branch is pushed to GitHub

## Workflow Secrets

No secrets are required for the current workflow. However, if you add features that need secrets (e.g., code signing), add them in:

**Repository Settings** → **Secrets and variables** → **Actions** → **New repository secret**

Common secrets for future use:
- `APPLE_CERTIFICATE` - For macOS code signing
- `WINDOWS_CERTIFICATE` - For Windows code signing
- `DEPLOY_KEY` - For automatic deployment

## Troubleshooting

### Workflow Fails on Build

**Check the logs:**
1. Go to Actions tab
2. Click on the failed workflow
3. Click on the failed job
4. Expand the failed step

Common issues:
- **Cargo build fails**: Check for compilation errors in logs
- **Tests fail**: Fix tests before releasing
- **Packaging fails**: Ensure package scripts are executable and correct

### Missing Artifacts

**Issue**: Some artifacts aren't uploaded

**Solution**:
- Check that the build completed successfully
- Verify artifact paths in workflow YAML
- Ensure `dist/` directory is created correctly

### Artifacts Not Attached to Release

**Issue**: Workflow succeeds but assets don't appear on release

**Causes**:
- Workflow triggered by workflow_dispatch instead of release event
- GitHub token permissions issue
- Artifact paths incorrect

**Solution**:
- Ensure release was created (not just a tag)
- Check workflow run logs for upload errors

### Build Times Too Long

**Current optimizations:**
- Cargo caching enabled
- Only build on release (not every push)
- Parallel builds for different platforms

**Further optimizations:**
- Use sccache for distributed caching
- Reduce dependencies where possible
- Enable LTO only for releases

## Monitoring

### View Workflow Status

**Badge** (add to README.md):
```markdown
[![Release](https://github.com/USERNAME/family-policy/actions/workflows/release.yml/badge.svg)](https://github.com/USERNAME/family-policy/actions/workflows/release.yml)
```

**GitHub Actions Tab**:
- View all workflow runs
- Filter by event, status, branch
- Re-run failed workflows
- Download workflow logs

### Notifications

GitHub sends notifications for:
- Workflow failures (to repository owner)
- Successful releases (if watching repository)

Configure in: **GitHub Settings** → **Notifications**

## Future Enhancements

Potential workflow improvements:

1. **Continuous Integration** (on every push)
   - Run tests
   - Check formatting (`cargo fmt --check`)
   - Run clippy (`cargo clippy`)
   - Security audit (`cargo audit`)

2. **Code Signing**
   - macOS: Sign with Apple Developer certificate
   - Windows: Sign with Authenticode certificate

3. **Automatic Deployment**
   - Deploy to package repositories
   - Update Homebrew formula
   - Update Chocolatey package

4. **Release Notes**
   - Auto-generate from commit messages
   - Parse conventional commits
   - Create changelog automatically

5. **Pre-release Builds**
   - Build on every commit to main
   - Tag as pre-release
   - Allow testing before official release

6. **Matrix Testing**
   - Test on multiple OS versions
   - Test on multiple Rust versions
   - Test with different feature flags

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Cargo Book - Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
