#!/usr/bin/env bash
# Script to prepare and build packages
# Exit codes:
# 1 - ALVR client preparation failed
# 2 - ALVR client build failed
# 3 - ALVR server preparation failed
# 4 - ALVR server build failed
# 5 - ALVR server tarball creation failed
# 6 - Unable to download Deb control file
# 7 - Unable to install / upgrade rustup
# 8 - Unable to create deb
# 99 - Script run as root
#
# Disable warnings about importing snapd
# shellcheck disable=SC1091

# GitHub repo
repo='alvr-org/ALVR'
# Git branch
branch='master'
# RPM spec file
specFile='packaging/rpm/alvr.spec'
# deb control file
controlFile='packaging/deb/control'
# Raw file provider
rawContentProvider='https://raw.githubusercontent.com'
# Android NDK version
ndkVersion=30

# Grab the repository directory
repoDir="$(realpath $(dirname "${0}"))/.."
if ! [ -d "${repoDir}/.git" ]; then
    # Get the absolute directory the script is running in, and add the repo name
    repoDir="$(dirname "$(realpath "${0}")")/$(basename "${repo}")"
fi

# Set a temporary working directory
tmpDir="/tmp/alvr_$(date '+%Y%m%d-%H%M%S')"
buildDir="${repoDir}/build/alvr_server_linux/"

# Import OS info - provides ${ID}
. /etc/os-release

# Make sure we're not building as root
if [ "${USER}" == 'root' ]; then
    exit 99
fi

# Basic logger
# Logs various types of output with details
# Arguments: errorMessage [exitCode| NOKILL
log() {
    prefix=$(date +'%F %H:%M:%S');
    case "${1,,}" in
        'debug') printf "\E[35m%s - Debug: \n%s\e[0m\n" "${prefix}" "${2}" ;;
        'info') printf "\E[36m%s - Info: %s\e[0m\n" "${prefix}" "${2}" ;;
        'warning') printf "\E[33m%s - Warning: %s\e[0m\n" "${prefix}" "${2}" ;;
        'error')
            printf "\E[31m%s - Error: %s\e[0m" "${prefix}" "${2}"
            if [ "${3^^}" != 'NOKILL' ]; then
                printf "\nWould you like to continue (Y/[N])? "
                read -r keepGoing
                if [ "${keepGoing^^}" != 'Y' ]; then
                    log info "Exiting on user cancel"
                    exit "${3}"
                fi
            else
                echo
            fi
        ;;
        'critical')
            printf "\E[41m%s - Critical Error: %s\e[0m\n" "${prefix}" "${2}"
            exit "${3}"
        ;;
    esac
}

help_docs() {
    cat <<HELPME
Usage: $(basename "${0}") ACTION
Description: Script to prepare the system and build ALVR package(s)
Arguments:
    ACTIONS
        all             Prepare and build ALVR client and server
        client          Prepare and build ALVR client
        server          Prepare and build ALVR server
    FLAGS
        --build-only    Only build ALVR package(s)
        --bump-versions Bump versions before building
        --no-nvidia     Build without NVIDIA CUDA support
        --prep-only     Only prepare system for ALVR package build
HELPME
}

maybe_clone() {
    if ! [ -e "${repoDir}" ]; then
        log info "Cloning ${repo} into ${repoDir} ..."
        git clone -b "${branch}" "https://github.com/${repo}.git"
    fi

    # Get the short hash for this commit
    shortHash=$(git -C "${repoDir}" rev-parse --short HEAD)

    # Check if a tag exists; if not we're a nightly
    if [ "$(git -C "${repoDir}" tag --points-at HEAD)" == '' ]; then
        nightly=true
    fi
    nightlyVer="+$(date +%s)+${shortHash}"
}

