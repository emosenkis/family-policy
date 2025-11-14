Name:           family-policy
Version:        0.1.0
Release:        1%{?dist}
Summary:        Browser Extension Policy Manager

License:        MIT
URL:            https://github.com/username/family-policy
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  systemd-rpm-macros
Requires:       systemd

%description
Manages browser extension force-install policies and privacy controls
for Chrome, Firefox, and Edge across Linux systems. Supports both
local mode and remote GitHub-based policy management.

Features:
 - Force-install browser extensions
 - Disable private/incognito browsing modes
 - Remote policy management via GitHub polling
 - Support for Chrome, Firefox, and Edge browsers

%prep
%setup -q

%build
# Binary is pre-built

%install
rm -rf $RPM_BUILD_ROOT

# Install binary
mkdir -p $RPM_BUILD_ROOT/usr/local/bin
install -m 755 family-policy $RPM_BUILD_ROOT/usr/local/bin/family-policy

# Install systemd service
mkdir -p $RPM_BUILD_ROOT/etc/systemd/system
install -m 644 family-policy-agent.service $RPM_BUILD_ROOT/etc/systemd/system/

# Create directories
mkdir -p $RPM_BUILD_ROOT/etc/family-policy
mkdir -p $RPM_BUILD_ROOT/var/lib/browser-extension-policy

%post
%systemd_post family-policy-agent.service

echo ""
echo "Family Policy Agent has been installed."
echo ""
echo "To configure and start the agent:"
echo "  1. sudo family-policy agent setup --url YOUR_GITHUB_POLICY_URL"
echo "  2. sudo family-policy agent install"
echo "  3. sudo family-policy agent start"
echo ""

%preun
%systemd_preun family-policy-agent.service

%postun
%systemd_postun_with_restart family-policy-agent.service

%files
%attr(755,root,root) /usr/local/bin/family-policy
%attr(644,root,root) /etc/systemd/system/family-policy-agent.service
%dir %attr(755,root,root) /etc/family-policy
%dir %attr(755,root,root) /var/lib/browser-extension-policy

%changelog
* Thu Jan 16 2025 Family Policy Team <noreply@example.com> - 0.1.0-1
- Initial release
- Support for Chrome, Firefox, and Edge
- Agent mode with GitHub polling
- Local mode for direct configuration
