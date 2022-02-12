#!/usr/bin/env bash
# Functions to prepare and build packages for Fedora
# Disable warnings for importing snapd and variable referenced but not assigned
# shellcheck disable=SC1091,SC2154
prep_fedora_client() {
    log error 'Fedora client builds are not recommended, as they currently install and utilize non-rpm Rust packages'
    sudo -s <<SUDOCMDS
dnf -y install java snapd
systemctl enable --now snapd

snap install androidsdk
SUDOCMDS
    # This is a very basic check; ideally this and others should be checked individually in the heredoc above
    # shellcheck disable=SC2181
    if [ $? -eq 0 ]; then
        # Load any additional snapd binary locations
        . /etc/profile.d/snapd.sh
        prep_rustup
    else
        return 1
    fi
}

prep_fedora_server() {
    log info 'Copying spec file ...'
    cp "${repoDir}/${specFile}" "${tmpDir}/tmp.spec"

# Nvidia + CUDA build deps need to be added to the spec, then stripped here if --no-nvidia is use
#    if [ "${kwArgs['--no-nvidia']}" != '' ]; then
#        log info 'Removing unused nvidia build dependency ...'
#        sed -i 's/nvidia-cuda-toolkit,//' "${tmpDir}/tmp.spec"
#    fi

    basePackages=(
        'dnf-utils'
        'git'
        "https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-${VERSION_ID}.noarch.rpm"
        "https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-${VERSION_ID}.noarch.rpm"
    )
    # ONLY these need sudo
    sudo -s <<SUDOCMDS
dnf -y install ${basePackages[@]}
yum-builddep -y "${tmpDir}/tmp.spec"
SUDOCMDS
}

build_fedora_client() { build_generic_client "${@}"; }

build_fedora_server() {
    # Don't care if this fails
    mkdir -p "${HOME}/rpmbuild/SOURCES" > /dev/null 2>&1
    log info 'Building tarball ...'
    # The relative path at the end here is a rlly bad idea, but where does it live?!
    if tar -czf "${HOME}/rpmbuild/SOURCES/$(spectool "${tmpDir}/tmp.spec" | grep -oP 'v\d+\.\d+\..*\.tar\.gz')" -C "${repoDir}" .; then
        log info 'Mangling spec file version and building RPMS ...'
        [ "${buildVer}" != '' ] && sed -i "s/Release:.*/\0${buildVer}/" "${tmpDir}/tmp.spec"

        # Replace build arguments in specfile if needed
        if [ "${kwArgs['--server-args']}" != '' ]; then
            sed -i "s/cargo xtask build-server --release/cargo xtask build-server ${kwArgs['--server-args']}/" "${tmpDir}/tmp.spec"
        fi

        rpmbuild -ba "${tmpDir}/tmp.spec"
    else
        log critical 'Failed to build tarball!' 5
    fi
}