###########
# Generic #
###########
prep_rustup() {
    # Manually add the default path
    export PATH="${PATH}:/snap/bin:/var/lib/snapd/snap"

    # Install rustup if it does not exist
    if ! command -v rustup > /dev/null 2>&1 && ! sudo snap install rustup --classic; then
        return 7
    fi
    log info 'Installing rust nightly ...'
    if ! rustup install nightly; then
        return 7
    fi
    # This doesn't necessarily need to succeed, but ideally it will
    rustup default nightly
}

build_generic_client() {
    # Make sure we agreed to licenses
    log info 'Accepting licenses ...'
    yes | androidsdk --licenses > /dev/null 2>&1

    # Grab the SDK root
    log info 'Installing Android NDK bundle ...'
    export "$(androidsdk ndk-bundle 2>&1 | grep 'SDK_ROOT=')"
    export ANDROID_SDK_ROOT="${SDK_ROOT}"
    log info "Using Android SDK: ${ANDROID_SDK_ROOT}"

    # Add LLVM / Clang Android path
    toolchainRoot="${SDK_ROOT}/ndk-bundle/toolchains/llvm/prebuilt/linux-x86_64/bin/"
    export PATH="${PATH}:${toolchainRoot}"

    log info "Linking Android ${ndkVersion} NDK toolchain to generic..."
    if ! [ -L "${toolchainRoot}/aarch64-linux-android-clang" ]; then
        ln -s "${toolchainRoot}/"{"aarch64-linux-android${ndkVersion}-clang",'aarch64-linux-android-clang'}
    fi
    if ! [ -L "${toolchainRoot}/aarch64-linux-android-clang++" ]; then
        ln -s "${toolchainRoot}/"{"aarch64-linux-android${ndkVersion}-clang++",'aarch64-linux-android-clang++'}
    fi


    log info 'Starting client build ...'
    # no subshell expansion warnings
    cd "${repoDir}" > /dev/null || return 2
    if cargo xtask build-android-deps && cargo xtask build-client --release; then
        # This needs stable support, only nightlies get built right now
        cp "${repoDir}/build/alvr_client_oculus_go/"* "../alvr_client_oculus_go${nightlyVer}.apk"
        cp "${repoDir}/build/alvr_client_oculus_quest/"* "../alvr_client_oculus_quest${nightlyVer}.apk"
        cd - > /dev/null || return 2
    else
        cd - > /dev/null && return 2
    fi
}

##########
# Fedora #
##########
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
    basePackages=(
        'dnf-utils'
        'git'
        "https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-${VERSION_ID}.noarch.rpm"
        "https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-${VERSION_ID}.noarch.rpm"
    )
    # ONLY these need sudo
    sudo -s <<SUDOCMDS
dnf -y install ${basePackages[@]}
yum-builddep -y ${rawContentProvider}/${repo}/${branch}/${specFile}
SUDOCMDS
}

build_fedora_server() {
    # Don't care if this fails
    mkdir -p "${HOME}/rpmbuild/SOURCES" > /dev/null 2>&1
    log info 'Building tarball ...'
    if tar -czf "${HOME}/rpmbuild/SOURCES/$(spectool "${repoDir}/${specFile}" | grep -oP 'v\d+\.\d+\..*\.tar\.gz')" -C "${repoDir}" .; then
        log info 'Mangling spec file version and building RPMS ...'
        if $nightly; then
            sed "s/Release:.*/\0+$(date +%s)+${shortHash}/" "${repoDir}/${specFile}" > "${tmpDir}/tmp.spec"
        else
            cp "${repoDir}/${specFile}" "${tmpDir}/tmp.spec"
        fi

        rpmbuild -ba "${tmpDir}/tmp.spec"
    else
        log critical 'Failed to build tarball!' 5
    fi
}

