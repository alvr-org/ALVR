# ALVR Launcher Flatpak

This is an experimental Flatpak for ALVR Launcher! It is **only** compatible with the Flatpak version of Steam! For all non-Flatpak Steam users, use the non-flatpak launcher that is already provided.

## Installation

Currently, no precompiled builds are available. However, building from source does not take very long, and just requires the usage of the terminal.

1. Install the Flatpak dependencies

```
flatpak install flathub org.flatpak.Builder org.freedesktop.Sdk//23.08 
```

2. Clone and enter this repository

```
git clone https://github.com/alvr-org/ALVR.git
cd ALVR
```

3. Build and install the flatpak via the provided script

```
cd alvr/xtask/flatpak
./build_and_install.sh  
```

## Usage

Because this is an extension we can't write our xdg shortcut into steams folder - it is read-only.
Thus the launcher has to be run manually, or the shortcut installed manually. 
A convenience script ("setup_xdg_shortcut.sh") to copy the shortcut and icon is provided in same directory as above. Note you may need to logoff or otherwise refresh desktop to get the icon to appear.

To launch manually, run the following command used in the shortcut:

```
flatpak run --command=alvr_launcher com.valvesoftware.Steam
```

## Caveats

Flatpak graphics drivers must match host drivers. This is especially problematic with Nvidia GPU. Remember to always do flatpak update after any major system update. You may then see new graphics packages being downloaded - it should automatically select the appropriate version.

Launching SteamVR from the dashboard will always launch a new instance of Steam. To avoid this, register the ALVR driver with Steam from the dashboard. However, the dashboard will not appear if SteamVR is launched from Steam. If any configuration needs to be made, launch the dashboard like the above. If the visibility of the Steam client does not matter, then simply launch SteamVR from the dashboard. Otherwise, launch SteamVR from inside of Steam after the driver is registered.

From launcher - file browser does not work yet. APK install feature requires additional setup for keys. 

Certain fixes may need to be manually applied - similar to the non-flatpak version of alvr. At this time this means fix for vrmonitor.sh and optionally sudo set cap to stop steamvr complaining. Even with a working setup steamvr may will print errors and have buggy windows - the same as non-flatpak version.

Fix for flatpak steamvr setup error set cap: https://github.com/flathub/com.valvesoftware.Steam/issues/898
```
sudo setcap CAP_SYS_NICE+ep ~/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/SteamVR/bin/linux64/vrcompositor-launcher
```

Fix for flatpak steamvr "vrmonitor.sh" issue:
Path is slightly different vs native steam, but fix is same - add to steamvr launch options:
```
~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%
```
### failed to create pipewire errors
Use flatseal to add permission in "otherfiles" section for: xdg-run/pipewire-0


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

Previously this flatpak had a portable build environment for ALVR - but now the launcher exists this was not necessary. A portable build environment is a useful thing - so use git history to resurrect if needed!
