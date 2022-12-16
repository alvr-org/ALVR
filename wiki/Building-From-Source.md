ALVR can be built on Windows and Linux.

# Windows

Preferred IDE (optional): Visual Studio Code with rust-analyzer extension

### Prerequisites

 * [Chocolatey](https://chocolatey.org/install)
 * [rustup](https://rustup.rs/). Alternatively install with: `choco install rustup.install` 

## Server

**Note: These instructions are for the master branch.**

On the repository root execute:
```
cargo xtask prepare-deps --platform windows
cargo xtask build-server --release
```
ALVR server will be built into `build/alvr_server_windows/`.

To compile with software encoding support execute:
```
cargo xtask build-server --release --gpl
```
This will download and use FFmpeg binaries that are GPL licensed.

## Client

 * [Android Studio](https://developer.android.com/studio)
 * Latest NDK (currently 25.1.8937393)
 * Environment variable `JAVA_HOME` set to `C:\Program Files\Android\Android Studio\jre`
 * Environment variable `ANDROID_SDK_ROOT` set to `%LOCALAPPDATA%\Android\Sdk`
 * Environment variable `ANDROID_NDK_HOME` set to `%LOCALAPPDATA%\Android\Sdk\ndk\25.1.8937393`

On the repository root execute:
```
cargo xtask prepare-deps --platform android
cargo xtask build-client --release 
```
ALVR client will be built into `build/alvr_client_<platform>/`.

# Linux
**Note: Linux builds of ALVR are still experimental!**

## Supported GPU Configurations

 * AMD using radv is known to work, with hardware encoding
 * AMD using amdvlk does not work
 * NVIDIA using proprietary driver works, with hardware encoding.
 * Intel is untested

## Packaged Builds

#### Deb and RPM Distributions
The build script located at `packaging/alvr_build_linux.sh` allows building of client and server together or independently, along with installation of any necessary dependencies if requested. This script will respect existing git repositories; if you would like a specific release, simply clone this repository at the release tag you need, then run the script in the directory above the repository.

#### Note:
 * Fedora **client** builds are not recommended as they may potentially pollute the system Rust install; better support for this will be added later
 * Releases prior to the merge of [PR 786](https://github.com/alvr-org/ALVR/pull/786) will not function due to a lack of required packaging files
 * This script is designed to request superuser permissions only when neccessary; do not run it as root

#### Usage:
```
Usage: alvr_build_linux.sh ACTION
Description: Script to prepare the system and build ALVR package(s)
Arguments:
    ACTIONS
        all             Prepare and build ALVR client and server
        client          Prepare and build ALVR client
        server          Prepare and build ALVR server
    FLAGS
        --build-only    Only build ALVR package(s)
        --prep-only     Only prepare system for ALVR package build
```

#### Example:
```bash
git clone https://github.com/alvr-org/ALVR.git
./ALVR/packaging/alvr_build_linux.sh all
```

## Server

### Dependencies

You need [rustup](https://rustup.rs/) and the following platform specific dependencies:

* **Arch**
  ```bash
  sudo pacman -Syu clang curl nasm pkgconf yasm vulkan-headers libva-mesa-driver unzip
  ```

* **Gentoo**
    * `media-video/ffmpeg >= 4.4 [encode libdrm vulkan vaapi]`
    * `sys-libs/libunwind`
    * `dev-lang/rust >= 1.51`

* **Nix(OS)**

    Use the `shell.nix` in `packaging/nix`.

* **Ubuntu / Pop!_OS 20.04**
    ```bash
    sudo apt install build-essential pkg-config libclang-dev libssl-dev libasound2-dev libjack-dev libgtk-3-dev libvulkan-dev libunwind-dev gcc-8 g++-8 yasm nasm curl libx264-dev libx265-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libdrm-dev
    ```

### Building

Bundled version:
```bash
cargo xtask prepare-deps --platform linux --gpl [--no-nvidia]
cargo xtask build-server --release --gpl
```

To use the system ffmpeg install (you need a ffmpeg version with vulkan) you just run:
```bash
cargo xtask build-server --release
```

## Client

### Dependencies
* **Arch**
  ```bash
  sudo pacman -Syu git unzip rustup cargo jre11-openjdk-headless jdk8-openjdk clang python libxtst fontconfig lib32-gcc-libs lib32-glibc libxrender
  ```
  - Android SDK (can be installed using [android-sdk](https://aur.archlinux.org/packages/android-sdk)<sup>AUR</sup>)
  ```bash
  sudo ${ANDROID_HOME}/tools/bin/sdkmanager "patcher;v4" "ndk;22.1.7171670" "cmake;3.10.2.4988404" "platforms;android-31" "build-tools;32.0.0"
  ```

### Building
```bash
cargo xtask prepare-deps --platform android
cargo xtask build-client --release
```

### Docker
You can also build the client using Docker: https://gist.github.com/m00nwtchr/fae4424ff6cda5772bf624a08005e43e

### Troubleshooting

On some distributions, Steam Native runs ALVR a little better. To get Steam Native on Ubuntu run it with:
```bash
env STEAM_RUNTIME=0 steam
```

On Arch Linux, you can also get all the required libraries by downloading the `steam-native-runtime` package from the multilib repository
```bash
sudo pacman -S steam-native-runtime
```

Dependencies might be missing then, so run:
```bash
cd ~/.steam/root/ubuntu12_32
file * | grep ELF | cut -d: -f1 | LD_LIBRARY_PATH=. xargs ldd | grep 'not found' | sort | uniq
```

Some dependencies have to be fixed manually for example instead of forcing a downgrade to libffi version 6 (which could downgrade a bunch of the system) you can do a symlink instead (requires testing):

```bash
cd /lib/i386-linux-gnu
ln -s libffi.so.7 libffi.so.6
```
and
```bash
cd /lib/x86_64-linux-gnu
ln -s libffi.so.7 libffi.so.6
```

A few dependencies are distro controlled, you can attempt to import the package at your own risk perhaps needing the use of alien or some forced import commands, but its not recommended (turns your system into a dependency hybrid mess), nor supported!
