#!/usr/bin/env bash
# Cross-build ALVR for Linux from macOS using Docker (Ubuntu 24.04).
#
# Usage:
#   ./docker/build-linux.sh                    # full build: prepare-deps + build-streamer + build-launcher
#   ./docker/build-linux.sh check              # cargo check -D warnings (matches CI)
#   ./docker/build-linux.sh <xtask-args...>    # run a specific cargo xtask subcommand
#   ./docker/build-linux.sh shell              # drop into an interactive shell
#
# Build artifacts land in build/alvr_streamer_linux/ and build/alvr_launcher_linux/
# as they would from CI.
#
# The Rust target dir is kept in a named Docker volume (alvr-linux-cargo-target)
# so it persists across builds without conflicting with the macOS target/ dir.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

IMAGE_NAME="alvr-linux-build"
CARGO_TARGET_VOLUME="alvr-linux-cargo-target"
CARGO_REGISTRY_VOLUME="alvr-linux-cargo-registry"

build_image() {
    # --platform linux/amd64 ensures x86_64 even on Apple Silicon — our target is
    # always x86_64 Linux and openvr/ffmpeg deps are x86_64-only.
    # docker buildx + --network=host works around buildkit DNS failures on macOS
    # Docker Desktop that affect plain `docker build --network=host`.
    # --load makes the built image available in the local docker images store.
    docker buildx build \
        --platform linux/amd64 \
        --network=host \
        --load \
        --file "$SCRIPT_DIR/Dockerfile.linux-build" \
        --tag "$IMAGE_NAME" \
        "$SCRIPT_DIR"
}

run_in_container() {
    docker run --rm \
        --volume "$REPO_ROOT:/workspace" \
        --volume "$CARGO_TARGET_VOLUME:/cargo-target" \
        --volume "$CARGO_REGISTRY_VOLUME:/root/.cargo/registry" \
        --env CARGO_TARGET_DIR=/cargo-target \
        --env CARGO_TERM_COLOR=always \
        --env RUST_BACKTRACE=1 \
        --workdir /workspace \
        "$IMAGE_NAME" \
        "$@"
}

build_image

if [[ $# -eq 0 ]]; then
    run_in_container bash -c "
        set -e
        cargo xtask prepare-deps --platform linux
        cargo xtask build-streamer --platform linux
        cargo xtask build-launcher --platform linux
    "
elif [[ "$1" == "check" ]]; then
    run_in_container bash -c "
        set -e
        cargo xtask prepare-deps --platform linux
        cargo clean -p alvr_server_openvr -p alvr_dashboard -p alvr_launcher
        RUSTFLAGS='-D warnings' cargo check \
            -p alvr_server_openvr \
            -p alvr_dashboard \
            -p alvr_launcher
    "
elif [[ "$1" == "shell" ]]; then
    run_in_container bash
else
    run_in_container cargo xtask "$@"
fi
