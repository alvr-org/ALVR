#!/usr/bin/env bash
# Generic functions to prepare and build packages
# Disable warnings for importing snapd and variable referenced but not assigned
# shellcheck disable=SC1091,SC2154
prep_rustup() {
    # Manually add the default path
    export PATH="${PATH}:/snap/bin:/var/lib/snapd/snap"

    # Install rustup ONLY if it does not exist
    if ! command -v rustup; then
        case "${kwArgs['--rustup-src'],,}" in
            'rustup.rs')
                # Untested
                curl -sSf https://sh.rustup.rs | sh
            ;;
            # If the keyword is the value, there was no keyword specified
            'snap' | '--rustup-src') ! sudo snap install rustup --classic && exit 7 ;;
            *)
                log critical "Neither Rustup installation nor Rustup source type found; bad source was: ${kwArgs['--rustup-src']}" 7
            ;;
        esac
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

    log info "Linking Android ${ndkVersion} NDK toolchain to generic ..."
    if ! [ -L "${toolchainRoot}/aarch64-linux-android-clang" ]; then
        ln -s "${toolchainRoot}/"{"aarch64-linux-android${ndkVersion}-clang",'aarch64-linux-android-clang'}
    fi
    if ! [ -L "${toolchainRoot}/aarch64-linux-android-clang++" ]; then
        ln -s "${toolchainRoot}/"{"aarch64-linux-android${ndkVersion}-clang++",'aarch64-linux-android-clang++'}
    fi

    # Get the version
    apkVer="-$(grep -P '^version' "${repoDir}/alvr/common/Cargo.toml" | sed -E 's/^version = "(.*)"$/\1/')${buildVer}"

    log info 'Starting client build ...'
    # no subshell expansion warnings
    cd "${repoDir}" > /dev/null || return 2
    if cargo xtask build-android-deps && cargo xtask build-client ${kwArgs['--client-args']:---release}; then
        # Move and rename the files at the top of the build directory
        mv "${repoDir}/build/alvr_client_oculus_go/"* "${repoDir}/build/alvr_client_oculus_go${apkVer}.apk"
        mv "${repoDir}/build/alvr_client_oculus_quest/"* "${repoDir}/build/alvr_client_oculus_quest${apkVer}.apk"
        rmdir "${repoDir}/build/"{'alvr_client_oculus_quest','alvr_client_oculus_go'}
        cd - > /dev/null || return 2
    else
        cd - > /dev/null && return 2
    fi
}
