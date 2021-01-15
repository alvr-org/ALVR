<p align="center"> <img width="500" src="resources/alvr_combined_logo_hq.png"/> </p>

# ALVR - Air Light VR

[![badge-discord][]][link-discord] [![badge-opencollective][]][link-opencollective]

Stream VR games from your PC to your headset via Wi-Fi.  
ALVR uses technologies like [Asynchronous Timewarp](https://developer.oculus.com/documentation/native/android/mobile-timewarp-overview) and [Fixed Foveated Rendering](https://developer.oculus.com/documentation/native/android/mobile-ffr) for a smoother experience.  
All games that work with an Oculus Rift (s) should work with ALVR.  
This is a fork of [ALVR](https://github.com/polygraphene/ALVR).

|    Headset     |                        Support                         |
| :------------: | :----------------------------------------------------: |
|  Oculus Quest  |                   :heavy_check_mark:                   |
| Oculus Quest 2 |                   :heavy_check_mark:                   |
|   Oculus Go    |           :heavy_check_mark: (no controller)           |
|     GearVR     | :x: (use [this](https://github.com/polygraphene/ALVR)) |

## Requirements

-   Oculus Quest, Oculus Quest 2 or Oculus Go on the latest firmware  

-   SteamVR  

-   High-end gaming PC
    -   Windows 10 May 2020 update is recommended. If you are on an older version, you need to install Chrome or another Chromium based browser.  
    -   Minimum supported OS version is Windows 8.  
    -   NVIDIA GPU that supports NVENC ([Supported GPUs](https://github.com/polygraphene/ALVR/wiki/Supported-GPU)) (or with an AMD GPU that supports AMF VCE) with the latest driver.  
    -   Laptops with an onboard (Intel HD, AMD iGPU) and an additional dedicated GPU (NVidia GTX/RTX, AMD HD/R5/R7): you should assign the dedicated GPU or "high performance graphics adapter" to the applications ALVR, SteamVR for best performance and compatibility. (NVidia: Nvidia control panel->3d settings->application settings; AMD: similiar way) 

-   802.11ac 5Ghz wireless or ethernet wired connection  
    -   It is recommended to use 802.11ac 5Ghz for the headset and ethernet for PC  
    -   You need to connect both the PC and the headset to same router (or use a routed connection as described [here](https://github.com/alvr-org/ALVR/wiki/ALVR-client-and-server-on-separate-networks))

## Install

It is recommended (but not mandatory) to uninstall any other VR streaming software on your PC, including older versions of ALVR. If you didn't already, install SteamVR, launch it and then close it (this is to make sure SteamVR executes its first time setup).

To install ALVR download and execute `ALVR_Installer_vX.X.X.exe` from the [releases page](https://github.com/alvr-org/ALVR/releases). An entry will appear in the Start menu. The first time you open ALVR Launcher.exe you may have to allow it in the SmartScreen prompt. In the release page you can also find the portable version `alvr_server_windows.zip`. If you already have ALVR v13.1.0 or greater you can upgrade with the autoupdater.

Install the client on your headset through Sidequest ([Quest version](https://sidequestvr.com/app/9), [Go version](https://sidequestvr.com/app/2658)). To make the Oculus Quest microphone work you need to install the [VB-CABLE Virtual Audio Device](https://www.vb-audio.com/Cable/).

### Nightly versions

You can install the latest untested server version from the [nightly releases page](https://github.com/alvr-org/ALVR-nightly/releases).

The nightly client app can be installed from Sidequest ([Quest version](https://sidequestvr.com/app/2281), [Go version](https://sidequestvr.com/app/2580)) (it needs to be updated for each server update).

## Usage

-   Open `ALVR Launcher.exe` (ALVR dashboard). The first time a setup wizard will guide you through the installation. Oculus Go users should import the `oculus_go_preset.json` found the in the installation folder.  
-   Launch ALVR on your headset. While the headset screen is on, click `Trust` next to the client entry (on the PC) to start streaming.  
-   To change settings, open the dashboard on the headset with a long press of the menu button (on the left controller), change what you want and then press `Restart SteamVR`. The current playing game could shutdown so any unsaved progress could be lost.  
-   To shutdown ALVR you need to close both the ALVR dashboard on PC and SteamVR.  
-   If you want play games without launching the ALVR dashboard first, you need to register the driver. Go to Installation tab, then click on `Register ALVR driver`. This is normally discouraged because it can cause problems with other SteamVR drivers (for example the Oculus Link).

IMPORTANT: SteamVR must be always running, otherwise the dashboard will not save settings and the client will not connect.

### Notes

-   You can access ALVR dashboard from your smartphone. On the browser you need to type the local IP of your PC followed by `:8082` (for example: `192.168.0.3:8082` ).

## Troubleshooting

-   To reset ALVR, delete the file `session.json` from the installation folder.  
-   Please check the [Troubleshooting](https://github.com/alvr-org/ALVR/wiki/Troubleshooting) page. The original repository [wiki](https://github.com/polygraphene/ALVR/wiki/Troubleshooting) can also help.  
-   You can find some setup advice [here](https://github.com/alvr-org/ALVR/wiki/Setup-advice).

## Uninstall

Open `ALVR Launcher.exe`, go to `Installation` tab then press `Remove firewall rules`. Close ALVR window and delete the ALVR folder.

If you have a version prior to v12.0 you need to launch `remove_firewall_rules.bat` and `driver_uninstall.bat` in the installation folder.

## Build from source

Preferred IDE (optional): Visual Studio Code with rust-analyzer extension

### Common requisites

-   Install [LLVM](https://releases.llvm.org/download.html)  
-   Install [rustup](https://rustup.rs/)  
-   Download this repository

### Build server

-   Install the MSVC compiler (for example installing C++ build tools with [Visual Studio](https://visualstudio.microsoft.com/downloads))  

-   On the repository root execute:

    ```bash
    cargo xtask build-server --release
    ```

-   ALVR server will be built into `/build/alvr_server_windows`.

### Build client

-   Install [Python](https://www.microsoft.com/store/productId/9MSSZTT1N39L)  

-   Install Android Studio >=4.0, API Level 30. Requires latest LLDB and NDK packages.  

-   Set the environment variable `JAVA_HOME` to `C:\Program Files\Android\Android Studio\jre`.  

-   Set the environment variable `ANDROID_SDK_ROOT` to `%LOCALAPPDATA%\Android\Sdk`.  

-   On the repository root execute:

    ```bash
    cargo xtask install-deps
    cargo xtask build-client --release
    ```

-   ALVR client will be built into `/build`.

Note: After doing the above steps, you can debug the client normally by opening the Android Studio project at `alvr/client/android`.

## License

ALVR is licensed under the [MIT License](LICENSE).

## Donate

If you want to support this project you can make a donation to our [Open Source Collective account](https://opencollective.com/alvr).

You can also donate to the original author of ALVR using Paypal (polygraphene@gmail.com) or with bitcoin (1FCbmFVSjsmpnAj6oLx2EhnzQzzhyxTLEv).

[badge-discord]: https://img.shields.io/discord/720612397580025886?style=for-the-badge&logo=discord "Join us on Discord"
[link-discord]: https://discord.gg/KbKk3UM
[badge-opencollective]: https://img.shields.io/opencollective/all/alvr?style=for-the-badge "Donate"
[link-opencollective]: https://opencollective.com/alvr
