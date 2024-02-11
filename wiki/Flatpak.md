## Installing ALVR and using SteamVR on Linux through Flatpak

## Disclaimer

1. This is not a fully-featured version of ALVR! It lacks Nvidia support and has bugs related to Flatpak sandboxing

2. Nvidia GPUs are currently not supported

3. Native Linux SteamVR utility applications such as OpenVRAS are not supported nor tested, use at your own risk

4. Firewall configuration does not work

5. Any scripts that affect the host will run within the sandbox

6. Sometimes, a new instance of Steam will launch when launching the dashboard. To fix this, close both ALVR and Steam then launch Steam. As soon as Steam opens to the storefront, launch the ALVR dashboard.

7. The ALVR Dashboard is not available in the Applications menu. To run the dashboard, run the following command to run `alvr_dashboard` in the Steam Flatpak environment:

```
flatpak run --command=alvr_dashboard com.valvesoftware.Steam
```

8. This only works with the Steam Flatpak. For non-Flatpak Steam, use the AppImage instead

## Dependencies

First, flatpak must be installed from your distro's repositories. Refer to [this page](https://flatpak.org/setup/) to find the instructions for your distro.

Once Flatpak is installed, the flatpak dependencies must also be installed. They are:

* Rust
* LLVM
* Freedesktop SDK
* Steam

These can be installed like so:

```
flatpak install flathub org.freedesktop.Sdk//22.08 \
    org.freedesktop.Sdk.Extension.llvm16//22.08 \
    org.freedesktop.Sdk.Extension.rust-stable//22.08 \
    com.valvesoftware.Steam
```

AMD users may need to install the appropriate Mesa codec extensions as well:

```
flatpak install flathub org.freedesktop.Platform.GL.default//22.08-extra \
   org.freedesktop.Platform.GL32.default//22.08-extra
```

## Setup

Install SteamVR via the Steam Flatpak. After installing SteamVR, run the following command:

```
sudo setcap CAP_SYS_NICE+eip ~/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/SteamVR/bin/linux64/vrcompositor-launcher
```

This command is normally run by SteamVR, but due to the lack of sudo access within the Flatpak sandbox, it must be run outside of the Flatpak sandbox. After running the command, run SteamVR once then close it.

## Install

Download `com.valvesoftware.Steam.Utility.alvr.flatpak` file from one of the latest [nightly](https://github.com/alvr-org/ALVR-nightly/releases) that contains flatpak bundle and install like so:

```
flatpak --user install --bundle com.valvesoftware.Steam.Utility.alvr.flatpak
```

Use apk for headset from the same nightly.

## Build and Install

Alternatively, if the file is not available or a newer version is needed, the flatpak can be built from source and installed.

First, the dependencies from above must be fulfilled. Then, install `flatpak-builder` like so:

```
flatpak install flathub org.flatpak.Builder
```

Once the dependencies are fulfilled, clone and enter the repository.

```
git clone https://github.com/alvr-org/ALVR.git
cd ALVR
```

Once inside the repository, simply run the following command to build and install the Flatpak.

```
flatpak run org.flatpak.Builder --user --install --force-clean .flatpak-build-dir alvr/xtask/flatpak/com.valvesoftware.Steam.Utility.alvr.json
```

If ALVR is not cloned under the home directory, permission to access the directory may need to be given to the build command. An example of this is given below.

```
flatpak run --filesystem="$(pwd)" org.flatpak.Builder --user --install --force-clean .flatpak-build-dir alvr/xtask/flatpak/com.valvesoftware.Steam.Utility.alvr.json
```

## Notes

### Running the dashboard

To run the ALVR Dashboard, run the following command:

```
flatpak run --command=alvr_dashboard com.valvesoftware.Steam
```

A desktop file named `com.valvesoftware.Steam.Utility.alvr.desktop` is supplied within the `alvr/xtask/flatpak` directory. Move this to where other desktop files are located on your system in order to run the dashboard without the terminal.

### Automatic Audio & Microphone setup

Currently the game audio and microphone to and from the headset isn't routed automatically. The setup of this script will therefore run every time the headset connects or disconnects to the ALVR dashboard. This is based on [the steps](Installation-guide.md#automatic-audio--microphone-setup) in the installation guide, modified for the Flatpak.

1. In the ALVR Dashboard under All Settings (Advanced) > Audio, enable Game Audio and Microphone.

2. In the same place under Microphone, click Expand and set Devices to custom. Enter `default` for the name for both Sink and Source.

3. Download the [audio-flatpak-setup.sh](../alvr/xtask/flatpak/audio-flatpak-setup.sh) script and place it into the Flatpak app data directory located at `~/.var/app/com.valvesoftware.Steam/`. Make sure it has execute permissions (e.g. `chmod +x audio-flatpak-setup.sh`).

5. In the ALVR Dashboard, under All Settings (Advanced) > Connection, set the On connect script and On disconnect script to the absolute path of the script (relative to the Flatpak environment), e.g. `/home/$USER/.var/app/com.valvesoftware.Steam/audio-flatpak-setup.sh`.

6. In a terminal, run `flatpak override --user --filesystem=xdg-run/pipewire-0 com.valvesoftware.Steam` to allow the script to set and map your headset's microphone

7. Restart both Steam and the ALVR Dashboard

### Other Applications

The support for other applications that are not launched via Steam is non-existent due to the Flatpak sandbox.

Various SteamVR utilities such as [WlxOverlay](https://github.com/galister/WlxOverlay) and [OpenVR-AdvancedSettings](https://github.com/OpenVR-Advanced-Settings/OpenVR-AdvancedSettings) cannot run within the Flatpak sandbox due to their usage of AppImage. However, unpacking the supplied AppImage or building the utilities from source and running their binaries from within the sandbox similiarly to `alvr_dashboard` could work, but there is no guarantee that they will work properly.

Some applications such as [Godot](https://godotengine.org) support OpenXR. However, unless they are launched within the Steam Flatpak sandbox, they will not work with the Steam Flatpak. See [here](https://github.com/flathub/com.valvesoftware.Steam/issues/1010) for more details.
