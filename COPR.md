<!--
SPDX-FileCopyrightText: Sergio Arroutbi <sarroutb@redhat.com>

SPDX-License-Identifier: CC0-1.0
-->

# COPR Build Instructions

This document explains how to build and publish clevis-pin-trustee RPM packages to COPR for RHEL9 and RHEL10.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Manual Build Process](#manual-build-process)
- [COPR Project Setup](#copr-project-setup)
- [Testing the Package](#testing-the-package)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Tools

Install the necessary tools on your Fedora/RHEL system:

```bash
# Install COPR CLI and RPM build tools
sudo dnf install copr-cli rpm-build rpmdevtools

# Ensure Rust toolchain is installed
cargo --version
```

### COPR Account and API Token

1. Create a COPR account at: https://copr.fedorainfracloud.org/
2. Generate an API token: https://copr.fedorainfracloud.org/api/
3. Save it to `~/.config/copr`:
   ```bash
   mkdir -p ~/.config
   # Copy the configuration from the COPR web UI
   ```

## Quick Start

Use the automated build script:

```bash
# Build and upload to COPR
./copr-build.sh 0.1.0
```

The script will:
1. Create vendored dependencies
2. Generate source tarball
3. Build SRPM
4. Optionally upload to COPR

## Manual Build Process

If you prefer to build manually, follow these steps:

### Step 1: Create Vendored Dependencies

```bash
# Clean previous vendor directory
rm -rf vendor .cargo/config.toml

# Vendor all Rust dependencies
cargo vendor --versioned-dirs vendor
```

### Step 2: Configure Vendored Sources

```bash
# Create cargo config
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF
```

### Step 3: Create Vendored Tarball

```bash
VERSION="0.1.0"
PACKAGE="clevis-pin-trustee"
TARBALL="${PACKAGE}-${VERSION}-vendor.tar.gz"

# Archive git content
git archive --format=tar --prefix=${PACKAGE}-${VERSION}/ HEAD | gzip > /tmp/git-archive.tar.gz

# Extract and add vendor directory
mkdir -p /tmp/${PACKAGE}-${VERSION}
cd /tmp && tar xzf git-archive.tar.gz
cp -r ${OLDPWD}/vendor ${PACKAGE}-${VERSION}/
cp -r ${OLDPWD}/.cargo ${PACKAGE}-${VERSION}/

# Create final tarball
tar czf ${TARBALL} ${PACKAGE}-${VERSION}/
mv ${TARBALL} ${OLDPWD}/
cd ${OLDPWD}

echo "Created: ${TARBALL}"
```

### Step 4: Build SRPM

```bash
# Setup RPM build tree
rpmdev-setuptree

# Copy source and spec
cp clevis-pin-trustee-0.1.0-vendor.tar.gz ~/rpmbuild/SOURCES/
cp clevis-pin-trustee.spec ~/rpmbuild/SPECS/

# Build source RPM
rpmbuild -bs ~/rpmbuild/SPECS/clevis-pin-trustee.spec

# The SRPM will be in ~/rpmbuild/SRPMS/
```

### Step 5: Upload to COPR

```bash
# Find the SRPM
SRPM=$(ls -t ~/rpmbuild/SRPMS/clevis-pin-trustee-0.1.0-*.src.rpm | head -1)

# Upload to COPR for RHEL9 and RHEL10
copr-cli build sarroutb/clevis-pin-trustee \
    --chroot epel-9-x86_64 \
    --chroot epel-9-aarch64 \
    --chroot epel-10-x86_64 \
    --chroot epel-10-aarch64 \
    ${SRPM}
```

## COPR Project Setup

### Creating the COPR Project

If you haven't created the COPR project yet:

1. Go to: https://copr.fedorainfracloud.org/coprs/add/
2. Fill in:
   - **Project name**: `clevis-pin-trustee`
   - **Chroots**: Select EPEL 9 and EPEL 10 (both x86_64 and aarch64)
   - **Description**: `Clevis PIN for Trustee attestation`
   - **Instructions**: Add installation instructions
3. Click "Create"

### Project Settings

Recommended COPR project settings:

- **Enable networking**: Yes (required for building, though actual build is offline)
- **Auto-rebuild**: No (manual rebuilds only)
- **Follow Fedora branching**: No (we target EPEL specifically)

## Available Chroots

The package supports these build targets:

- `epel-9-x86_64` - RHEL9 / CentOS Stream 9 (x86_64)
- `epel-9-aarch64` - RHEL9 / CentOS Stream 9 (ARM64)
- `epel-10-x86_64` - RHEL10 (x86_64)
- `epel-10-aarch64` - RHEL10 (ARM64)

## Installation Instructions

After the package is built in COPR, users can install it:

### RHEL9 / CentOS Stream 9

```bash
# Enable COPR repository
sudo dnf copr enable sarroutb/clevis-pin-trustee

# Install the package
sudo dnf install clevis-pin-trustee
```

### RHEL10

```bash
# Enable COPR repository
sudo dnf copr enable sarroutb/clevis-pin-trustee

# Install the package
sudo dnf install clevis-pin-trustee
```

## Testing the Package

After building, you can test the package locally:

### Install from COPR

```bash
# Enable the repository
sudo dnf copr enable sarroutb/clevis-pin-trustee

# Install
sudo dnf install clevis-pin-trustee

# Verify installation
clevis-pin-trustee --version
which clevis-encrypt-trustee
which clevis-decrypt-trustee
```

### Test Basic Functionality

```bash
# Test encryption (requires a running Trustee server)
echo "test data" | clevis-pin-trustee encrypt '{"servers":[{"url":"http://localhost:8080","cert":""}],"path":"test/path"}'

# Test decrypt
# (output from encrypt) | clevis-pin-trustee decrypt
```

### Local RPM Build Test

Before uploading to COPR, test the build locally:

```bash
# Build locally
rpmbuild -ba ~/rpmbuild/SPECS/clevis-pin-trustee.spec

# Install the built RPM
sudo dnf install ~/rpmbuild/RPMS/x86_64/clevis-pin-trustee-0.1.0-1.*.x86_64.rpm

# Test
clevis-pin-trustee --version
```

## Troubleshooting

### Build Failures

#### Missing Dependencies

If the build fails with missing Rust dependencies:

```
Error: This package requires rust >= 1.85.0
```

**Solution**: The EPEL buildroots should have Rust 1.85+. If not, the build will fail and you may need to wait for EPEL updates or use a different approach.

#### Vendor Directory Issues

If the build fails with "can't find crate":

```
error: no matching package named `xxx` found
```

**Solution**: Regenerate the vendor directory:

```bash
rm -rf vendor .cargo
cargo vendor --versioned-dirs vendor
```

#### SRPM Build Errors

If `rpmbuild` fails:

```
error: File not found: /home/user/rpmbuild/SOURCES/clevis-pin-trustee-0.1.0-vendor.tar.gz
```

**Solution**: Ensure the tarball is in the correct location:

```bash
cp clevis-pin-trustee-0.1.0-vendor.tar.gz ~/rpmbuild/SOURCES/
```

### COPR Upload Issues

#### Authentication Errors

```
Error: Login invalid/expired. Please visit https://copr.fedorainfracloud.org/api/
```

**Solution**: Regenerate your COPR API token and update `~/.config/copr`.

#### Permission Denied

```
Error: You don't have permissions to build in this project
```

**Solution**: Ensure you're the owner of the COPR project or have been granted permissions.

### Runtime Issues

#### Binary Not Found

After installation, if `clevis-pin-trustee` is not found:

```bash
# Verify package installation
rpm -ql clevis-pin-trustee

# Check if binary is in PATH
ls -l /usr/bin/clevis-pin-trustee
```

#### Missing Dependencies

If running fails with missing dependencies:

```bash
# Install runtime dependencies
sudo dnf install clevis jose

# For development/testing
sudo dnf install trustee-attester
```

## Updating the Package

When releasing a new version:

1. **Update version** in:
   - `cli/Cargo.toml`
   - `lib/Cargo.toml`
   - `clevis-pin-trustee.spec` (Version and %changelog)

2. **Regenerate vendored tarball**:
   ```bash
   ./copr-build.sh 0.2.0
   ```

3. **Upload to COPR**

4. **Tag the release** (for GitHub):
   ```bash
   git tag -a v0.2.0 -m "Release version 0.2.0"
   git push origin v0.2.0
   ```

## File Locations

### Source Files

- `clevis-pin-trustee.spec` - RPM spec file
- `copr-build.sh` - Automated build script
- `clevis-pin-trustee-VERSION-vendor.tar.gz` - Vendored source tarball (generated)

### Build Artifacts

- `~/rpmbuild/SOURCES/` - Source tarballs
- `~/rpmbuild/SPECS/` - RPM spec files
- `~/rpmbuild/SRPMS/` - Source RPM packages
- `~/rpmbuild/RPMS/` - Binary RPM packages

## Additional Resources

- **COPR Documentation**: https://docs.pagure.org/copr.copr/
- **COPR Project**: https://copr.fedorainfracloud.org/coprs/sarroutb/clevis-pin-trustee/
- **Rust Packaging Guidelines**: https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/
- **RPM Packaging Guide**: https://rpm-packaging-guide.github.io/

## License Compliance

The spec file includes proper licensing information:

- **Source License**: MIT
- **SPDX Headers**: All files include SPDX-FileCopyrightText and SPDX-License-Identifier

When packaging, ensure:
- `%license` tag points to correct license files
- `LICENSES/` directory is included in source tarball
