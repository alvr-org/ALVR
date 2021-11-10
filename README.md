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

Follow the installation guide [here](https://github.com/alvr-org/ALVR/wiki/Installation).

## Usage

Follow the guide [here](https://github.com/alvr-org/ALVR/wiki/Usage).

## Troubleshooting

-   Please check the [Troubleshooting](https://github.com/alvr-org/ALVR/wiki/Troubleshooting) page. The original repository [wiki](https://github.com/polygraphene/ALVR/wiki/Troubleshooting) can also help.  
-   Configuration recommendations and information may be found [here](https://github.com/alvr-org/ALVR/wiki/Configuration-Information-and-Recommendations)

## Uninstall

Open `ALVR Launcher.exe`, go to `Installation` tab then press `Remove firewall rules`. Close ALVR window and delete the ALVR folder.

If you have a version prior to v12.0 you need to launch `remove_firewall_rules.bat` and `driver_uninstall.bat` in the installation folder.

## Build from source

You can follow the guide [here](https://github.com/alvr-org/ALVR/wiki/Build-from-source)

## License

ALVR is licensed under the [MIT License](LICENSE).

## Privacy policy

ALVR apps do not directly collect any kind of data.

## Donate

If you want to support this project you can make a donation to our [Open Source Collective account](https://opencollective.com/alvr).

You can also donate to the original author of ALVR using Paypal (polygraphene@gmail.com) or with bitcoin (1FCbmFVSjsmpnAj6oLx2EhnzQzzhyxTLEv).

[badge-discord]: https://img.shields.io/discord/720612397580025886?style=for-the-badge&logo=discord "Join us on Discord"
[link-discord]: https://discord.gg/ALVR
[badge-opencollective]: https://img.shields.io/opencollective/all/alvr?style=for-the-badge "Donate"
[link-opencollective]: https://opencollective.com/alvr
