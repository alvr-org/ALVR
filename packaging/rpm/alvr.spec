Name: alvr
Version: 17.0.0
Release: 0.0.1dev.10
Summary: Stream VR games from your PC to your headset via Wi-Fi
License: MIT
Source: https://github.com/alvr-org/ALVR/archive/refs/tags/v17.0.0-dev.10.tar.gz
URL: https://github.com/alvr-org/ALVR/
ExclusiveArch: x86_64
BuildRequires: alsa-lib-devel cairo-gobject-devel cargo clang-devel ffmpeg-devel gcc gcc-c++ cmake ImageMagick libunwind-devel openssl-devel rpmdevtools rust rust-atk-sys-devel rust-cairo-sys-rs-devel rust-gdk-sys-devel rust-glib-sys-devel rust-pango-sys-devel selinux-policy-devel vulkan-headers vulkan-loader-devel
BuildRoot: %{_tmppath}/%{name}-%{version}-%{release}-root
Requires: ffmpeg steam
Requires(post): policycoreutils
Requires(postun): policycoreutils
%define alvrBuildDir build/%{name}_server_linux
# find-debuginfo.sh doesn't appear to be working
%global debug_package %{nil}

%description
ALVR is an open source remote VR display which allows playing SteamVR games on
a standalone headset such as Gear VR or Oculus Go/Quest.

%prep
%autosetup -D -n %{_builddir}

%build
# Set CXXFLAGS for ffmpeg
export CXXFLAGS+=' -I/usr/include/ffmpeg'
# Build ALVR
cargo xtask build-server --release
# Build SELinux policy
rm -f 'packaging/selinux/%{name}.pp.bz2'
make -f '/usr/share/selinux/devel/Makefile' -C 'packaging/selinux'
bzip2 'packaging/selinux/%{name}.pp'

%changelog
* Fri Jul 30 2021 Trae Santiago <trae32566@gmail.com> - 16.0.0-0.0.1rc1
    - Initial release; see GitHub For changelog

%install
# Create dirs
newDirs=(
    '%{_bindir}'
    '%{_datadir}/'{'applications','licenses/%{name}','selinux/packages'}
    '%{_libdir}'
    '%{_usr}/lib/firewalld/services'
    '%{_libexecdir}'
)
for newDir in "${newDirs[@]}"; do
    mkdir -p "%{buildroot}${newDir}"
done
# Strip binaries
newBins=(
    'bin/%{name}_launcher'
    'lib64/%{name}/bin/linux64/driver_%{name}_server.so'
    'lib64/lib%{name}_vulkan_layer.so'
    'libexec/%{name}/vrcompositor-wrapper'
)
for newBin in "${newBins[@]}"; do
    strip "%{alvrBuildDir}/${newBin}"
done
# Copy build files
cp '%{alvrBuildDir}/bin/%{name}_launcher' '%{buildroot}%{_bindir}'
cp -ar '%{alvrBuildDir}/lib64/'* '%{buildroot}%{_libdir}/'
cp -ar '%{alvrBuildDir}/libexec/%{name}' '%{buildroot}%{_libexecdir}/'
cp -ar '%{alvrBuildDir}/share/'* '%{buildroot}%{_datadir}/'
cp 'LICENSE' '%{buildroot}%{_datadir}/licenses/%{name}/'
# Copy source files
cp 'packaging/selinux/%{name}.pp.bz2' '%{buildroot}%{_datadir}/selinux/packages/'
cp 'packaging/freedesktop/%{name}.desktop' '%{buildroot}%{_datadir}/applications/'
cp 'packaging/firewall/%{name}-firewalld.xml' '%{buildroot}/%{_usr}/lib/firewalld/services/%{name}.xml'
cp 'packaging/firewall/%{name}_fw_config.sh' '%{buildroot}%{_libexecdir}/%{name}/'
cp 'packaging/firewall/ufw-%{name}' '%{buildroot}%{_datadir}/%{name}'
# Generate png icons
for res in 16x16 32x32 48x48 64x64 128x128 256x256; do
    mkdir -p "%{buildroot}%{_datadir}/icons/hicolor/${res}/apps"
    convert '%{name}/launcher/res/launcher.ico' -thumbnail "${res}" -alpha on -background none -flatten "%{buildroot}%{_datadir}/icons/hicolor/${res}/apps/%{name}.png"
done

%files
%{_bindir}/%{name}_launcher
%{_datadir}/%{name}/
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/16x16/apps/%{name}.png
%{_datadir}/icons/hicolor/32x32/apps/%{name}.png
%{_datadir}/icons/hicolor/48x48/apps/%{name}.png
%{_datadir}/icons/hicolor/64x64/apps/%{name}.png
%{_datadir}/icons/hicolor/128x128/apps/%{name}.png
%{_datadir}/icons/hicolor/256x256/apps/%{name}.png
%doc %{_datadir}/licenses/%{name}
%{_datadir}/selinux/packages/%{name}.pp.bz2
%{_datadir}/vulkan/explicit_layer.d/%{name}_x86_64.json
%{_libdir}/%{name}/
%{_libdir}/lib%{name}_vulkan_layer.so
%{_libexecdir}/%{name}/
%{_usr}/lib/firewalld/services/%{name}.xml

%clean
rm -rf '%{buildroot}'

%postun
if [ "${1}" = 0 ]; then
    # Unlabel ports
    semanage port -d -p tcp 9943-9944
    semanage port -d -p udp 9943-9944
    # Unload SELinux policy
    semodule -nr '%{name}' >/dev/null
fi

%post
# Check if firewalld is running and reload
if firewall-cmd --get-active-zones >/dev/null 2>&1; then
    firewall-cmd --reload >/dev/null
fi
# Check if SELinux is enabled and load policy
if selinuxenabled; then
    # Load SELinux policy
    semodule -ni '%{_datadir}/selinux/packages/%{name}.pp.bz2'
    load_policy
    # Restore contexts
    restorecon -FR '%{_bindir}/%{name}_launcher' '%{_libdir}/'{'%{name}','lib%{name}_vulkan_layer.so'} '%{_libexecdir}/%{name}'
    # Label ports if they're unlabeled
    if ! semanage port -l | grep -P '%{name}_port_t.*9943-9944' >/dev/null 2>&1; then
        semanage port -a -t '%{name}_port_t' -p tcp 9943-9944
        semanage port -a -t '%{name}_port_t' -p udp 9943-9944
    fi
fi
