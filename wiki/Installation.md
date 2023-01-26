# Basic Installation
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

For any problem visit the [troubleshooting page](https://github.com/alvr-org/ALVR/wiki/Troubleshooting).

# Advanced Installation
## Portable version
There is also a portable version for the PC that requires more manual steps to make it work.

* Install SteamVR and launch it once.
* Download `alvr_server_windows.zip` from the latest release [download page](https://github.com/alvr-org/ALVR/releases/latest).
* Unzip into a path that contains only ASCII characters and has edit permissions without administrator rights.

## Nightly
If you want to get new features early or you want to help with testing you can install a nightly version.

Download the latest nightly server [here](https://github.com/alvr-org/ALVR-nightly/releases/latest). Download the latest nightly client from Sidequest ([Quest version](https://sidequestvr.com/app/2281), [Go version](https://sidequestvr.com/app/2580)).

Since nightly releases can be unstable, for maximum compatibility always use matching versions for PC and headset. They are updated once a day.

## Microphone streaming
To use the microphone you need to install the [VB-CABLE driver](https://vb-audio.com/Cable/). Set "CABLE Output" as the default microphone. Then you can enable the microphone in the ALVR setting, leave "Virtual microphone input" to Default.

## Connect headset and PC on separate networks
Check out the guide [here](https://github.com/alvr-org/ALVR/wiki/ALVR-on-separate-networks).

## ALVR with third-party drivers
By default ALVR disables other SteamVR drivers before starting. Among these drivers there is [Driver4VR](https://www.driver4vr.com/) for full body tracking. ALVR disables these drivers to maximize compatibility with every PC setup. You can disable this behavior by manually registering the ALVR driver. Go to the `Installation` tab and click on `Register ALVR driver`. The next time you launch ALVR you will be able to use the other drivers concurrently.

## Launching ALVR with SteamVR
You can skip the ALVR Launcher and open ALVR automatically together with SteamVR. Open ALVR, go to the `Installation` tab and click on `Register ALVR driver`.

## Default browser different than Chrome
ALVR requires a Chromium-based browser to correctly display the dashboard. Chrome and Edge works out of the box, but Edge has a few bugs that make ALVR behave weirdly. If you want to use other Chromium-based browsers like Brave or Vivaldi you have to add an environment variable `ALCRO_BROWSER_PATH` pointing to the path of the browser executable (for example `C:\Program Files\Vivaldi\Application\vivaldi.exe`). Unfortunately, Firefox is not supported.

## Connect headset and PC via a USB Cable
Check out the guide [here](https://github.com/alvr-org/ALVR/wiki/Using-ALVR-through-a-USB-connection).

# Linux
Unless you are using a nightly version, make sure all audio streaming options are disabled.

## Arch Linux
* Install `rustup` and a rust toolchain, if you don't have it: <https://wiki.archlinux.org/title/Rust#Arch_Linux_package>.
* Install [alvr](https://aur.archlinux.org/packages/alvr)<sup>AUR</sup> (recommended), [alvr-nightly](https://aur.archlinux.org/packages/alvr-nightly)<sup>AUR</sup>, or [alvr-git](https://aur.archlinux.org/packages/alvr-git)<sup>AUR</sup>
* Install SteamVR, **launch it once** then close it.
* Run `alvr_launcher` or ALVR from your DE's application launcher.

## Other
* Install FFmpeg with VAAPI/NVENC + DRM + Vulkan + x264/x265 support. You can use this [ppa:savoury1/ffmpeg5](https://launchpad.net/~savoury1/+archive/ubuntu/ffmpeg5) under Ubuntu, or download `alvr_server_portable.tar.gz` which has ffmpeg bundled.
* Install SteamVR, **launch it once** then close it.
* Download `alvr_server_linux(_portable).tar.gz` from the release [download page](https://github.com/alvr-org/ALVR/releases/latest).
* Run `bin/alvr_launcher`

If you do not install the correct version of FFmpeg systemwide, a common problem is the server crashing or failing to show images on the headset because SteamVR loads the wrong version of FFmpeg.

## Audio Setup
* If you are on PipeWire, install `pipewire-alsa` and `pipewire-pulse`
* `pavucontrol` and `pactl` (PulseAudio tools used as an example)

### Game Audio
* Must be on v19.0.0 or later
* Enable Game Audio in ALVR dashboard.
* Select `pipewire` or `pulse` as the device.
* Connect with headset and wait until streaming starts.
* In `pavucontrol` set the device ALVR is recording from to "Monitor of \<your audio output\>". You might have to set "Show:" to "All Streams" for it to show up.
* Any audio should now be played on the headset, optionally you can mute the audio output.

### Microphone
* Run: `pactl load-module module-null-sink sink_name=VirtMain` (will have to be ran every time you restart/relog, on pipewire can also be done in configuration file)
* Enable microphone streaming in ALVR dashboard.
* Connect with headset and wait until streaming starts.
* In `pavucontrol` set ALVR Playback to "VirtMain"
* Set "Monitor of VirtMain" as your microphone.
