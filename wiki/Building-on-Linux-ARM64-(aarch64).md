# Building ALVR on Linux ARM64 (aarch64)

ALVR can be built and run natively on ARM64 Linux (e.g. NVIDIA DGX Spark, Raspberry Pi 5, Apple Silicon via Asahi). Two small workarounds are needed compared to x86-64 Linux.

## Prerequisites

Same as the standard [Linux build](Building-From-Source.md#debian-12--ubuntu-2004--pop_os-2004), plus:

```bash
sudo apt install cmake git
```

## 1. Build OpenVR from source

OpenVR's official releases only include `linux64` (x86-64) prebuilt binaries. On aarch64 you must build it yourself — it compiles cleanly in about 30 seconds:

```bash
git clone --depth 1 https://github.com/ValveSoftware/openvr.git /tmp/openvr-arm64
cd /tmp/openvr-arm64
cmake -B build -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED=ON
cmake --build build -j$(nproc)

# Copy into the ALVR workspace (openvr submodule path)
cp build/bin/linux64/libopenvr_api.so /path/to/alvr-src/openvr/lib/linux64/
```

The ALVR build script (`alvr/server_openvr/build.rs`) links against `openvr/lib/linux64/libopenvr_api.so` on all Linux architectures, so placing the ARM64 binary there is all that's needed.

## 2. Build ALVR normally

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Install dependencies (Ubuntu/Debian)
sudo apt install build-essential pkg-config libclang-dev libssl-dev \
  libasound2-dev libjack-dev libgtk-3-dev libvulkan-dev libunwind-dev \
  gcc yasm nasm libx264-dev libx265-dev libxcb-render0-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libdrm-dev \
  libva-dev libvulkan-dev libpipewire-0.3-dev

# Clone
git clone --recurse-submodules https://github.com/alvr-org/ALVR.git
cd ALVR

# Place ARM64 OpenVR library (see step 1)
cp /tmp/openvr-arm64/build/bin/linux64/libopenvr_api.so openvr/lib/linux64/

# Prepare dependencies and build
cargo xtask prepare-deps --platform linux
cargo xtask build-streamer --release
```

Output: `build/alvr_streamer_linux/`

## Known ARM64 differences

- `char` is unsigned (`u8`) on ARM64 by default, unlike x86-64 where it is signed (`i8`). One instance of this affected `alvr_server_openvr` and is fixed in this patch.
- FFmpeg and NVENC compile correctly for aarch64 — `h264_nvenc`, `hevc_nvenc`, and `av1_nvenc` all build and work with NVIDIA ARM GPUs (tested on DGX Spark / GB10).
- The Vulkan layer and dashboard both run natively on aarch64.

## Tested on

| Hardware | OS | GPU | Result |
|---|---|---|---|
| NVIDIA DGX Spark (GB10) | Ubuntu 24.04 | NVIDIA GB10 (Blackwell) | ✅ Builds and runs |

## Notes

- SteamVR does not run on ARM64 Linux, which limits the streamer to non-SteamVR OpenXR workflows. See [#XXXX] for ongoing work on a standalone OpenXR server mode.
- `box64` (x86-64 emulation) can run the prebuilt ALVR binary on ARM64, but native compilation is preferred for performance.
