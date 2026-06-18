#!/usr/bin/env bash
# Cross-build ALVR for Windows (x86_64-pc-windows-msvc) from macOS using Docker.
#
# Usage:
#   ./docker/build-windows.sh                  # full build
#   ./docker/build-windows.sh <xtask-args...>  # run a specific cargo xtask subcommand
#   ./docker/build-windows.sh shell            # interactive shell for debugging
#
# The Windows MSVC SDK is downloaded by cargo-xwin on first run and cached in a
# named Docker volume (alvr-xwin-cache). libvpl is cross-compiled from source on
# first run and the result cached in deps/windows/libvpl/alvr_build/.
#
# Note: cargo xtask build-streamer/build-launcher do not support Linux→Windows
# cross-compilation (they don't pass --target). The default build here compiles
# the server and launcher packages directly via cargo xwin.
#
# Usage:
#   ./docker/build-windows.sh         # full build
#   ./docker/build-windows.sh check   # cargo xwin check -D warnings (matches CI)
#   ./docker/build-windows.sh shell   # interactive shell for debugging

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

IMAGE_NAME="alvr-windows-build"
CARGO_TARGET_VOLUME="alvr-windows-cargo-target"
CARGO_REGISTRY_VOLUME="alvr-windows-cargo-registry"
XWIN_CACHE_VOLUME="alvr-xwin-cache"

TARGET="x86_64-pc-windows-msvc"

build_image() {
    DOCKER_BUILDKIT=0 docker build \
        --network=host \
        --file "$SCRIPT_DIR/Dockerfile.windows-build" \
        --tag "$IMAGE_NAME" \
        "$SCRIPT_DIR"
}

run_in_container() {
    docker run --rm \
        --volume "$REPO_ROOT:/workspace" \
        --volume "$CARGO_TARGET_VOLUME:/cargo-target" \
        --volume "$CARGO_REGISTRY_VOLUME:/root/.cargo/registry" \
        --volume "$XWIN_CACHE_VOLUME:/root/.cache/cargo-xwin" \
        --env CARGO_TARGET_DIR=/cargo-target \
        --env XWIN_INCLUDE_ATL=true \
        --env CARGO_TERM_COLOR=always \
        --env RUST_BACKTRACE=1 \
        --workdir /workspace \
        "$IMAGE_NAME" \
        "$@"
}

# Cross-compile libvpl for Windows x64 using clang-cl + xwin MSVC SDK.
# Installs into deps/windows/libvpl/alvr_build/ (include/ + lib/) to match
# the paths expected by alvr/server_openvr/build.rs.
setup_libvpl() {
    cat <<'INNER'
set -e
LIBVPL_VERSION="2.15.0"
DEST="/workspace/deps/windows/libvpl/alvr_build"
XWIN="/root/.cache/cargo-xwin/xwin"

if [ -f "$DEST/lib/vpl.lib" ]; then
    echo "==> libvpl already built, skipping."
else
    echo "==> Downloading libvpl ${LIBVPL_VERSION} source..."
    curl -fsSL -o /tmp/libvpl.zip \
        "https://github.com/intel/libvpl/archive/refs/tags/v${LIBVPL_VERSION}.zip"
    unzip -q /tmp/libvpl.zip -d /tmp/libvpl-src
    SRC="/tmp/libvpl-src/libvpl-${LIBVPL_VERSION}"

    # Patch cmake steps that fail on a Linux cross-build host:
    # InstallRequiredSystemLibraries queries VS install dirs (Windows-only info).
    # env/ tries to install a directory as a program (cross-build artifact).
    sed -i "s/include(InstallRequiredSystemLibraries)/# cross-build skip/" "$SRC/CMakeLists.txt"
    sed -i "s/add_subdirectory(env)/# cross-build skip/" "$SRC/CMakeLists.txt"

    # CMake toolchain file for Windows cross-compilation via clang-cl + xwin MSVC SDK
    cat > /tmp/windows_toolchain.cmake << 'TOOLCHAIN'
cmake_policy(SET CMP0091 NEW)
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_SYSTEM_PROCESSOR AMD64)
set(CMAKE_C_COMPILER clang-cl)
set(CMAKE_CXX_COMPILER clang-cl)
set(CMAKE_AR llvm-lib)
set(CMAKE_RANLIB true)
set(CMAKE_C_COMPILER_WORKS ON)
set(CMAKE_CXX_COMPILER_WORKS ON)
# Static CRT to match ALVR's /MT build
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded")
set(CMAKE_C_FLAGS_RELEASE "/MT /O2 /Ob2 /DNDEBUG" CACHE STRING "" FORCE)
set(CMAKE_CXX_FLAGS_RELEASE "/MT /O2 /Ob2 /DNDEBUG" CACHE STRING "" FORCE)
TOOLCHAIN

    IMSVC_FLAGS="-fuse-ld=lld-link \
        --target=x86_64-pc-windows-msvc \
        /imsvc${XWIN}/crt/include \
        /imsvc${XWIN}/sdk/include/ucrt \
        /imsvc${XWIN}/sdk/include/um \
        /imsvc${XWIN}/sdk/include/shared \
        /EHsc"

    LINK_FLAGS="\
        /LIBPATH:${XWIN}/crt/lib/x86_64 \
        /LIBPATH:${XWIN}/sdk/lib/um/x86_64 \
        /LIBPATH:${XWIN}/sdk/lib/ucrt/x86_64"

    echo "==> Cross-compiling libvpl for Windows x64..."
    cmake -B /tmp/libvpl-build \
        -S "$SRC" \
        -DCMAKE_TOOLCHAIN_FILE=/tmp/windows_toolchain.cmake \
        "-DCMAKE_C_FLAGS=${IMSVC_FLAGS}" \
        "-DCMAKE_CXX_FLAGS=${IMSVC_FLAGS}" \
        "-DCMAKE_EXE_LINKER_FLAGS=${LINK_FLAGS}" \
        "-DCMAKE_SHARED_LINKER_FLAGS=${LINK_FLAGS}" \
        -DUSE_MSVC_STATIC_RUNTIME=ON \
        -DBUILD_SHARED_LIBS=OFF \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="$DEST"

    cmake --build /tmp/libvpl-build --config Release -j"$(nproc)"
    cmake --install /tmp/libvpl-build --config Release

    rm -rf /tmp/libvpl.zip /tmp/libvpl-src /tmp/libvpl-build /tmp/windows_toolchain.cmake
    echo "==> libvpl cross-compiled and installed to ${DEST}."
fi
INNER
}

build_image

if [[ $# -eq 0 ]]; then
    run_in_container bash -c "
        $(setup_libvpl)
        cargo xwin build \
            --target $TARGET \
            -p alvr_server_openvr \
            -p alvr_dashboard \
            -p alvr_launcher
    "
elif [[ "$1" == "check" ]]; then
    run_in_container bash -c "
        cargo clean -p alvr_server_openvr -p alvr_dashboard -p alvr_launcher
        RUSTFLAGS='-D warnings' cargo xwin check \
            --target $TARGET \
            -p alvr_server_openvr \
            -p alvr_dashboard \
            -p alvr_launcher
    "
elif [[ "$1" == "shell" ]]; then
    run_in_container bash
else
    run_in_container cargo xtask "$@"
fi
