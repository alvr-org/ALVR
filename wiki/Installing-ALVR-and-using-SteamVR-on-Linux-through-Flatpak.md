# PREREQUISITES
Must have both flatpak and flatpak steam already installed. Verify steam is working correctly with flatscreen games before attempting to use ALVR.  

# ALVR Launcher Flatpak

This is an experimental Flatpak for ALVR Launcher! It is **only** compatible with the Flatpak version of Steam! For all non-Flatpak Steam users, use the non-flatpak launcher that is already provided.

## Installation

Currently, no precompiled builds are available. However, building from source does not take very long, and just requires the usage of the terminal.

1. Install the Flatpak dependencies - which is just "flatpak builder" - it will download the other dependencies defined in the json file

```sh
flatpak install flathub org.flatpak.Builder
```

2. Clone and enter this repository

```sh
git clone https://github.com/alvr-org/ALVR.git
cd ALVR
```

3. Build and install the flatpak via the provided script

```sh
cd alvr/xtask/flatpak
./build_and_install.sh  
```

## SteamVR setup

Install SteamVR via the Steam Flatpak. After installing SteamVR, run the following command:

```sh
sudo setcap CAP_SYS_NICE+eip ~/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/SteamVR/bin/linux64/vrcompositor-launcher
```

This command is normally run by SteamVR, but due to the lack of sudo access within the Flatpak sandbox, it must be run outside of the Flatpak sandbox. After running the command, run SteamVR once then close it.

Another manual fix needs to be applied (vrmonitor.sh) - this time as custom steamvr launch options. A similar fix is needed for the non-flatpak version, only the path is diffrent.
```
~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%
```

## Usage

Because this is an extension we can't write our xdg shortcut into steams folder - it is read-only.
Thus the launcher has to be run manually, or the shortcut installed manually. 
A convenience script ("setup_xdg_shortcut.sh") to copy the shortcut and icon is provided in same directory as above. Note you may need to logoff or otherwise refresh desktop to get the icon to appear.

To launch manually, run the following command used in the shortcut:

```sh
flatpak run --command=alvr_launcher com.valvesoftware.Steam
```

## Caveats

Flatpak graphics drivers must match host drivers. This is especially problematic with Nvidia GPU. Remember to always do flatpak update after any major system update. You may then see new graphics packages being downloaded - it should automatically select the appropriate version.

Launching SteamVR from the dashboard will always launch a new instance of Steam. To avoid this, register the ALVR driver with Steam from the dashboard. However, the dashboard will not appear if SteamVR is launched from Steam. If any configuration needs to be made, launch the dashboard like the above. If the visibility of the Steam client does not matter, then simply launch SteamVR from the dashboard. Otherwise, launch SteamVR from inside of Steam after the driver is registered.

From launcher - file browser does not work yet. APK install feature requires additional setup for keys. 

Certain fixes may need to be manually applied - similar to the non-flatpak version of alvr. At this time this means fix for vrmonitor.sh and optionally sudo set cap to stop steamvr complaining. Even with a working setup steamvr may will print errors and have buggy windows - the same as non-flatpak version.

### failed to create pipewire errors
Use flatseal to add permissions to steam - in Filesystem section - "otherfiles" - add new entry with content: "xdg-run/pipewire-0"
Should see some other permissions there "xdg-music:ro", "xdg-pictures:ro" and maybe more for other integration (like discord).
TODO: add nice picture of what exactly this looks like, or shell command to do it


### SteamVR does not seem to like HDR being enabled with nvidia gpu
At the time of writing using a desktop environment with hdr enabled seems to break steamvr when using nvidia. The symptom is steamvr not rendering and showing a popup saying "update your graphics drivers". To fix this disable hdr.
This is not reported to be an issue with amd graphics cards.


### ADB doesnt't work in flatpak: 
First need to setup adb on host, and enable usb debugging on device. Verify that devices shows up when you run "adb devices" and is authorised.
Script assumes that user has AndroidStudio installed with keys in default location ($HOME/.android/adbkey.pub) - change if necessary
Convenience script is provided: run_with_adb_keys.sh
```
export ADB_VENDOR_KEYS=~/.android/adbkey.pub
flatpak override --user --filesystem=~/.android com.valvesoftware.Steam.Utility.alvr
flatpak run --env=ADB_VENDOR_KEYS=$ADB_VENDOR_KEYS --command=alvr_launcher com.valvesoftware.Steam
```

If you get error saying "no devices" exist then check "adb devices" on host. Unplug/replug device and check again. If still stuck reboot then test again (seriously).

### Wayland variable causes steamvr error:
Make sure the QT_QPA_PLATFORM var allows x11 option - or steamvr freaks out. Launch from terminal to see errors.
This can be a problem if you have modified this variable globally to force usage of wayland for some program like GameScope. 
You can fix this by setting the variable passed to steamvr

Example custom launch options for steamvr - including both QT_QPA_PLATFORM and vrmonitor fixes:
```
QT_QPA_PLATFORM=xcb ~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%
```

## Additional notes

Previously this flatpak had a portable build environment for ALVR - but now the launcher exists this was not necessary. The build environment is available in a different folder (alvr/xtask/flatpak_build_environment)

### Other Applications

The support for other applications that are not launched via Steam is non-existent due to the Flatpak sandbox.

Various SteamVR utilities such as [WlxOverlay](https://github.com/galister/WlxOverlay) and [OpenVR-AdvancedSettings](https://github.com/OpenVR-Advanced-Settings/OpenVR-AdvancedSettings) cannot run within the Flatpak sandbox due to their usage of AppImage. However, unpacking the supplied AppImage or building the utilities from source and running their binaries from within the sandbox similiarly to `alvr_dashboard` could work, but there is no guarantee that they will work properly.

Some applications such as [Godot](https://godotengine.org) support OpenXR. However, unless they are launched within the Steam Flatpak sandbox, they will not work with the Steam Flatpak. See [here](https://github.com/flathub/com.valvesoftware.Steam/issues/1010) for more details.
