# ALVR Flatpak

This is an experimental Flatpak for ALVR! It is **only** compatible with the Flatpak version of Steam! For all non-Flatpak Steam users, use the AppImage that is already provided.

## Installation

Currently, no precompiled builds are available. However, building from source does not take very long, and just requires the usage of the terminal.

1. Install the Flatpak dependencies 

```
flatpak install flathub org.flatpak.Builder org.freedesktop.Sdk//22.08 \
    org.freedesktop.Sdk.Extension.llvm16//22.08 \
    org.freedesktop.Sdk.Extension.rust-stable//22.08
```

2. Clone and enter this repository

```
git clone https://github.com/alvr-org/ALVR.git
cd ALVR
```

3. Build and install the flatpak

```
flatpak run org.flatpak.Builder --user --install --force-clean .flatpak-build-dir alvr/xtask/flatpak/com.valvesoftware.Steam.Utility.alvr.json
```

## Usage

To launch the ALVR Dashboard, run the following command:

```
flatpak run --command=alvr_dashboard com.valvesoftware.Steam
```

## Caveats

Launching SteamVR from the dashboard will always launch a new instance of Steam. To avoid this, register the ALVR driver with Steam from the dashboard. However, the dashboard will not appear if SteamVR is launched from Steam. If any configuration needs to be made, launch the dashboard like the above. If the visibility of the Steam client does not matter, then simply launch SteamVR from the dashboard. Otherwise, launch SteamVR from inside of Steam after the driver is registered.
