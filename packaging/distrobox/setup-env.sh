#!/bin/bash

source ./helper-functions.sh

# Required on xorg setups
if [[ -z "$WAYLAND_DISPLAY" ]]; then
    xhost "+si:localuser:$USER"
    if [ $? -ne 0 ]; then
        echor "Couldn't use xhost, please install it and re-run installation"
        exit 1
    fi
fi

prefix="installation"

if [[ -n "$(which podman)" ]] && [[ -n "$(which distrobox)" ]]; then
    echog "Using system podman and distrobox"
    return
fi

init_prefixed_installation "$@"

export CONTAINERS_CONF="$prefix/.config/containers/containers.conf"
export CONTAINERS_REGISTRIES_CONF="$prefix/.config/containers/registries.conf"
export CONTAINERS_STORAGE_CONF="$prefix/.config/containers/storage.conf"
export PATH="$prefix/podman/bin:$prefix/distrobox/bin:$PATH"