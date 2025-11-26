#!/bin/bash
# SPDX-FileCopyrightText: Sergio Arroutbi <sarroutb@redhat.com>
#
# SPDX-License-Identifier: MIT

set -euo pipefail

PACKAGE_NAME="clevis-pin-trustee"
COPR_PROJECT="sarroutb/clevis-pin-trustee"
VERSION="${1:-0.1.0}"

echo "==> Building COPR package for ${PACKAGE_NAME} v${VERSION}"

# Check if copr-cli is installed
if ! command -v copr-cli &> /dev/null; then
    echo "Error: copr-cli is not installed"
    echo "Install with: dnf install copr-cli"
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Must be run from the project root directory"
    exit 1
fi

# Step 1: Clean previous vendor directory
echo "==> Cleaning previous vendor directory"
rm -rf vendor .cargo/config.toml

# Step 2: Create vendor directory
echo "==> Creating vendor directory"
cargo vendor --versioned-dirs vendor > /dev/null

# Step 3: Create .cargo/config.toml
echo "==> Creating .cargo/config.toml"
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

# Step 4: Create vendored tarball
TARBALL="${PACKAGE_NAME}-${VERSION}-vendor.tar.gz"
echo "==> Creating vendored tarball: ${TARBALL}"

# Create tarball with git content + vendor directory
git archive --format=tar --prefix=${PACKAGE_NAME}-${VERSION}/ HEAD | gzip > /tmp/git-archive.tar.gz
mkdir -p /tmp/${PACKAGE_NAME}-${VERSION}
cd /tmp && tar xzf git-archive.tar.gz
cp -r ${OLDPWD}/vendor ${PACKAGE_NAME}-${VERSION}/
cp -r ${OLDPWD}/.cargo ${PACKAGE_NAME}-${VERSION}/
tar czf ${TARBALL} ${PACKAGE_NAME}-${VERSION}/
mv ${TARBALL} ${OLDPWD}/
cd ${OLDPWD}

echo "==> Tarball created: ${TARBALL} ($(du -h ${TARBALL} | cut -f1))"

# Step 5: Build SRPM
echo "==> Building SRPM"
if ! command -v rpmbuild &> /dev/null; then
    echo "Error: rpmbuild is not installed"
    echo "Install with: dnf install rpm-build rpmdevtools"
    exit 1
fi

# Create RPM build directories
mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Copy sources
cp ${TARBALL} ~/rpmbuild/SOURCES/
cp ${PACKAGE_NAME}.spec ~/rpmbuild/SPECS/

# Build SRPM
rpmbuild -bs ~/rpmbuild/SPECS/${PACKAGE_NAME}.spec

SRPM=$(ls -t ~/rpmbuild/SRPMS/${PACKAGE_NAME}-${VERSION}-*.src.rpm | head -1)
echo "==> SRPM created: ${SRPM}"

# Step 6: Upload to COPR
echo ""
echo "==> Ready to upload to COPR"
echo ""
echo "To upload to COPR, run:"
echo "  copr-cli build ${COPR_PROJECT} ${SRPM}"
echo ""
echo "Or to build for specific chroots:"
echo "  copr-cli build ${COPR_PROJECT} --chroot epel-9-x86_64 --chroot epel-10-x86_64 ${SRPM}"
echo ""
read -p "Upload to COPR now? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "==> Uploading to COPR"
    copr-cli build ${COPR_PROJECT} \
        --chroot epel-9-x86_64 \
        --chroot epel-9-aarch64 \
        --chroot epel-10-x86_64 \
        --chroot epel-10-aarch64 \
        ${SRPM}
    echo "==> Build submitted to COPR"
    echo "==> Check status at: https://copr.fedorainfracloud.org/coprs/${COPR_PROJECT}/builds/"
else
    echo "==> Skipping COPR upload"
    echo "==> SRPM is available at: ${SRPM}"
fi

echo ""
echo "==> Done!"
