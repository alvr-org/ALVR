<p align="center"> <img width="500" src="resources/alvr_combined_logo_hq.png"/> </p>

# ALVR - Air Light VR

[![badge-discord][]][link-discord] [![badge-matrix][]][link-matrix] [![badge-opencollective][]][link-opencollective]

Stream VR games from your PC to your headset via Wi-Fi.  
ALVR uses technologies like [Asynchronous Timewarp](https://developer.oculus.com/documentation/native/android/mobile-timewarp-overview) and [Fixed Foveated Rendering](https://developer.oculus.com/documentation/native/android/mobile-ffr) for a smoother experience.  
Most of the games that run on SteamVR or Oculus Software (using Revive) should work with ALVR.  
This is a fork of [ALVR](https://github.com/polygraphene/ALVR).

|    VR Headset     |         Support          |
| :---------------: | :----------------------: |
|   Quest 1/2/Pro   |    :heavy_check_mark:    |
|   Pico 4/Neo 3    |     :construction: *     |
|   Vive Focus 3    |     :construction: *     |
| Smartphone/Monado |     :construction: *     |
|      GearVR       |     :construction: *     |
|  Google Daydream  | :construction: * (maybe) |
|     Oculus Go     |          :x: **          |

\* : Coming soon  
\** : Oculus Go support was dropped on v19. Minimum supported OS is Android 8.

|        PC OS        |       Support       |
| :-----------------: | :-----------------: |
|   Windows 8/10/11   | :heavy_check_mark:  |
|    Windows 7/XP     |         :x:         |
|     Ubuntu/Arch     |    :warning: ***    |
| Other linux distros | :grey_question: *** |
|        macOS        |         :x:         |

\*** : Linux support is still in beta. To be able to make audio work or run ALVR at all you may need advanced knowledge of your distro for debugging or building from source.

## Requirements

-   A supported standalone VR headset (see compatibility table above)

-   SteamVR

-   High-end gaming PC
    -   See OS compatibility table above.
    -   NVIDIA GPU that supports NVENC (1000 GTX Series or higher) (or with an AMD GPU that supports AMF VCE) with the latest driver.
    -   Laptops with an onboard (Intel HD, AMD iGPU) and an additional dedicated GPU (NVidia GTX/RTX, AMD HD/R5/R7): you should assign the dedicated GPU or "high performance graphics adapter" to the applications ALVR, SteamVR for best performance and compatibility. (NVidia: Nvidia control panel->3d settings->application settings; AMD: similiar way)

-   802.11ac 5Ghz wireless or ethernet wired connection  
    -   It is recommended to use 802.11ac 5Ghz for the headset and ethernet for PC  
    -   You need to connect both the PC and the headset to same router (or use a routed connection as described [here](https://github.com/alvr-org/ALVR/wiki/ALVR-v14-and-Above))

## Install

Follow the Basic-installation guide [here](https://github.com/alvr-org/ALVR/wiki/Basic-Basic-installation).

## Troubleshooting

-   Please check the [Troubleshooting](https://github.com/alvr-org/ALVR/wiki/Troubleshooting) page. The original repository [wiki](https://github.com/polygraphene/ALVR/wiki/Troubleshooting) can also help.  
-   Configuration recommendations and information may be found [here](https://github.com/alvr-org/ALVR/wiki/PC)

## Uninstall

Open `ALVR Launcher.exe`, go to `Basic-installation` tab then press `Remove firewall rules`. Close ALVR window and delete the ALVR folder.

If you have a version prior to v12.0 you need to launch `remove_firewall_rules.bat` and `driver_uninstall.bat` in the Basic-installation folder.

## Build from source

You can follow the guide [here](https://github.com/alvr-org/ALVR/wiki/Building-From-Source).

## License

ALVR is licensed under the [MIT License](LICENSE).

## Privacy policy

ALVR apps do not directly collect any kind of data.

## Donate

If you want to support this project you can make a donation to our [Open Source Collective account](https://opencollective.com/alvr).

You can also donate to the original author of ALVR using Paypal (polygraphene@gmail.com) or with bitcoin (1FCbmFVSjsmpnAj6oLx2EhnzQzzhyxTLEv).

[badge-discord]: https://img.shields.io/discord/720612397580025886?style=for-the-badge&logo=discord&color=5865F2 "Join us on Discord"
[link-discord]: https://discord.gg/ALVR
[badge-matrix]: https://img.shields.io/static/v1?label=chat&message=%23alvr&style=for-the-badge&logo=matrix&color=blueviolet "Join us on Matrix"
[link-matrix]: https://matrix.to/#/#alvr:ckie.dev?via=ckie.dev
[badge-opencollective]: https://img.shields.io/opencollective/all/alvr?style=for-the-badge&logo=opencollective&color=79a3e6 "Donate"
[link-opencollective]: https://opencollective.com/alvr
