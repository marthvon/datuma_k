%global debug_package %{nil}

Name:           datuma-k
Version:        @VERSION@
Release:        1%{?dist}
Summary:        Data contract plus templates that generate source
License:        AGPL-3.0-only
URL:            @REPO_HOMEPAGE@
Source0:        @BASE_URL@/datuma_k-linux-x86_64
Source1:        @BASE_URL@/datuma_k-linux-aarch64
ExclusiveArch:  x86_64 aarch64

%description
A data contract (*.dtct) plus templates (*.ngin) that generate source.
Declare the shape once; each platform gets its own types, validation, and UI.

%prep

%build

%install
%ifarch x86_64
install -D -m 0755 %{SOURCE0} %{buildroot}%{_bindir}/datuma_k
%else
install -D -m 0755 %{SOURCE1} %{buildroot}%{_bindir}/datuma_k
%endif

%files
%{_bindir}/datuma_k

%changelog
* @CHANGELOG_DATE@ mamertvonn <https://github.com/marthvon> - @VERSION@-1
- Release @VERSION@
