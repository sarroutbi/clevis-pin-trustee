<!--
SPDX-FileCopyrightText: Sergio Arroutbi <sarroutb@redhat.com>

SPDX-License-Identifier: CC0-1.0
-->

# Release Process for clevis-pin-trustee

This document describes the complete release process for clevis-pin-trustee, which uses [cargo-dist](https://github.com/axodotdev/cargo-dist) for automated binary releases.

## Table of Contents

- [Quick Start](#quick-start)
- [Detailed Release Process](#detailed-release-process)
- [Pre-Release Checklist](#pre-release-checklist)
- [Understanding the Automation](#understanding-the-automation)
- [GitHub Actions Workflow](#github-actions-workflow)
- [Testing Releases Locally](#testing-releases-locally)
- [Troubleshooting](#troubleshooting)
- [Manual Release (Fallback)](#manual-release-fallback)
- [Version Numbering](#version-numbering)

## Quick Start

For experienced maintainers, here's the TL;DR:

```bash
# 1. Update version numbers (if needed)
# 2. Create and push a tag
git tag -a v0.2.0 -m "Release version 0.2.0"
git push origin v0.2.0

# 3. That's it! GitHub Actions handles the rest
```

## Detailed Release Process

### Step 1: Pre-Release Preparation

Before creating a release, ensure:

1. **All tests pass locally**:
   ```bash
   cargo test --release
   cargo clippy --all-targets -- -D warnings
   cargo fmt -- --check
   ```

2. **Update version numbers** (if needed):

   The version is defined in two Cargo.toml files:
   - `cli/Cargo.toml` - the main binary package
   - `lib/Cargo.toml` - the library package

   Update the `version` field in both files:
   ```toml
   [package]
   version = "0.2.0"  # Update this
   ```

3. **Update CHANGELOG** (if you maintain one):
   - Document all changes since the last release
   - Include breaking changes, new features, bug fixes

4. **Commit version changes**:
   ```bash
   git add cli/Cargo.toml lib/Cargo.toml CHANGELOG.md
   git commit -m "chore: bump version to 0.2.0"
   git push
   ```

### Step 2: Create and Push the Release Tag

The release process is triggered by pushing a git tag that matches a version pattern.

1. **Create an annotated tag**:
   ```bash
   git tag -a v0.2.0 -m "Release version 0.2.0

   Summary of changes:
   - Feature: Added X functionality
   - Fix: Resolved Y issue
   - Improvement: Enhanced Z performance"
   ```

   **Important**:
   - Use **annotated tags** (`git tag -a`), not lightweight tags
   - The tag MUST start with `v` followed by a semantic version (e.g., `v0.2.0`, `v1.0.0-beta.1`)
   - The tag message will NOT be used for release notes (cargo-dist generates them)

2. **Push the tag to GitHub**:
   ```bash
   git push origin v0.2.0
   ```

   **This triggers the automated release process!**

### Step 3: Monitor the GitHub Actions Workflow

Once you push the tag, GitHub Actions automatically starts the release workflow.

#### Where to Monitor

1. Go to your repository on GitHub: https://github.com/sarroutbi/clevis-pin-trustee
2. Click the "Actions" tab
3. Look for the "Release" workflow run for your tag

#### What Happens During the Workflow

The workflow consists of several jobs that run in sequence:

```
plan → build-local-artifacts → build-global-artifacts → host → announce
```

**Job Descriptions**:

1. **`plan`** (Duration: ~30 seconds)
   - Installs cargo-dist
   - Determines what needs to be built
   - Creates a build manifest
   - **Outputs**: Build plan JSON

2. **`build-local-artifacts`** (Duration: ~5-10 minutes)
   - Runs in parallel for each platform:
     - `x86_64-unknown-linux-gnu` (on Ubuntu runner)
     - `aarch64-unknown-linux-gnu` (on Ubuntu runner with cross-compilation)
   - Builds release binaries with optimizations
   - Creates compressed tarballs
   - Generates SHA256 checksums
   - **Outputs**: Platform-specific `.tar.gz` files and checksums

3. **`build-global-artifacts`** (Duration: ~1 minute)
   - Creates cross-platform checksums
   - Generates installer scripts (if configured)
   - **Outputs**: Global artifact files

4. **`host`** (Duration: ~1-2 minutes)
   - Creates the GitHub Release
   - Uploads all artifacts to the release
   - Generates release notes from git history
   - **Outputs**: Published GitHub Release

5. **`announce`** (Duration: ~30 seconds)
   - Posts announcements (if configured)
   - Currently just validates the release succeeded

#### Monitoring Progress

Watch the workflow logs in real-time:

```
Actions → Release → [your tag] → [click on any job to see logs]
```

**Success indicators**:
- ✅ Green checkmarks on all jobs
- GitHub Release created at: `https://github.com/sarroutbi/clevis-pin-trustee/releases/tag/v0.2.0`

**Failure indicators**:
- ❌ Red X on any job
- See [Troubleshooting](#troubleshooting) section below

### Step 4: Verify the Release

After the workflow completes successfully:

1. **Check the GitHub Release page**:
   ```
   https://github.com/sarroutbi/clevis-pin-trustee/releases/tag/v0.2.0
   ```

2. **Verify artifacts are present**:
   - `clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz`
   - `clevis-pin-trustee-0.2.0-aarch64-unknown-linux-gnu.tar.gz`
   - Checksum files for each tarball

3. **Test a release artifact**:
   ```bash
   # Download and extract
   wget https://github.com/sarroutbi/clevis-pin-trustee/releases/download/v0.2.0/clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz
   tar xzf clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz
   cd clevis-pin-trustee-0.2.0

   # Verify checksum
   sha256sum -c ../clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256

   # Test the binary
   ./bin/clevis-pin-trustee --help
   ```

4. **Update release notes** (optional):
   - GitHub Release notes are auto-generated from commits
   - You can manually edit them to add more context
   - Click "Edit" on the release page to customize

## Pre-Release Checklist

Before pushing a release tag, verify:

- [ ] All tests pass: `cargo test --release`
- [ ] Linting passes: `cargo clippy --all-targets -- -D warnings`
- [ ] Formatting is correct: `cargo fmt -- --check`
- [ ] Version numbers updated in `cli/Cargo.toml` and `lib/Cargo.toml`
- [ ] CHANGELOG updated (if applicable)
- [ ] Git working directory is clean: `git status`
- [ ] On the correct branch (usually `main`)
- [ ] Latest changes pulled: `git pull`
- [ ] REUSE compliance: `reuse lint` (if reuse tool is installed)

## Understanding the Automation

### What is cargo-dist?

cargo-dist is a tool that automates the distribution of Rust binaries. It:

- Builds binaries for multiple platforms
- Creates release archives (`.tar.gz`)
- Generates checksums for verification
- Publishes GitHub Releases
- Can create installers (shell scripts, PowerShell, etc.)

### Configuration Files

The automation is configured in these files:

1. **`dist-workspace.toml`** - Main cargo-dist configuration
   ```toml
   [dist]
   cargo-dist-version = "0.30.2"
   ci = "github"
   installers = []
   targets = ["aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"]
   include = ["clevis-encrypt-trustee", "clevis-decrypt-trustee"]
   ```

2. **`Cargo.toml`** - Workspace configuration with dist profile
   ```toml
   [profile.dist]
   inherits = "release"
   lto = "thin"
   ```

3. **`.github/workflows/release.yml`** - Auto-generated GitHub Actions workflow
   - **DO NOT EDIT MANUALLY** - regenerate with `cargo dist generate`

### Platforms Built

Currently configured to build for:

- **x86_64-unknown-linux-gnu** - 64-bit Linux (Intel/AMD)
- **aarch64-unknown-linux-gnu** - 64-bit Linux (ARM64)

These are the primary platforms where Clevis/LUKS are used. Windows and macOS are not included since LUKS is Linux-specific.

### What Gets Included in Releases

Each release tarball contains:

```
clevis-pin-trustee-X.Y.Z/
├── bin/
│   ├── clevis-pin-trustee          # Main binary
│   ├── clevis-encrypt-trustee      # Wrapper script
│   └── clevis-decrypt-trustee      # Wrapper script
```

## GitHub Actions Workflow

### Workflow File Location

`.github/workflows/release.yml`

### Trigger Conditions

The workflow runs when:

1. **A version tag is pushed**: Any tag matching `**[0-9]+.[0-9]+.[0-9]+*`
   - Examples: `v0.1.0`, `v1.0.0`, `v2.1.3-beta.1`, `clevis-pin-trustee/0.2.0`

2. **Pull Requests** (for testing only):
   - Builds artifacts but doesn't create a release
   - Useful for validating changes to the release process

### Workflow Permissions

The workflow requires these permissions:

```yaml
permissions:
  contents: write  # To create GitHub Releases and upload assets
```

These are already configured in the workflow file.

### Secrets Required

No secrets are required! The workflow uses the built-in `GITHUB_TOKEN` which is automatically provided by GitHub Actions.

### Workflow Environment Variables

Key variables set during the workflow:

- `GH_TOKEN`: GitHub token for API access
- `BUILD_MANIFEST_NAME`: Path to build manifest JSON
- `CARGO_TERM_COLOR`: Always set to colorize cargo output

### Manual Workflow Trigger

You cannot manually trigger the release workflow - it only runs on tag pushes. However, you can test the workflow on a PR by:

1. Creating a branch with changes to `.github/workflows/release.yml`
2. Opening a PR
3. The workflow runs in "plan mode" without publishing

## Testing Releases Locally

Before pushing a tag, you can test the release build locally:

### Build Release Binaries Locally

```bash
# Build with the dist profile (same as CI)
cargo build --profile dist

# The binary will be at:
ls -lh target/dist/clevis-pin-trustee
```

### Test cargo-dist Locally

Install cargo-dist:

```bash
cargo install cargo-dist
```

Generate what the release would look like:

```bash
# Plan the release (doesn't build, just shows what would happen)
cargo dist plan

# Build release artifacts locally (creates tarballs)
cargo dist build --artifacts all

# Outputs will be in target/distrib/
```

### Validate Configuration

Check if cargo-dist config is valid:

```bash
cargo dist init --yes  # Re-run init to validate config
cargo dist generate    # Regenerate workflow files
git diff               # Check if anything changed
```

## Troubleshooting

### Common Issues and Solutions

#### Issue: "REUSE compliance check failed"

**Symptom**: The workflow fails with missing license errors.

**Solution**:
```bash
# Check which licenses are missing
reuse lint

# Download missing SPDX licenses
reuse download --all

# Add custom licenses manually to LICENSES/
# Commit the new license files
git add LICENSES/
git commit -m "Add missing license files"
git push
```

#### Issue: "Tag already exists"

**Symptom**: Cannot create tag because it exists locally or remotely.

**Solution**:
```bash
# Delete local tag
git tag -d v0.2.0

# Delete remote tag (careful!)
git push origin :refs/tags/v0.2.0

# Re-create the tag
git tag -a v0.2.0 -m "Release version 0.2.0"
git push origin v0.2.0
```

#### Issue: "Build failed for platform X"

**Symptom**: The `build-local-artifacts` job fails for a specific platform.

**Solution**:
1. Check the job logs for the specific error
2. Common causes:
   - Missing system dependencies
   - Cross-compilation issues
   - Incompatible dependencies for target platform
3. Test locally with:
   ```bash
   # For aarch64
   cargo build --target aarch64-unknown-linux-gnu --release
   ```

#### Issue: "Version mismatch between tag and Cargo.toml"

**Symptom**: Warning about version mismatch in cargo-dist output.

**Solution**:
- The tag version (e.g., `v0.2.0`) should match the version in `cli/Cargo.toml`
- Update Cargo.toml and create a new tag, or delete and recreate the tag

#### Issue: "Workflow doesn't trigger"

**Symptom**: Pushed a tag but GitHub Actions doesn't run.

**Checks**:
1. Tag format is correct (must match `**[0-9]+.[0.9]+.[0-9]+*`)
2. Tag was pushed to the correct repository
3. GitHub Actions is enabled in repository settings
4. Check `.github/workflows/release.yml` exists on the default branch

### Debugging Failed Releases

If a release fails:

1. **Review the workflow logs**:
   - Actions → Release → [failed run] → [click on failed job]

2. **Check the build manifest**:
   - Download the `artifacts-plan-dist-manifest` artifact
   - Examine `plan-dist-manifest.json` for the build plan

3. **Re-run failed jobs**:
   - On the workflow run page, click "Re-run failed jobs"
   - GitHub Actions will retry only the failed parts

4. **Cancel and retry**:
   - If needed, delete the tag and re-push:
     ```bash
     git tag -d v0.2.0
     git push origin :refs/tags/v0.2.0
     git tag -a v0.2.0 -m "Release version 0.2.0"
     git push origin v0.2.0
     ```

### Getting Help

If you encounter issues not covered here:

1. Check cargo-dist documentation: https://opensource.axo.dev/cargo-dist/
2. Review cargo-dist GitHub issues: https://github.com/axodotdev/cargo-dist/issues
3. Check the release workflow syntax: https://docs.github.com/en/actions/

## Manual Release (Fallback)

If the automated release process is broken, you can create a release manually.

### Step 1: Build Binaries

```bash
# Clean build
cargo clean

# Build release binaries
cargo build --release

# Strip symbols to reduce size
strip target/release/clevis-pin-trustee
```

### Step 2: Create Release Archive

```bash
# Create directory structure
mkdir -p release-artifacts/clevis-pin-trustee-0.2.0/bin

# Copy binaries and scripts
cp target/release/clevis-pin-trustee release-artifacts/clevis-pin-trustee-0.2.0/bin/
cp clevis-encrypt-trustee release-artifacts/clevis-pin-trustee-0.2.0/bin/
cp clevis-decrypt-trustee release-artifacts/clevis-pin-trustee-0.2.0/bin/

# Create tarball
cd release-artifacts
tar czf clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz clevis-pin-trustee-0.2.0/

# Generate checksum
sha256sum clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz > \
    clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```

### Step 3: Create GitHub Release Manually

1. Go to: https://github.com/sarroutbi/clevis-pin-trustee/releases/new
2. Select the tag (or create it): `v0.2.0`
3. Fill in:
   - **Release title**: `v0.2.0`
   - **Description**: Add release notes
4. Upload artifacts:
   - `clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz`
   - `clevis-pin-trustee-0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256`
5. Click "Publish release"

## Version Numbering

This project follows [Semantic Versioning](https://semver.org/):

### Format: MAJOR.MINOR.PATCH

- **MAJOR** version: Incompatible API changes
- **MINOR** version: New functionality (backwards-compatible)
- **PATCH** version: Bug fixes (backwards-compatible)

### Examples

- `v0.1.0` → `v0.2.0` - Added new features
- `v0.2.0` → `v0.2.1` - Bug fixes only
- `v0.2.1` → `v1.0.0` - First stable release or breaking changes
- `v1.0.0` → `v1.0.1` - Bug fixes
- `v1.0.1` → `v1.1.0` - New features

### Pre-release Versions

For pre-releases, use suffixes:

- `v0.2.0-alpha.1` - Alpha release
- `v0.2.0-beta.1` - Beta release
- `v0.2.0-rc.1` - Release candidate

Pre-release tags will create GitHub Releases marked as "Pre-release".

## Regenerating Workflow Files

If you modify `dist-workspace.toml`, regenerate the workflow:

```bash
# Install/update cargo-dist
cargo install cargo-dist

# Regenerate workflow file
cargo dist generate

# Review changes
git diff .github/workflows/release.yml

# Commit if changes look good
git add .github/workflows/release.yml dist-workspace.toml
git commit -m "Update cargo-dist configuration"
git push
```

**Important**: Always regenerate after changing cargo-dist configuration to keep the workflow in sync.

## Summary

The release process is now fully automated with cargo-dist:

1. **Prepare**: Update versions, run tests
2. **Tag**: Create and push a version tag
3. **Wait**: GitHub Actions builds and releases
4. **Verify**: Check the release page and test artifacts

For any questions or issues with the release process, refer to this document or the cargo-dist documentation.
