ALVR can be built on Windows and Linux. The following instructions are for both OSes.

# Common Prerequisites

Preferred IDE (optional): Visual Studio Code with rust-analyzer extension

You need to install [rustup](https://www.rust-lang.org/tools/install).

On Windows you need also [Chocolatey](https://chocolatey.org/install).

# Streamer Building

First you need to gather some additional resources in preparation for the build.  

If you are on Linux, install these additional packages:

* **Arch**
  
  ```bash
  sudo pacman -S clang curl nasm pkgconf yasm vulkan-headers libva-mesa-driver unzip ffmpeg
  ```
  
  * The [`alvr-git`](https://aur.archlinux.org/packages/alvr-git) [AUR package](https://wiki.archlinux.org/title/Arch_User_Repository) may also be used to do this automatically.

* **Gentoo**
  
  * `media-video/ffmpeg >= 4.4 [encode libdrm vulkan vaapi]`
  * `sys-libs/libunwind`
  * `dev-lang/rust >= 1.72`
  * `media-video/pipewire [jacksdk]`

* **Nix(OS)**
  
  * Use the `shell.nix` in `packaging/nix`.

* **Debian 12 / Ubuntu 20.04 / Pop!\_OS 20.04**
  
  ```bash
  sudo apt install build-essential pkg-config libclang-dev libssl-dev libasound2-dev libjack-dev libgtk-3-dev libvulkan-dev libunwind-dev gcc-8 g++-8 yasm nasm curl libx264-dev libx265-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libdrm-dev libva-dev libvulkan-dev vulkan-headers
  ```

* **Fedora**
  
  ```bash
  sudo dnf groupinstall 'Development Tools' | For c++ and build tools
  sudo dnf install yasm libdrm-devel vulkan-headers pipewire-jack-audio-connection-kit-devel atk-devel gdk-pixbuf2-devel cairo-devel rust-gdk0.15-devel x264-devel vulkan-devel libunwind-devel clang openssl-devel alsa-lib-devel libva-devel
  ```
  
  If you are using Nvidia, see [Fedora cuda installation](https://github.com/alvr-org/ALVR/wiki/Building-From-Source#fedora-cuda-installation)

Move to the root directory of the project, then run this command (paying attention to the bullet points below):

```bash
cargo xtask prepare-deps --platform [your platform] [--gpl] [--no-nvidia]
```

* Replace `[your platform]` with your computer OS, either `windows` or `linux`
* **Windows only:** Use the `--gpl` flag if you want to download, build and bundle FFmpeg inside the ALVR streamer. Keep in mind that this is only needed for software encoding. As the name suggests, if you use this flag you can only redistribute the final package as GPLv2.0 licensed; because of the x264 encoder.
* **Linux only:** Use the `--no-nvidia` flag if you have a AMD gpu.

Next up is the proper build of the streamer. Run the following:

```bash
cargo xtask build-streamer --release [--gpl]
```

**Windows only:** Again, the `--gpl` flag is needed only if you want to bundle FFmpeg.

You can find the resulting package in `build/alvr_streamer_[your platform]`

If you want to edit and rebuild the code, you can skip the `prepare-deps` command and run only the `build-streamer` command.

## Fedora CUDA installation

If you are here for CUDA installation on Fedora you're at the right place! Else continue down to [Client Building](https://github.com/alvr-org/ALVR/wiki/Building-From-Source#client-building)

### 1. Install Nvidia drivers and Fedora CUDA driver

```bash
sudo dnf update -y
```

(Reboot if you have a new kernel)

```bash
sudo dnf install akmod-nvidia
sudo dnf install xorg-x11-drv-nvidia-cuda
```

Wait until ```modinfo -F version nvidia``` doesn't report ```"ERROR: Module nvidia not found"``` anymore

### 2. Install Nvidia's CUDA

In the previous step, we installed Fedora's CUDA that doesn't work with ALVR, installing Nvidia's CUDA works and creates directories instead

```bash
sudo dnf config-manager --add-repo https://developer.download.nvidia.com/compute/cuda/repos/fedora37/x86_64/cuda-fedora37.repo
```

Change the Fedora version if you are on a different version. You should check if your version is supported by inspecting the repo

```bash
sudo dnf clean all
sudo dnf module disable nvidia-driver
sudo dnf -y install cuda
export PATH=/usr/local/cuda-12.3/bin${PATH:+:${PATH}}
```

If your cuda version is different, change it to the version that is installed. You can check installed versions by doing ```ls /usr/local/ | grep "cuda"``` in your terminal

**Comments**

* Disabling the nvidia-driver doesn't disable Nvidia drivers but prevents nvidia dkms from installing over the akmod driver

### 3. Install gcc11 install with homebrew

Becuase cuda cannot be ran without a gcc version lower than or equal to gcc12, you will need to install a gcc version on homebrew. The fedora gcc11 package got removed so this is the only way sadly
To install homebrew, run this command:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Then install gcc11

```bash
brew install gcc@11
```

**Comments**

* If brew is not found in your path, run the following separately to add brew to your path:
  
  ```bash
  test -d ~/.linuxbrew && eval "$(~/.linuxbrew/bin/brew shellenv)" 
  test -d /home/linuxbrew/.linuxbrew && eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
  echo "eval \"\$($(brew --prefix)/bin/brew shellenv)\"" >> ~/.bashrc
  ```

### 4. Modify dependencies.rs to use correct cuda path and gcc version

Because CURA installs as a symlink by default, we need to change the dependencies.rs to use the directory
From the ALVR directory edit the ./alvr/xtask/src/dependencies.rs, and change two lines:

* Line 159, change ```cuda``` -> ```cuda-12.3``` (or whatever version you have)
* Line 179, replace that line with ```--nvccflags=\"-ccbin /home/linuxbrew/.linuxbrew/bin/g++-11 -gencode arch=compute_52,code=sm_52 -O2\"``` (Change homebrew path if needed, default is used)

You should be good to go! Refer to [Streamer Building](https://github.com/alvr-org/ALVR/wiki/Building-From-Source#streamer-building) for the commands to build ALVR

# Client Building

## 1. Installing necessary packages

For the client you need install:

* [Android Studio](https://developer.android.com/studio) or the [sdkmanager](https://developer.android.com/studio/command-line/sdkmanager)
* Android SDK Platform-Tools 29 (Android 10)
* Latest Android NDK (currently v25.1.8937393)

On Linux, the specific package names for the android tools can differ from distro to distro, see up on the wiki for more information:

* Gentoo:
  * https://wiki.gentoo.org/wiki/Android
* Arch:
  * https://wiki.archlinux.org/title/Android
* Debian:
  * https://wiki.debian.org/AndroidStudio
* Ubuntu:
  * https://help.ubuntu.com/community/AndroidSDK
* Pop!\_OS:
  * N/A

The three mentioned developer applications can be installed from upstream; although the packages and setup responsible for the required tools can differ between distros, being:

* **Arch**
  * Packages can vary, read up on the Arch Wiki's [Android](https://wiki.archlinux.org/title/Android) page.
* **Gentoo**
  * `dev-util/android-studio`
  * `dev-util/android-sdk-update-manager`
  * `dev-util/android-ndk >= 25.1`

For Debian, it requires to have the `non-free` repository to be enabled:

* **Debian 12 / Ubuntu 22.10 / Pop!\_OS 22.10**
  
  ```bash
  sudo apt install android-sdk-platform-tools-common sdkmanager google-android-ndk-r26b-installer
  ```
  
  ## 2. Setting environment variables

For Windows, set the environment variables:

* `JAVA_HOME`:
  * Example: `C:\Program Files\Android\Android Studio\jre`
* `ANDROID_HOME`:
  * Example: `%LOCALAPPDATA%\Android\Sdk`
* `ANDROID_NDK_HOME`:
  * Example: `%LOCALAPPDATA%\Android\Sdk\ndk\25.1.8937393`

For Linux, the correct directories for the environment variables can greatly differ depending on the type of install. See the wiki page of your distro for more information:

* Gentoo:
  * https://wiki.gentoo.org/wiki/Android
* Ubuntu:
  * https://help.ubuntu.com/community/AndroidSDK#Post-Installation_Configuration

Distro wikis that weren't listed above does not mention of environment variables, although generally they would be as:

* `JAVA_HOME`:
  * `/usr/lib/jvm/default-java/bin`
* `ANDROID_HOME`:
  * Arch: `~/Android/Sdk`
  * Gentoo: `~/Android`
  * Debian / Ubuntu / Pop!\_OS: `~/AndroidSDK`
* `ANDROID_NDK_HOME`:
  * Arch: `/opt/android-sdk/ndk`
  * Linux: `/usr/lib/android-sdk/ndk`

## 3. Building

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
