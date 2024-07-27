## Launcher (BETA)

Launcher will allow you to manage old, current and new installations of ALVR streamer and allow to automatically install and upgrade to specific ALVR app version on headset

### Installation

* Download `alvr_launcher_windows.zip` (on Windows) or `alvr_launcher_linux.tar.gz` (on Linux) from the release [download page](https://github.com/alvr-org/ALVR/releases/latest) and extract into a path that contains only ASCII characters (english only) and has edit permissions without administrator or root rights.
* Run `ALVR Launcher.exe` (on Windows) or `alvr_launcher_linux/ALVR Launcher` (on Linux)
* Press `Add version` button
* For default installation keep channel and version as is and press `Install`
* Wait until it finishes downloading, installing (depends on your connection)
* To install ALVR app on headset, use button `Install APK`
* In the list, to open streamer app (PC) press `Launch`. You will be greeted with a setup wizard. Follow the setup to set the firewall rules and other settings.

### Usage

* Before launching SteamVR through ALVR, please install it. First time launch will result in steamvr being blank and alvr will not work - close it and start again. It will have registered driver and should work.
* Launch ALVR app on your headset. While the headset screen is on, click `Trust` next to the device entry (in the ALVR streamer app on PC, in the `Devices` tab) to start streaming.
* You can change settings on the PC in the `Settings` tab. Most of the settings require to restart SteamVR to be applied. Use the apposite button on the bottom right corner.

For any problem visit the [Troubleshooting page](https://github.com/alvr-org/ALVR/wiki/Troubleshooting).

### Windows microphone streaming

To use the microphone you need to install the [VB-CABLE driver](https://vb-audio.com/Cable/).
Set "CABLE Output" as the default microphone.
Then you can enable the microphone in the ALVR setting, leave "Virtual microphone input" to Default.

## Advanced installation

### Installing app using Sidequest

* Install SideQuest on your PC and enable developer mode on the headset. You can follow [this guide](https://sidequestvr.com/setup-howto).
* Connect your headset to Sidequest. If you have Quest, Pico, and other compatible device download the ALVR app [here](https://sidequestvr.com/app/9)

### Manually installing ALVR streamer

There is also a portable version for the PC that requires more manual steps to make it work.

#### Windows

* Download `alvr_streamer_windows.zip` from the latest release [download page](https://github.com/alvr-org/ALVR/releases/latest).
* Unzip into a path that contains only ASCII characters and has edit permissions without administrator rights.
* Run

#### Linux

* Download `alvr_streamer_linux.tar.gz` from the release [download page](https://github.com/alvr-org/ALVR/releases/latest), extract it.
* Run `bin/alvr_dashboard`

#### Nightly

If you want to get new features early or you want to help with testing you can install a nightly version.

Download the latest nightly streamer [here](https://github.com/alvr-org/ALVR-nightly/releases/latest).

Since nightly releases can be unstable, always use matching versions for PC and headset. They are updated once a day.

### Arch Linux (AUR)

* Install `rustup` and a rust toolchain, if you don't have it: <https://wiki.archlinux.org/title/Rust#Arch_Linux_package>.
* Install [alvr](https://aur.archlinux.org/packages/alvr)<sup>AUR</sup> (stable, amdgpu), or [alvr-nvidia](https://aur.archlinux.org/packages/alvr-nvidia)<sup>AUR</sup> (stable, nvidia), or [alvr-git](https://aur.archlinux.org/packages/alvr-git)<sup>AUR</sup>(nightly, unstable)
* Install SteamVR, **launch it once** then close it.
* Run `alvr_dashboard` or ALVR from your DE's application launcher.

### Flatpak

For Flatpak users, refer to the instructions [here](https://github.com/alvr-org/ALVR/wiki/Installing-ALVR-and-using-SteamVR-on-Linux-through-Flatpak)

## Advanced usage

### Use ALVR together with third-party drivers

By default ALVR disables other SteamVR drivers before starting. Among these drivers there is [Driver4VR](https://www.driver4vr.com/) for full body tracking. ALVR disables these drivers to maximize compatibility with every PC setup. You can disable this behavior by manually registering the ALVR driver. Go to the `installation` tab and click on `Register ALVR driver`. The next time you launch ALVR you will be able to use the other drivers concurrently.

### Launch ALVR together with SteamVR

You can skip the ALVR Dashboard and open ALVR automatically together with SteamVR.

**Note:** You can only do that while SteamVR is not already running. Otherwise driver might be unregistered on shutdown.

Open ALVR, go to the `Installation` tab and click on `Register ALVR driver`.

### Connect headset to PC via a USB Cable

Check out the guide [here](https://github.com/alvr-org/ALVR/wiki/Using-ALVR-through-a-USB-connection).
