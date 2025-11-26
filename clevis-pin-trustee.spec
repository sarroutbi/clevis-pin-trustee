# SPDX-FileCopyrightText: Sergio Arroutbi <sarroutb@redhat.com>
#
# SPDX-License-Identifier: MIT

# Disable debuginfo generation for Rust binaries
%global debug_package %{nil}

%if 0%{?rhel} || 0%{?epel}
# RHEL/EPEL: Use bundled deps as it doesn't ship Rust libraries
%global bundled_rust_deps 1
%else
# Fedora: Could use system Rust libraries, but we use vendored for simplicity
%global bundled_rust_deps 1
%endif

Name:           clevis-pin-trustee
Version:        0.1.0
Release:        1%{?dist}
Summary:        Clevis PIN for Trustee attestation

License:        MIT
URL:            https://github.com/sarroutbi/clevis-pin-trustee
Source0:        %{name}-%{version}-vendor.tar.gz

%if 0%{?bundled_rust_deps}
BuildRequires:  rust-toolset
%else
BuildRequires:  rust-packaging >= 25
%endif
BuildRequires:  openssl-devel

# Runtime dependencies
Requires:       clevis
Requires:       jose

%description
clevis-pin-trustee is a Clevis PIN that implements encryption and decryption
operations using remote attestation via a Trustee server. It enables automated
unlocking of LUKS-encrypted volumes in confidential computing environments by
fetching encryption keys from Trustee servers after successful attestation.

%prep
%autosetup -n %{name}-%{version}
%if 0%{?bundled_rust_deps}
# Configure cargo to use vendored dependencies
%cargo_prep -v vendor
%else
%cargo_prep
%endif

%build
# Build using cargo macros
%cargo_build
%if 0%{?bundled_rust_deps}
# Generate vendor manifest (required for bundled crates tracking)
%cargo_vendor_manifest
%endif

%install
# Install main binary
install -D -m 0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

# Install Clevis wrapper scripts
install -D -m 0755 clevis-encrypt-trustee %{buildroot}%{_bindir}/clevis-encrypt-trustee
install -D -m 0755 clevis-decrypt-trustee %{buildroot}%{_bindir}/clevis-decrypt-trustee

%check
# Run tests using cargo macro
%cargo_test

%files
%license LICENSES/MIT.txt
%if 0%{?bundled_rust_deps}
%license cargo-vendor.txt
%endif
%doc README.md
%{_bindir}/%{name}
%{_bindir}/clevis-encrypt-trustee
%{_bindir}/clevis-decrypt-trustee

%changelog
* Wed Nov 26 2025 Sergio Arroutbi <sarroutb@redhat.com> - 0.1.0-1
- Initial release
- Clevis PIN for Trustee attestation
- Support for multiple Trustee server URLs with failover
- Certificate-based TLS authentication
- Optional initdata for attestation context