#############################
# Debian / Pop!_OS / Ubuntu #
#############################
prep_ubuntu_client() {
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

prep_ubuntu_server() {
    log info "Downloading control file and installing packages ..."
    if ! curl "${rawContentProvider}/${repo}/${branch}/${controlFile}" > "${tmpDir}/control"; then
        log critical "Unable to download control file from ${rawContentProvider}/${repo}/${branch}/${controlFile}" 6
    fi
    sudo -s <<SUDOCMDS
apt -y install devscripts equivs snapd
yes | mk-build-deps -ir "${tmpDir}/control"
SUDOCMDS
    # shellcheck disable=SC2181
    if [ $? -eq 0 ]; then
        prep_rustup
    else
        return 1
    fi
}

# This needs srs error checking
build_ubuntu_server() {
    # Create version
    debVer="$(grep '^Version' "${repoDir}/${controlFile}" | awk '{ print $2 }')"
    if $nightly; then
        debVer+="${nightlyVer}"
    fi

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

    cd "${repoDir}" > /dev/null || return 4
    # There's no vulkan-enabled ffmpeg afaik
    log info 'Building ALVR server ...'
    if cargo xtask build-server --release --bundle-ffmpeg; then
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
    cp "${buildDir}/bin/alvr_launcher" "${debTmpDir}/usr/bin/"
    cp -ar "${buildDir}/lib64/"*"alvr"* "${debTmpDir}/usr/lib64/"
    cp -ar "${buildDir}/libexec/alvr/" "${debTmpDir}/usr/libexec/"
    cp -ar "${buildDir}/share/"* "${debTmpDir}/usr/share/"
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
        cp "${tmpDir}/alvr_${debVer}.deb" "${HOME}"
    else
        log critical 'Unable to create package!' 8
    fi
}

# Debian
prep_debian_client() { prep_ubuntu_client "${@}"; }
prep_debian_server() { prep_ubuntu_server "${@}"; }
build_debian_server() { build_ubuntu_server "${@}"; }
# Pop!_OS
prep_pop_client() { prep_ubuntu_client "${@}"; }
prep_pop_server() { prep_ubuntu_server "${@}"; }
build_pop_server() { build_ubuntu_server "${@}"; }

main() {
    mkdir "${tmpDir}"

    case "${1,,}" in
        'client')
            log info "Preparing ${PRETTY_NAME} (${ID}) to build ALVR client..."
            # If we're only building, clone, build, and check the exit codes
            if [ "${2,,}" == '--build-only' ] && maybe_clone && build_generic_client; then
                log info 'ALVR client built successfully.'
            # If we got here that means we failed something
            elif [ "${2,,}" == '--build-only' ]; then
                log critical 'Failed to build ALVR client!' 2
            # Prepare and check return code
            elif prep_"${ID}"_client; then
                # Exit successfully if we're only preparing
                if [ "${2,,}" == '--prep-only' ]; then
                    exit 0
                # Clone, build, and check the exit codes
                elif maybe_clone && build_generic_client; then
                    log info 'ALVR client built successfully.'
                else
                    log critical 'Failed to build ALVR client!' 2
                fi
            else
                log critical "Failed to prepare ${PRETTY_NAME} (${ID}) for ALVR client build!" 1
            fi
        ;;
        'server')
            log info "Preparing ${PRETTY_NAME} (${ID}) to build ALVR server..."
            if [ "${2,,}" == '--build-only' ] && (maybe_clone || exit) && build_"${ID}"_server; then
                log info "${PRETTY_NAME} (${ID}) package built successfully."
            elif [ "${2,,}" == '--build-only' ]; then
                log critical "Failed to build ${PRETTY_NAME} (${ID}) package!" 4
            elif prep_"${ID}"_server; then
                if [ "${2,,}" == '--prep-only' ]; then
                    exit 0
                elif maybe_clone && build_"${ID}"_server; then
                    log info "${PRETTY_NAME} (${ID}) package built successfully."
                else
                    log critical "Failed to build ${PRETTY_NAME} (${ID}) package!" 4
                fi
            else
                log critical "Failed to prepare ${PRETTY_NAME} (${ID}) for ALVR server build!" 3
            fi
        ;;
        'all')
            ${0} server "${@:2}"
            ${0} client "${@:2}"
        ;;
        *)
            help_docs
        ;;
    esac

    rm -rf "${tmpDir}"
}

main "${@}"
