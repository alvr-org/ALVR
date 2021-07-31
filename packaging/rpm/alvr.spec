Name: alvr
Version: 16.0.0-rc2
Release: 1.0.0
Summary: Stream VR games from your PC to your headset via Wi-Fi
License: MIT
Source: https://github.com/alvr-org/ALVR/archive/refs/tags/v%{version}.tar.gz
ExclusiveArch: x86_64
BuildRequires: alsa-lib-devel cairo-gobject-devel cargo clang-devel ffmpeg-devel gcc gcc-c++ ImageMagick libunwind-devel rust rust-atk-sys-devel rust-cairo-sys-rs-devel rust-gdk-sys-devel rust-glib-sys-devel rust-pango-sys-devel vulkan-headers vulkan-loader-devel
BuildRoot: %{_tmppath}/%{name}-%{version}-%{release}-root
Requires: ffmpeg steam
Requires(post): policycoreutils
Requires(postun): policycoreutils
%global debug_package %{nil} 

%description
ALVR is an open source remote VR display which allows playing SteamVR games on a standalone headset such as Gear VR or Oculus Go/Quest.

%pre
%define alvrBuildDir build/%{name}_server_linux

%prep
%autosetup -D -n %{_builddir}

%build
# Build ALVR
cargo xtask build-server --release
# Build SELinux policy
rm -f packaging/selinux/alvr.pp.bz2
make -f /usr/share/selinux/devel/Makefile -C 'packaging/selinux'
bzip2 "packaging/selinux/%{name}.pp"

%changelog
* Sun Jul 18 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-1.0.0
    - Updated descriptions
    - Updated license
    - Added trailing newlines
    - Removed path from executable in freedesktop config
    - Corrected license
    - Updated specfile to be a bit clearer
    - Added conditional logic for port labeling
* Sun Jul 18 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.b1
    - Added freedesktop desktop file for Gnome / KDE
    - Updated post script to reload firewalld
    - Added ImageMagick png generation for icons
* Sat Jul 17 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a6
    - Added SELinux port restrictions
* Sat Jul 17 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a5
    - Fixed restorecon
* Sat Jul 17 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a4
    - Fixed firewalld and SELinux policy
* Sat Jul 17 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a3
    - Added firewalld policy
    - Added SELinux policy and compilation
* Tue Jul 13 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a2
    - Removed dependencies on snapd in favor of system rust and cargo
* Tue Jul 13 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a1
    - Initial specfile

%install
# Create dirs 
mkdir -p %{buildroot}{%{_bindir},%{_datadir}/{applications,licenses/%{name},selinux/packages},%{_libdir},%{_usr}/lib/firewalld/services,%{_libexecdir},%{_docdir}}
# Copy build files
cp "%{alvrBuildDir}/bin/%{name}_launcher" "%{buildroot}%{_bindir}"
chmod +x "%{buildroot}%{_bindir}/%{name}_launcher"
cp -ar "%{alvrBuildDir}/lib64/"* "%{buildroot}%{_libdir}/"
cp -ar "%{alvrBuildDir}/libexec/%{name}" "%{buildroot}%{_libexecdir}/"
cp -ar "%{alvrBuildDir}/share/"* "%{buildroot}%{_datadir}/"
# Copy source files
cp -ar "LICENSE" "%{buildroot}%{_datadir}/licenses/%{name}/"
cp "packaging/selinux/%{name}.pp.bz2" "%{buildroot}%{_datadir}/selinux/packages/"
cp "packaging/freedesktop/%{name}.desktop" "%{buildroot}%{_datadir}/applications/"
cp "packaging/firewalld/alvr.xml" "%{buildroot}/%{_usr}/lib/firewalld/services/"
# Generate png icons
for res in 16x16 32x32 48x48 64x64 128x128 256x256; do
    mkdir -p "%{buildroot}%{_datadir}/icons/hicolor/${res}/apps"
    convert 'alvr/launcher/res/launcher.ico' -thumbnail "${res}" -alpha on -background none -flatten "%{buildroot}%{_datadir}/icons/hicolor/${res}/apps/alvr.png"
done

%files 
%{_bindir}/%{name}_launcher
%{_datadir}/%{name}/
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/16x16/apps/alvr.png
%{_datadir}/icons/hicolor/32x32/apps/alvr.png
%{_datadir}/icons/hicolor/48x48/apps/alvr.png
%{_datadir}/icons/hicolor/64x64/apps/alvr.png
%{_datadir}/icons/hicolor/128x128/apps/alvr.png
%{_datadir}/icons/hicolor/256x256/apps/alvr.png
%{_datadir}/licenses/%{name}
%{_datadir}/selinux/packages/%{name}.pp.bz2
%{_datadir}/vulkan/explicit_layer.d/%{name}_x86_64.json
%{_libdir}/%{name}/
%{_libdir}/lib%{name}_vulkan_layer.so
%{_libexecdir}/%{name}/
%{_usr}/lib/firewalld/services/%{name}.xml

%clean
rm -rf "%{buildroot}"

%postun
if [ "${1}" = 0 ]; then
    # Unlabel ports
    semanage port -d -p udp 9943-9944
    # Unload SELinux policy
    semodule -nr %{name} >/dev/null
fi

%post
# Check if firewalld is running and reload
if firewall-cmd --get-active-zones >/dev/null 2>&1; then 
    firewall-cmd --reload >/dev/null
fi

# Check if SELinux is enabled and load policy
if selinuxenabled; then
    # Load SELinux policy
    semodule -ni %{_datadir}/selinux/packages/%{name}.pp.bz2
    load_policy
    # Restore contexts
    restorecon -FR "%{_bindir}/%{name}_launcher" %{_libdir}/{%{name},lib%{name}_vulkan_layer.so} "%{_libexecdir}/%{name}"
    # Label ports if they're unlabeled
    if ! semanage port -l | grep -P "%{name}_port_t\s+udp\s+9943-9944" >/dev/null 2>&1; then
        semanage port -a -t "%{name}_port_t" -p udp 9943-9944
    fi
fi
