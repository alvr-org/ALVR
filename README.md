<img align="left" width="120" height="120" src="https://github.com/JackD83/ALVR/blob/master/server_release_template/web_gui/web_hi_res_512.png">

# ALVR - Air Light VR

[![Badge-Discord]][Link-Discord]

Stream VR games from your PC to your Oculus Quest via Wi-FI.  
ALVR uses technologies like Asynchronous Timewarp and Fixed Foveated Rendering for a smoother experience.  
All games that work with an Oculus Rift (s) should work with ALVR.  
This is a fork of [ALVR](https://github.com/polygraphene/ALVR) that works only with Oculus Quest.

## Requirements

- Oculus Quest (Headset-Version 358570.6090.0 or later)
- SteamVR
- High-end gaming PC
  - Windows 10 May 2020 update is recommended. If you are on an older version, you need to install Chrome or another Chromium based browser.
  - Minimum supported OS version is Windows 8.
  - NVIDIA GPU that supports NVENC ([Supported GPUs](https://github.com/polygraphene/ALVR/wiki/Supported-GPU)) (or with an AMD GPU that supports AMF VCE) with the latest driver.
  - Laptops with an onboard (Intel HD, AMD iGPU) and an additional dedicated GPU (NVidia GTX/RTX, AMD HD/R5/R7): you should assign the dedicated GPU or "high performance graphics adapter" to the applications ALVR, SteamVR for best performance and compatibility. (NVidia: Nvidia control panel->3d settings->application settings; AMD: similiar way)
- 802.11ac wireless or ethernet wired connection
  - It is recommended to use 802.11ac for the headset and ethernet for PC
  - You need to connect both the PC and the headset to same router (or use a routed connection as described [here](https://github.com/JackD83/ALVR/wiki/ALVR-client-and-server-on-separate-networks))

## Install

Please uninstall any other VR streaming software on your PC. This includes versions of ALVR prior to v12.0.

Install the latest [Visual C++ Redistrubutable x64 package](https://support.microsoft.com/en-us/help/2977003/the-latest-supported-visual-c-downloads). Do this every time you install a new ALVR version!

To install ALVR download and unzip `alvr_server_windows.zip` wherever you want and launch `ALVR.exe`. The first time you open ALVR.exe you may have to allow it in the SmartScreen prompt and allow the network access to alvr_web_server in another popup window (it could be behind the actual ALVR window).  
It's important not to move the folder after the first launch. To keep settings from a previous installation of ALVR (>=v12.0) you can unzip over the old installation folder.

Install the client on your headset through [SideQuest](https://sidequestvr.com/).

## Usage

- Launch `ALVR.exe` (ALVR dashboard). The first time a setup wizard will guide you through the installation.
- Launch ALVR on your headset. While the headset screen is on, click `Trust` next to the client entry (on the PC) to start streaming.
- If you trusted a client, you can start streaming by just launching ALVR on your headset, then SteamVR or the ALVR dashboard on PC.
- To change settings, open the dashboard on the headset with a long press of the menu button (on the left controller), change what you want and then press `Restart SteamVR`. The current playing game could shutdown so any unsaved progress could be lost.
- To shutdown ALVR you need to close both the ALVR dashboard on PC and SteamVR.

### Notes

- You can access ALVR dashboard from your smartphone. On the browser you need to type the local IP of your PC followed by `:8082` (for example: `192.168.0.3:8082` ).

## Troubleshooting

- To reset ALVR, delete the file `session.json` from the installation folder.
- Please check the [Troubleshooting](https://github.com/polygraphene/ALVR/wiki/Troubleshooting) page on the original repository.
- You can find some setup advice [here](https://github.com/JackD83/ALVR/wiki/Setup-advice).

## Uninstall

Launch `ALVR.exe`, go to `About` tab, press `Uninstall driver` and `Remove firewall rules`. Close ALVR window and delete the ALVR folder.

If you have a version prior to v12.0 you need to launch `remove_firewall_rules.bat` and `driver_uninstall.bat` in the installation folder manually.

## Build from source

- Install Visual Studio Code and the extension rust-analyzer (optional)
- Install [LLVM](https://releases.llvm.org/download.html)
- Install the MSVC compiler (for example installing C++ build tools with [Visual Studio](https://visualstudio.microsoft.com/downloads))
- Install Android Studio >=4.0, API Level 30. Requires LLDB and NDK. The environment variable `JAVA_HOME` must be set.
- Install [rustup](https://rustup.rs/)
- Download this repository and on the project root execute:

    ```bash
    cargo xtask build-all --release
    ```

- ALVR server and client will be in `/build`.

Note: The Visual Studio solution is left only for IDE support while coding. If compiled, the resulting binary will not be valid.

## License

ALVR is licensed under the [MIT License](LICENSE).

## Donate to the original author

If you like this project, please donate to the original author!

### Donate with PayPal

[![Donate](https://img.shields.io/badge/Donate-PayPal-green.svg)](https://www.paypal.com/cgi-bin/webscr?cmd=_donations&business=polygraphene@gmail.com&lc=US&item_name=Donate+for+ALVR+developer&no_note=0&cn=&curency_code=USD&bn=PP-DonationsBF:btn_donateCC_LG.gif:NonHosted)

If you cannot use this link, please try the following.

1. Login with your PayPal account
2. Open "Send and request" tab
3. Click "Pay for goods or services"
4. Put "polygraphene@gmail.com" (it's the PayPal account of the original author) and click next

### Donate with bitcoin

bitcoin:1FCbmFVSjsmpnAj6oLx2EhnzQzzhyxTLEv

[Badge-Discord]: https://img.shields.io/discord/720612397580025886?style=for-the-badge&logo=discord "Join us on Discord"
[Link-Discord]: https://discord.gg/KbKk3UM
