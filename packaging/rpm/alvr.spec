Name: alvr
Version: 15.2.1
Release: 0.0.a3
Summary: Stream VR games from your PC to your headset via Wi-Fi
License: MIT
Source: v%{version}.tar.gz
ExclusiveArch: x86_64
BuildRequires: alsa-lib-devel cairo-gobject-devel cargo clang-devel ffmpeg-devel gcc gcc-c++ libunwind-devel rust rust-atk-sys-devel rust-cairo-sys-rs-devel rust-gdk-sys-devel rust-glib-sys-devel rust-pango-sys-devel vulkan-headers vulkan-loader-devel
BuildRoot: %{_tmppath}/%{name}-%{version}-%{release}-root
Requires: ffmpeg rpmfusion-free-release rpmfusion-nonfree-release steam
Requires(post): policycoreutils
Requires(postun): policycoreutils
# Thank you for the useless documentation on the turd nugget that is debuginfo...
%global debug_package %{nil} 

%description
ALVR uses technologies like Asynchronous Timewarp and Fixed Foveated Rendering for a smoother experience. All games that work with an Oculus Rift (s) should work with ALVR.

%pre
%define alvrSrcDir %{_builddir}/ALVR-%{version}
%define alvrBuildDir %{alvrSrcDir}/build/%{name}_server_linux

%prep
%autosetup -D -n %{_builddir}

%build
cd %{_builddir}/ALVR-%{version}
cargo xtask build-server --release
make -f /usr/share/selinux/devel/Makefile -C "%{alvrSrcDir}/packaging/selinux"
#cp "%{ssSrcDir}/policies/%{name}.pp" "%{buildroot}%{_datadir}/selinux/packages/" 
#Package and move 
#bzip2 %{buildroot}%{_datadir}/selinux/packages/%{name}{,-rsyncd}.pp

%changelog
* Sat Jul 17 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a3
    - Added firewalld policy
    - Added SELinux policy and compilation
* Tue Jul 13 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a2
    - Removed dependencies on snapd in favor of system rust and cargo
* Tue Jul 13 2021 Trae Santiago <trae32566@gmail.com> - 15.2.1-0.0.a1
    - Initial specfile

%install
# Create dirs 
mkdir -p %{buildroot}{%{_bindir},%{_datadir}/{licenses/%{name},selinux/packages},%{_libdir},%{_libexecdir},%{_docdir}}

# Copy build files
cp "%{alvrBuildDir}/bin/%{name}_launcher" "%{buildroot}%{_bindir}"
chmod +x "%{buildroot}%{_bindir}/%{name}_launcher"
cp -ar "%{alvrBuildDir}/lib64/"* "%{buildroot}%{_libdir}/"
cp -ar "%{alvrBuildDir}/libexec/%{name}" "%{buildroot}%{_libexecdir}/"
cp -ar "%{alvrBuildDir}/share/"* "%{buildroot}%{_datadir}/"

# Copy source files
cp -ar "%{alvrSrcDir}/LICENSE" "%{buildroot}%{_datadir}/licenses/%{name}"
cp "%{alvrSrcDir}/packaging/firewalld/alvr.xml" "%{buildroot}%{_libdir}/firewalld/services/"

%files 
%{_bindir}/%{name}_launcher
%{_datadir}/%{name}
%{_datadir}/licenses/%{name}
%{_datadir}/selinux/packages/%{name}.pp.bz2
%{_datadir}/vulkan/explicit_layer.d/%{name}_x86_64.json
%{_libdir}/firewalld/services/%{name}.xml
%{_libdir}/%{name}/
%{_libdir}/lib%{name}_vulkan_layer.so
%{_libexecdir}/%{name}/

%clean
rm -rf %{buildroot}

%postun
if [ "${1}" = 0 ]; then
    # Unload SELinux policy
    semodule -nr %{name}
fi


%post
# Check if SELinux is enabled and load policy
selinuxenabled && if [ $? -eq 0 ]; then
    # Load SELinux policy
    semodule -ni %{_datadir}/selinux/packages/%{name}.pp.bz2
    load_policy
    # Restore contexts
    restorecon -FR %{_bindir}/%{name}_launcher %{_sysconfdir}/%{name} %{_libdir}/%{name}
fi