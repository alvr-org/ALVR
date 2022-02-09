#!/usr/bin/env bash
# Functions to prepare and build packages for Debian-based distributions
# Disable warnings for importing snapd and variable referenced but not assigned
# shellcheck disable=SC1091,SC2154
prep_debian_client() {
    sudo -s <<SUDOCMDS
apt -y install default-jre python snapd
snap install androidsdk
SUDOCMDS
    # shellcheck disable=SC2181
    if [ $? -eq 0 ]; then
        prep_rustup
    else
        return 1
    fi
}

prep_debian_server() {
    log info 'Copying control file ...'
    cp "${repoDir}/${controlFile}" "${tmpDir}/control"

    if [ "${kwArgs['--no-nvidia']}" != '' ]; then
        log info 'Removing unused nvidia build dependency ...'
        sed -i 's/nvidia-cuda-toolkit,//' "${tmpDir}/control"
    fi

    basePackages=(
        'devscripts'
        'equivs'
        'git'
        'snapd'
    )
    # ONLY these need sudo
    log info 'Installing packages ...'
    sudo -s <<SUDOCMDS
apt -y install ${basePackages[@]}
yes | mk-build-deps -ir "${tmpDir}/control"
SUDOCMDS
    # shellcheck disable=SC2181
    if [ $? -eq 0 ]; then
        prep_rustup
    else
        return 1
    fi
}

build_debian_client() { build_generic_client "${@}"; }

# This needs srs error checking
build_debian_server() {
    # Create debian-specific version
    debVer="$(grep '^Version' "${tmpDir}/control" | awk '{ print $2 }')"
    [ "${buildVer}" != '' ] && debVer+="${buildVer}"

    debTmpDir="${tmpDir}/alvr_${debVer}"
    newBins=(
        'bin/alvr_launcher'
        'lib64/alvr/bin/linux64/driver_alvr_server.so'
        'lib64/libalvr_vulkan_layer.so'
        'libexec/alvr/vrcompositor-wrapper'
    )
    newDirs=(
        'DEBIAN'
        'etc/ufw/applications.d'
        'usr/bin'
        'usr/share/'{'applications','licenses/alvr','selinux/packages'}
        'usr/lib64'
        'usr/lib/firewalld/services'
        'usr/libexec/alvr/'
    )

    # Add package config (required for Ubuntu)
    export PKG_CONFIG_PATH="${PKG_CONFIG_PATH}:${repoDir}/packaging/deb/cuda.pc"

    cd "${repoDir}" > /dev/null || return 4
    # There's no vulkan-enabled ffmpeg in the ubuntu repos afaik
    log info 'Building ALVR server ...'
    if cargo xtask build-server ${kwArgs['--server-args']:---release --bundle-ffmpeg}; then
        cd - > /dev/null || return 4
    else
        cd - > /dev/null && return 4
    fi

    log info 'Creating directories ...'
    for newDir in "${newDirs[@]}"; do
        mkdir -p "${debTmpDir}/${newDir}"
    done

    log info 'Stripping binaries ...'
    for newBin in "${newBins[@]}"; do
        strip "${buildDir}/${newBin}"
    done

    log info 'Copying files and mangling control file version...'
    # Copy build files
    cp "${buildDir}bin/alvr_launcher" "${debTmpDir}/usr/bin/"
    cp -ar "${buildDir}lib64/"*"alvr"* "${debTmpDir}/usr/lib64/"
    cp -ar "${buildDir}libexec/alvr/" "${debTmpDir}/usr/libexec/"
    cp -ar "${buildDir}share/"* "${debTmpDir}/usr/share/"
    cp "${repoDir}/LICENSE" "${debTmpDir}/usr/share/licenses/alvr/"
    # Copy source files
    cp "${repoDir}/packaging/deb/"* "${debTmpDir}/DEBIAN/"
    # Mangle version to version+<short-hash> AFTER it's copied
    sed -i "s/^Ver.*/Version: ${debVer}/" "${debTmpDir}/DEBIAN/control"
    cp "${repoDir}/packaging/freedesktop/alvr.desktop" "${debTmpDir}/usr/share/applications/"
    cp "${repoDir}/packaging/firewall/alvr-firewalld.xml" "${debTmpDir}/usr/share/alvr/"
    cp "${repoDir}/packaging/firewall/alvr_fw_config.sh" "${debTmpDir}/usr/libexec/alvr/"
    cp "${repoDir}/packaging/firewall/ufw-alvr" "${debTmpDir}/etc/ufw/applications.d/"

    log info 'Generating icons ...'
    for res in 16x16 32x32 48x48 64x64 128x128 256x256; do
        mkdir -p "${debTmpDir}/usr/share/icons/hicolor/${res}/apps"
        convert "${repoDir}/alvr/launcher/res/launcher.ico" -thumbnail "${res}" -alpha on -background none -flatten "${debTmpDir}/usr/share/icons/hicolor/${res}/apps/alvr.png"
    done

    log info 'Generating package ...'
    if dpkg-deb --build --root-owner-group "${debTmpDir}"; then
        # dpkg-deb puts the resulting file in the top level directory
        mv "${tmpDir}/alvr_${debVer}.deb" "${repoDir}/build"
    else
        log critical 'Unable to create package!' 8
    fi
}

# Pop!_OS
prep_pop_client() { prep_debian_client "${@}"; }
prep_pop_server() { prep_debian_server "${@}"; }
build_pop_client() { build_generic_client "${@}"; }
build_pop_server() { build_debian_server "${@}"; }

# Ubuntu
prep_ubuntu_client() { prep_debian_client "${@}"; }
prep_ubuntu_server() { prep_debian_server "${@}"; }
build_ubuntu_client() { build_generic_client "${@}"; }
build_ubuntu_server() { build_debian_server "${@}"; }
