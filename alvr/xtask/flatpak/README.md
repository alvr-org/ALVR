# ALVR Launcher Flatpak

This is an experimental Flatpak for ALVR Launcher! It is **only** compatible with the Flatpak version of Steam! For all non-Flatpak Steam users, use the AppImage that is already provided.

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

From launcher - file browser and APK install buttons do not work yet. For now download apk from github and use sidequest to install. The main functionality of launcher (download and run streamer) does seem to work. 

Certain fixes may need to be manually applied - similar to the non-flatpak version of alvr. At this time this means fix for vrmonitor.sh and optionally sudo set cap to stop steamvr complaining. Even with a working setup steamvr may will print errors and have buggy windows - the same as non-flatpak version.

## Additional notes

Previously this flatpak had a portable build environment for ALVR - but now the launcher exists this was not necessary. A portable build environment is a useful thing - so use git history to resurrect if needed!