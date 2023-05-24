#!/bin/bash

source ./helper-functions.sh

prefix="installation"
container_name="arch-alvr"

init_prefixed_installation "$@"
source ./setup-dev-env.sh "$prefix"

echog "If you're using something else than 'sudo', please write it as-is now, otherwise just press enter to confirm deletion of $prefix folder with $container_name container."
read -r ROOT_PERMS_COMMAND
if [[ -z "$ROOT_PERMS_COMMAND" ]]; then
   ROOT_PERMS_COMMAND="sudo"
fi

podman stop "$container_name" 2>/dev/null

"$ROOT_PERMS_COMMAND" rm -rf "$prefix"
DBX_SUDO_PROGRAM="$ROOT_PERMS_COMMAND" distrobox-rm --rm-home "$container_name" 2>/dev/null

echog "Uninstall completed."
