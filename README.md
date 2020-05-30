<img align="left" width="128" height="128" src="https://github.com/JackD83/ALVR/blob/web-ui/server_release_template/web_gui/web_hi_res_512.png">

# ALVR - Air Light VR

Stream VR games from your PC to your Oculus Quest via Wi-FI.  
ALVR uses technologies like Asynchronous Timewarp and Fixed Foveated Rendering for a smoother experience.

All games that work with an Oculus Rift (s) should work with ALVR.  
This is a fork of [ALVR](https://github.com/polygraphene/ALVR) that works only with Oculus Quest.

## Requirements

- Oculus Quest (Headset-Version 358570.6090.0 or later)
- SteamVR
- High-end gaming PC
  - Windows 10 May 2020 update is recommended. If you are on an older version, you need to install Chrome or another Chromium based browser.
  - NVIDIA GPU that supports NVENC ([Supported GPUs](https://github.com/polygraphene/ALVR/wiki/Supported-GPU)) (or with an AMD GPU that supports AMF VCE) with the latest driver.
  - Currently only NVIDIA GPUs are supported on Windows 7.
  - Laptops with dual GPU have to disable the on-board GPU.
- 802.11ac wireless or ethernet wired connection
  - It is recommended to use 802.11ac for the headset and ethernet for PC
  - You need to connect both the PC and the headset to same router (or use a routed connection as described [here](https://github.com/JackD83/ALVR/wiki/ALVR-client-and-server-on-separate-networks))

## Install

Please uninstall any other VR streaming software on your PC. This includes versions of ALVR prior to v11.0.

To install ALVR just download and unzip `alvr_server_windows.zip` wherever you want and launch `ALVR.exe`. It's important not to move the folder after the first launch.
To keep settings from a previous installation of ALVR (>=v11.0) you can unzip over the old installation folder.

Install the client on your headset through [SideQuest](https://sidequestvr.com/).

## Usage

- Launch `ALVR.exe`. The first time a setup wizard will guide you through the installation.
- Launch ALVR on your headset. In the VR dashboard, next to the client entry, check `Connect automatically` then press `Start`.
- To change settings, open the dashboard with the menu button (on the left controller), change what you want and then press `Apply`. SteamVR will restart, so any unsaved progress will be lost.
- To shutdown ALVR you need to close both the ALVR window on PC and SteamVR.

### Notes

- After the first time configuration, ALVR can be launched by simply launching SteamVR, but first you need to put on the headset and launch ALVR client.
- You can access ALVR dashboard from your smartphone. On the browser you need to type the local IP of your PC followed by `:8082` (for example: `192.168.0.3:8082` ).

## Troubleshooting

- Floorlevel: Use the SteamVR room setup to calibrate the room as standing only. Put your Quest on the ground while calibrating. Make sure that the stream is still working by covering the light sensor of the quest. Enter a height of 0 into the room setup.
Now you can press and hold the oculus key on the right controller to recenter SteamVR and fix the floor height at any time.
- To reset ALVR, delete the file `session.json` from the installation folder.
- Please check the [Troubleshooting](https://github.com/polygraphene/ALVR/wiki/Troubleshooting) page on the original repository.
- You can find some setup advice [here](https://github.com/JackD83/ALVR/wiki/Setup-advice).

## Uninstall

Launch `ALVR.exe`, go to `About` tab, press `Uninstall driver` and `Remove firewall rules`. Close ALVR window and delete the ALVR folder.

If you have a version prior to 11.0 you need to launch `remove_firewall_rules.bat` and `driver_uninstall.bat` in the installation folder manually.

## Build from source

- Install Visual Studio Code and the extension rust-analyzer (optional)
- Install [Visual Studio Community 2019](https://visualstudio.microsoft.com/downloads) with C++ build tools
- Alternatively, if you already have a Visual Studio 2019 installation, you can add the environment variable `MSBUILD_DIR` pointing to the folder containing `MSBuild.exe`
- Install [CUDA 10.2](https://developer.nvidia.com/cuda-downloads?target_os=Windows&target_arch=x86_64&target_version=10&target_type=exenetwork)
- Install Android Studio >=3.4, API Level 29. Requires LLDB and NDK. The environment variable `JAVA_HOME` must be set.
- Install [rustup](https://rustup.rs/)
- Download this repository and on the project root execute:

    ```bash
    cargo xtask install-deps
    cargo xtask build-all --release
    ```

- ALVR server and client will be in `/build`.

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
