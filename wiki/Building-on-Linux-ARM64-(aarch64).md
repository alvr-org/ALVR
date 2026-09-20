# Building ALVR on Linux ARM64 (aarch64)

ALVR can be built and run natively on ARM64 Linux (e.g. NVIDIA DGX Spark, Raspberry Pi 5, Apple Silicon via Asahi). Two small workarounds are needed compared to x86-64 Linux.

## Prerequisites

Same as the standard Linux build, plus:

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
