# Installation guide

## Basic installation

PC side:

* Install SteamVR, **launch it once** then close it. This is to make sure it sets the environment correctly for ALVR.
* Go to the latest release [download page](https://github.com/alvr-org/ALVR/releases/latest). In the "Assets" section at the bottom download the ALVR Installer.
* Run the installer. If prompted, allow the execution in the SmartScreen popup. You need to give administrator permissions to install ALVR. For best compatibility do not change the installation folder.
* Once the installation finished, launch ALVR. You are greeted with a setup wizard. Follow the setup to set the firewall rules and presets.

**If you have problems launching ALVR, follow the guide below to use the portable version**

Headset side:

* Install SideQuest on your PC and enable developer mode on the headset. You can follow [this guide](https://sidequestvr.com/setup-howto).
* Connect your headset to Sidequest. If you have an Oculus Quest 1/2 download the ALVR app [here](https://sidequestvr.com/app/9), if you have an Oculus Go download it [here](https://sidequestvr.com/app/2658)

### Usage

* Launch ALVR on your headset. While the headset screen is on, click `Trust` next to the client entry (on the PC) to start streaming.
* You can change settings on the PC in the `Settings` tab. Most of the settings require to restart SteamVR to be applied. Use the apposite button on the bottom right corner.

For any problem visit the [Troubleshooting page](https://github.com/alvr-org/ALVR/wiki/Troubleshooting).

## Advanced installation

### Portable version

There is also a portable version for the PC that requires more manual steps to make it work.

* Install SteamVR and launch it once.
* Download `alvr_streamer_windows.zip` from the latest release [download page](https://github.com/alvr-org/ALVR/releases/latest).
* Unzip into a path that contains only ASCII characters and has edit permissions without administrator rights.

### Nightly

If you want to get new features early or you want to help with testing you can install a nightly version.

Download the latest nightly streamer [here](https://github.com/alvr-org/ALVR-nightly/releases/latest). Download the latest nightly client from Sidequest ([download](https://sidequestvr.com/app/2281)).

Since nightly releases can be unstable, always use matching versions for PC and headset. They are updated once a day.

### Windows microphone streaming

To use the microphone you need to install the [VB-CABLE driver](https://vb-audio.com/Cable/). Set "CABLE Output" as the default microphone. Then you can enable the microphone in the ALVR setting, leave "Virtual microphone input" to Default.

### Use ALVR together with third-party drivers

By default ALVR disables other SteamVR drivers before starting. Among these drivers there is [Driver4VR](https://www.driver4vr.com/) for full body tracking. ALVR disables these drivers to maximize compatibility with every PC setup. You can disable this behavior by manually registering the ALVR driver. Go to the `installation` tab and click on `Register ALVR driver`. The next time you launch ALVR you will be able to use the other drivers concurrently.

### Launch ALVR together with SteamVR

You can skip the ALVR Dashboard and open ALVR automatically together with SteamVR. Open ALVR, go to the `Installation` tab and click on `Register ALVR driver`.

### Connect headset and PC via a USB Cable

Check out the guide [here](https://github.com/alvr-org/ALVR/wiki/Using-ALVR-through-a-USB-connection).

## Linux

### Arch Linux (AUR)

* Install `rustup` and a rust toolchain, if you don't have it: <https://wiki.archlinux.org/title/Rust#Arch_Linux_package>.
* Install [alvr](https://aur.archlinux.org/packages/alvr)<sup>AUR</sup> (recommended), or [alvr-git](https://aur.archlinux.org/packages/alvr-git)<sup>AUR</sup>
* Install SteamVR, **launch it once** then close it.
* Run `alvr_dashboard` or ALVR from your DE's application launcher.

### Semi-Automatic Distrobox installation and guidance

Notes:

* This is generally recommended way to install ALVR on non-arch distributions and can practically work on any distribution. You can of course use it in case you have issues installing or using ALVR even on Arch Linux.

* Guide also contains fixes, tweaks, additional software like desktop overlay to help you run with steamvr better and workaround it's issues.

* It didn't have a *huge* feedback history yet, so if you happen to have issues, please report them into [the said](https://github.com/alvr-org/ALVR-Distrobox-Linux-Guide) repository.

Installation:

1. `git clone https://github.com/alvr-org/ALVR-Distrobox-Linux-Guide.git` or download zip from https://github.com/alvr-org/ALVR-Distrobox-Linux-Guide, unpack it
   somewhere in your home directory (steam doesn't like long paths)

1.1. If you want to use nightly (potentially unstable, but fresh) builds use [nightly](https://github.com/alvr-org/ALVR-Distrobox-Linux-Guide/tree/nightly) branch.

2. `cd ALVR-Distrobox-Linux-Guide`

3. Carefully follow the [guide](ALVR-in-distrobox.md).

4. Any issues related to this installer/various tweaks/bugs should be reported as issue [here](https://github.com/alvr-org/ALVR-Distrobox-Linux-Guide/issues)

### Other

#### AppImage

You can get appimage for latest stable version from [here](https://github.com/alvr-org/ALVR/releases/latest).

#### Flatpak

For Flatpak users, refer to the instructions [here](https://github.com/alvr-org/ALVR/wiki/Flatpak)

#### Portable tar.gz

* Install FFmpeg with VAAPI/NVENC + DRM + Vulkan + x264/x265 support. You can use this [ppa:savoury1/ffmpeg5](https://launchpad.net/~savoury1/+archive/ubuntu/ffmpeg5) under Ubuntu.
* Install SteamVR, **launch it once** then close it.
* Download `alvr_streamer_linux.tar.gz` from the release [download page](https://github.com/alvr-org/ALVR/releases/latest).
* Run `bin/alvr_dashboard`

### Automatic Audio & Microphone Setup

* Must be on v20.5.0+

* Pipewire required

* Open installation -> Run setup wizard, skip to part with automatic audio setup

* Press the button to automatically download and set it
