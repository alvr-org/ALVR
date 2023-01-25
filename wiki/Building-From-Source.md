ALVR can be built on Windows and Linux. The following instructions are for both OSes.

# Common prerequisites

Preferred IDE (optional): Visual Studio Code with rust-analyzer extension

You need to install [rustup](https://www.rust-lang.org/tools/install).

On Windows you need also [Chocolatey](https://chocolatey.org/install).

# Server build

First you need to gather some additional resources in preparation for the build.  

If you are on Linux, install these additional packages:

* **Arch**

  ```bash
  sudo pacman -S clang curl nasm pkgconf yasm vulkan-headers libva-mesa-driver unzip ffmpeg
  ```

The [`alvr-git`](https://aur.archlinux.org/packages/alvr-git) [AUR package](https://wiki.archlinux.org/title/Arch_User_Repository) may also be used to do this automatically.

* **Gentoo**

  * `media-video/ffmpeg >= 4.4 [encode libdrm vulkan vaapi]`
  * `sys-libs/libunwind`
  * `dev-lang/rust >= 1.51`

* **Nix(OS)**

  Use the `shell.nix` in `packaging/nix`.

* **Debian 12 / Ubuntu 20.04 / Pop!_OS 20.04**
  ```bash
  sudo apt install build-essential pkg-config libclang-dev libssl-dev libasound2-dev libjack-dev libgtk-3-dev libvulkan-dev libunwind-dev gcc-8 g++-8 yasm nasm curl libx264-dev libx265-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libdrm-dev libva-dev libvulkan-dev vulkan-headers
  ```

Move to the root directory of the project, then run this command (paying attention to the bullet points below):

```bash
cargo xtask prepare-deps --platform [your platform] [--gpl] [--no-nvidia]
```

* Replace `[your platform]` with your computer OS, either `windows` or `linux`
* Use the `--gpl` flag if you want to download, build and bundle FFmpeg inside the ALVR server. Keep in mind that on Windows this is needed only for software encoding; on Linux FFmpeg is always needed, and if you omit the `--gpl` flag, the system ffmpeg libraries will be used, which can cause compatibility issues at runtime. As the name suggest, if you use this flag you can only redistribute the final package as GPLv2.0 licensed (because of the x264 encoder).
* Use the flag `--no-nvidia` only on Linux and if you have an AMD gpu.
* This is not required if your Linux distribution includes Vulkan support in its ffmpeg package, like Arch (see above).

Next up is the proper build of the server. Run the following:

```bash
cargo xtask build-server --release [--gpl]
```

Again, the `--gpl` flag is needed only if you want to bundle FFmpeg.

You can find the resulting package in `build/alvr_server_[your platform]`

If you want to edit and rebuild the code, you can skip the `prepare-deps` command and run only the `build-server` command.

# Client build

For the client you need install:

* [Android Studio](https://developer.android.com/studio) or the [sdkmanager](https://developer.android.com/studio/command-line/sdkmanager)
* Latest Android SDK Platform-Tools 33 (Android 13)
* Latest Android NDK (currently v25.1.8937393)

On Linux, the three mentioned developer applications can be installed on Linux.

* **Arch**

  ```bash
  sudo pacman -S tandroid-sdk-platform-tools sdkmanager android-ndk
  ```

* **Gentoo**

  * `dev-util/android-studio`
  * `dev-util/android-sdk-update-manager`
  * `dev-util/android-ndk >= 25.1`

* **Debian 12 / Ubuntu 22.10 / Pop!\_OS 22.10** (Debian requires to have the `non-free` repository enabled)
  ```bash
  sudo apt install android-sdk-platform-tools-common sdkmanager google-android-ndk-r25b-installer
  ```

Set the environment variables:
  * `JAVA_HOME`:
    * Windows: `C:\Program Files\Android\Android Studio\jre`
    * Linux: `/usr/lib/jvm/default-java/bin`
  * `ANDROID_SDK_ROOT`:
    * Windows: `%LOCALAPPDATA%\Android\Sdk`
    * Linux: `~/AndroidSDK`
      * Arch: `~/Android/Sdk`
      * Gentoo: `~/Android`
  * `ANDROID_NDK_HOME`:
    * Windows: `%LOCALAPPDATA%\Android\Sdk\ndk\25.1.8937393`
    * Linux: `/usr/lib/android-sdk/ndk`

First you need to gather some additional resources in preparation for the build.  
Move to the root directory of the project, then run this command:

```bash
cargo xtask prepare-deps --platform android
```

Before building the client, Android has to have us to agree to the licenses otherwise building the client will halt and fail. To accept the agreements, follow the instructions for your corresponding OS:
* Windows:
  ```shell
  cd "%ANDROID_SDK_ROOT%\tools\bin"
  sdkmanager.bat --licenses
  ```
* Linux:
  ```bash
  cd ~/AndroidSDK
  sdkmanager --licenses
  ```

Next up is the proper build of the client. Run the following:

```bash
cargo xtask build-client --release
```

The built APK will be in `build/alvr_client_quest`. You can then use adb or SideQuest to install it to your headset.

## `openxr-client` branch

To build and run:

```bash
cd alvr/client_openxr
cargo apk run
```

You need the headset to be connected via USB and with the screen on to successfully launch the debugger and logcat.

# Troubleshooting (Linux)

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
