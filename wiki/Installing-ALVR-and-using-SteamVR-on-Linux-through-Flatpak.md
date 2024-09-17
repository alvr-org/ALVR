## Disclaimer

1. Flatpak suppport is experimental - but it does seem to work. Some manual steps are needed!

2. Native Linux SteamVR utility applications such as OpenVRAS are not supported nor tested, use at your own risk

3. Firewall configuration does not work

4. Any scripts that affect the host will run within the sandbox

5. Sometimes, a new instance of Steam will launch when launching the dashboard. To fix this, close both ALVR and Steam then launch Steam. As soon as Steam opens to the storefront, launch the ALVR dashboard.

6. User must setup xdg shortcut themselves - see below. Without an xdg entry the launcher has to be run from terminal.

```sh
flatpak run --command=alvr_launcher com.valvesoftware.Steam
```

8. This does seem to work with both steam flatpak and native steam - it calls via xdg-open. But it is not recommended to have both versions of steam installed as this creates ambiguity.

## Dependencies

First, flatpak must be installed from your distro's repositories. Refer to [this page](https://flatpak.org/setup/) to find the instructions for your distro.

## Setup

Flatpak steam needs extra step compared to native steam. After installing SteamVR, run the following command:

```sh
sudo setcap CAP_SYS_NICE+eip ~/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/SteamVR/bin/linux64/vrcompositor-launcher
```

This command is normally run by SteamVR, but due to the lack of sudo access within the Flatpak sandbox, it must be run outside of the Flatpak sandbox. After running the command, run SteamVR once then close it.

### steamvr custom launch options
At the time of writing steamvr needs special options to work on linux - this applies to both the flatpak version and native. The flatpak uses a slightly different path is the only difference. Paths below assume steam has been installed in the "normal" location - if your steam is in a different place then adjust paths as appropriate.

For flatpak steam
```
~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%
```

For native steam
```
~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%
```

### failed to create pipewire errors
Use flatseal to add permissions to steam - in Filesystem section - "otherfiles" - add new entry with content: "xdg-run/pipewire-0"
Should see some other permissions there "xdg-music:ro", "xdg-pictures:ro" and maybe more for other integration (like discord).
TODO: add nice picture of what exactly this looks like, or shell command to do it

## Install

Download `com.valvesoftware.Steam.Utility.alvr.flatpak` file from one of the latest [nightly](https://github.com/alvr-org/ALVR-nightly/releases) that contains flatpak bundle and install like so:

```sh
flatpak install --user com.valvesoftware.Steam.Utility.alvr.flatpak
```

## Notes

### Running the launcher

It's recommended that user sets up an xdg shortcut - but the launcher can also be run from terminal via the following command:
```sh
flatpak run --command=alvr_launcher com.valvesoftware.Steam
```

An icon and desktop file named `com.valvesoftware.Steam.Utility.alvr.desktop` is supplied within the `alvr/xtask/flatpak` directory. Move this to where other desktop files are located on your system in order to run the dashboard without the terminal.

```sh
# systemwide shortcut
# sudo cp com.valvesoftware.Steam.Utility.alvr.desktop /var/lib/flatpak/exports/share/applications/ 

# users local folder
cp com.valvesoftware.Steam.Utility.alvr.desktop $HOME/.local/share/flatpak/exports/share/applications/

# install icon as well
xdg-icon-resource install --size 256 alvr_icon.png application-alvr-launcher
```

The shortcut may not appear until desktop session is refreshed (e.g. log off then back on)

### EXPERIMENTAL - APK install via flatpak launcher 
First need to setup adb on host, and enable usb debugging on device. Verify that devices shows up when you run "adb devices" and is authorised.
Script assumes that user has AndroidStudio installed with keys in default location ($HOME/.android/adbkey.pub) - change if necessary
Convenience script is provided in git: run_with_adb_keys.sh
It's likely one the keys are exposed to the flatpak in the default location it will work without needing more changes.
```
export ADB_VENDOR_KEYS=~/.android/adbkey.pub
flatpak override --user --filesystem=~/.android com.valvesoftware.Steam.Utility.alvr
flatpak run --env=ADB_VENDOR_KEYS=$ADB_VENDOR_KEYS --command=alvr_launcher com.valvesoftware.Steam
```

### Wayland variable causes steamvr error:
Make sure the QT_QPA_PLATFORM var allows x11 option - or steamvr freaks out. Launch from terminal to see errors.
This can be a problem if you have modified this variable globally to force usage of wayland for some program like GameScope. 
You can fix this by setting the variable passed to steamvr
Example custom launch options for steamvr - including both QT_QPA_PLATFORM and vrmonitor fixes:

```
QT_QPA_PLATFORM=xcb ~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%
```

### Hybrid graphics 
If using desktop it's recommended to disable igpu - makes things simpler. 
If using laptop then must pass extra options to ensure dgpu is used. These options are in addition to the others already mentioned.

#### Amd/Intel integrated gpu + Amd/Intel discrete gpu
Put DRI_PRIME=1 %command% into SteamVR's commandline options and in those of all VR games you intend to play with ALVR.
```
DRI_PRIME=1 QT_QPA_PLATFORM=xcb ~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%
```

#### Amd/Intel integrated gpu + Nvidia discrete gpu
Put __NV_PRIME_RENDER_OFFLOAD=1 __VK_LAYER_NV_optimus=NVIDIA_only __GLX_VENDOR_LIBRARY_NAME=nvidia %command% into SteamVR's commandline options and in those of all VR games you intend to play with ALVR. Again - in addition to other options.
```
__NV_PRIME_RENDER_OFFLOAD=1 __VK_LAYER_NV_optimus=NVIDIA_only __GLX_VENDOR_LIBRARY_NAME=nvidia QT_QPA_PLATFORM=xcb ~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%
```

### Other Applications

The support for other applications that are not launched via Steam is non-existent due to the Flatpak sandbox.

Various SteamVR utilities such as [WlxOverlay](https://github.com/galister/WlxOverlay) and [OpenVR-AdvancedSettings](https://github.com/OpenVR-Advanced-Settings/OpenVR-AdvancedSettings) cannot run within the Flatpak sandbox due to their usage of AppImage. However, unpacking the supplied AppImage or building the utilities from source and running their binaries from within the sandbox similiarly to `alvr_dashboard` could work, but there is no guarantee that they will work properly.

(at time of writing it does work properly)
Download wlx-overlay-s appimage. 
Make it executable (chmod +x Wlx-Overlay-xxx.Appimage). 
Extract it (./Wlx-Overlay-xxx.Appimage --app-image-extract)
Use flatseal or terminal to expose a folder to the steam flatpak (e.g. ~/test, should be in same section as the pipewire fix from above)
Copy the extracted files into the exposed folder.
Test it from terminal: flatpak run --command=bash com.valvesoftware.Steam (cd ~/test/squasroot-fs && ./Apprun)
To make a desktop shortcut, use a command like flatpak run --command=~/test/squashroot-fs/Apprun com.valvesoftware.Steam



Some applications such as [Godot](https://godotengine.org) support OpenXR. However, unless they are launched within the Steam Flatpak sandbox, they will not work with the Steam Flatpak. See [here](https://github.com/flathub/com.valvesoftware.Steam/issues/1010) for more details.
