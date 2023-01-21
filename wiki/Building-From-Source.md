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
  sudo pacman -Sy clang curl nasm pkgconf yasm vulkan-headers libva-mesa-driver unzip ffmpeg
  ```

The [`alvr-git`](https://aur.archlinux.org/packages/alvr-git) [AUR package](https://wiki.archlinux.org/title/Arch_User_Repository) may also be used to do this automatically.

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
* Platform Tools 33 (Android 13)
* Latest NDK (currently v25.1.8937393)
* Set the environment variable `JAVA_HOME`
  * For example on Windows: `C:\Program Files\Android\Android Studio\jre`
* Set the environment variable `ANDROID_SDK_ROOT`
  * For example on Windows: `%LOCALAPPDATA%\Android\Sdk`
* Set the environment variable `ANDROID_NDK_HOME`
  * For example on Windows: `%LOCALAPPDATA%\Android\Sdk\ndk\25.1.8937393`

First you need to gather some additional resources in preparation for the build.  
Move to the root directory of the project, then run this command:

```bash
cargo xtask prepare-deps --platform android
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
