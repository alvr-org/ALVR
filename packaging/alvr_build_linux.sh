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
# 9 - Unable to clone repository
# 99 - Script run as root
# Disable warnings for:
# - Dynamic import shellcheck incompatibility
# - Importing snapd
# - Variable assigned but not referenced
# shellcheck disable=SC1090,SC1091,SC2034

# GitHub repo
repo='alvr-org/ALVR'
# RPM spec file
specFile='packaging/rpm/alvr.spec'
# deb control file
controlFile='packaging/deb/control'
# Android NDK version
ndkVersion=30

# Set a temporary working directory
tmpDir="/tmp/alvr_$(date '+%Y%m%d-%H%M%S')"

# Import OS info - provides ${ID}
. /etc/os-release

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
        all                 Prepare and build ALVR client and server
        client              Prepare and build ALVR client
        server              Prepare and build ALVR server
        clobber             Clobber (clean) the entire build environment and dependencies
    CARGO BUILD DEFAULTS
        Fedora              --release
        Debian-based        --release --bundle-ffmpeg
        Client              --release
    FLAGS
        --branch=           Branch to clone
        --build-only        Only build ALVR package(s)
        --prep-only         Only prepare system for ALVR package build
        --client-args=      List of ALL cargo xtask client build arguments
        --server-args=      List of ALL cargo xtask server build arguments
        --rustup-src=       Source to install rustup from if not found:
            WARNING: This does NOT affect Fedora server builds
            rustup.rs       rustup.rs script        [RUNNING UNREVIEWED ONLINE SCRIPTS IS UNRECOMMENDED]
            snapd           Snapcraft package       [Default]

Example: $(basename "${0}") server --build-only --server-args='--release --no-nvidia'
HELPME
}

# Make sure we're not building as root
[ "${USER}" == 'root' ] && log critical 'This script cannot be run as root!' 99

maybe_clone() {
    # Import distro-specific helper functions if they exist relative to the script
    for helper in "$(dirname "${0}")/alvr_build_linux_targets/"*'.sh'; do
        . "${helper}"
    done

    # If the repo doesn't exist, or we need a specific version, we should clone
    if ! [ -d "${repoDir}" ] || [ "${kwArgs['--branch']}" != '' ]; then
        log info "Cloning ${repo} into ${repoDir//$(basename "${repo}")} ..."
        git -C "${repoDir//$(basename "${repo}")}" clone -b "${kwArgs['--branch']:-master}" "https://github.com/${repo}.git" || exit 1

        # If we can, import the version-specific helpers after for compatibility
        if [ -d "${repoDir}/packaging/alvr_build_linux_targets/" ]; then
            for helper in "${repoDir}/packaging/alvr_build_linux_targets/"*'.sh'; do
                . "${helper}"
            done
        fi
    fi
}

main() {
    # Parse any flags or key / value pairs into an associative array
    declare -A kwArgs
    for kwArg in "${@:2}"; do
        # Remove everything after the '=' as the key and remove everything before the '=' as the value
        # NOTE: If there is no actual value, the value is set to the key name for ease of conditional comparisons
        # with an empty string ('')
        kwArgs["${kwArg%%=*}"]="${kwArg#*=}"
    done

    # Create temporary directory if it doesn't exist
    [ -d "${tmpDir}" ] || mkdir "${tmpDir}"

    # Grab the repository directory
    repoDir="$(realpath "$(dirname "${0}")")/.."
    if [ "${kwArgs['--branch']}" != '' ]; then
        # Use a temp directory to not screw stuff up in cwd
        repoDir="${tmpDir}/$(basename "${repo}")"
    elif ! [ -d "${repoDir}/.git" ]; then
        # Get the absolute directory the script is running in, and add the repo name
        repoDir="$(dirname "$(realpath "${0}")")/$(basename "${repo}")"
    fi
    buildDir="${repoDir}/build/alvr_server_linux/"

    # We need to clone either way for distro-specific bash functions and deb control file
    maybe_clone || log critical 'Unable to clone repository!' 9

    case "${1,,}" in
        'client')
            # This conditionally logs any build arguments
            log info "Preparing ${PRETTY_NAME} (${ID}) to build ALVR client${kwArgs['--client-args']:+" with arguments: ${kwArgs['--client-args']}"}"
            # If we're only building, clone, build, and check the exit codes
            if [ "${kwArgs['--build-only']}" != '' ] && build_"${ID}"_client; then
                log info 'ALVR client built successfully.'
            # If we got here that means we failed something
            elif [ "${kwArgs['--build-only']}" != '' ]; then
                log critical 'Failed to build ALVR client!' 2
            # Prepare and check return code
            elif prep_"${ID}"_client; then
                # Exit successfully if we're only preparing
                if [ "${kwArgs['--prep-only']}" != '' ]; then
                    log info 'ALVR build environment prepared successfully.'
                # Clone, build, and check exit codes
                elif build_generic_client; then
                    log info 'ALVR client built successfully.'
                else
                    log critical 'Failed to build ALVR client!' 2
                fi
            else
                log critical "Failed to prepare ${PRETTY_NAME} (${ID}) for ALVR client build!" 1
            fi
        ;;
        'server')
            log info "Preparing ${PRETTY_NAME} (${ID}) to build ALVR server${kwArgs['--server-args']:+" with arguments: ${kwArgs['--server-args']}"}"
            if [ "${kwArgs['--build-only']}" != '' ] && build_"${ID}"_server; then
                log info "${PRETTY_NAME} (${ID}) package built successfully."
            elif [ "${kwArgs['--build-only']}" != '' ]; then
                log critical "Failed to build ${PRETTY_NAME} (${ID}) package!" 4
            elif prep_"${ID}"_server; then
                if [ "${kwArgs['--prep-only']}" != '' ]; then
                    log info 'ALVR build environment prepared successfully.'
                elif build_"${ID}"_server; then
                    log info "${PRETTY_NAME} (${ID}) package built successfully."
                else
                    log critical "Failed to build ${PRETTY_NAME} (${ID}) package!" 4
                fi
            else
                log critical "Failed to prepare ${PRETTY_NAME} (${ID}) for ALVR server build!" 3
            fi
        ;;
        'clobber')
            log info 'Clobbering build environment ...'
            rm -rf "${repoDir}/"{'build','deps','target'}
        ;;
        'all')
            ${0} server "${@:2}"
            ${0} client "${@:2}"
        ;;
        *)
            log error "Invalid action: ${1}!" NOKILL
            help_docs
        ;;
    esac

    # If there's a failure this will not run by design so we can debug
    rm -rf "${tmpDir}"
}

main "${@}"
