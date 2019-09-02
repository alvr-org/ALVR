# ALVR - Air Light VR

ALVR is an open source remote VR display. With it, you can play SteamVR games in your standalone headset.
This is a fork of [ALVR](https://github.com/polygraphene/ALVR) that is only working on the Oculus Quest


## Description

ALVR streams VR display output from your PC to  Oculus Quest via Wi-Fi. This is similar to Riftcat or Trinus VR, but our purpose is optimization for Oculus Quest. ALVR provides smooth head-tracking compared to other apps in a Wi-Fi environment using Asynchronous Timewarp.

All games that work with a Oculus Rift (s) should work with ALVR

## Requirements

ALVR requires any of the following devices:

- Oculus Quest (Headset-Version 358570.6090.0 or later)

- High-end gaming PC
    - with NVIDIA GPU which supports NVENC ([Supported GPUs](https://github.com/polygraphene/ALVR/wiki/Supported-GPU))
    - (or with AMD GPU which supports AMF VCE) with latest driver
    - Windows 10 is recommended
    - Currently only NVIDIA GPU is supported on Windows 7
    - Laptops with dual GPU have to disable the on-board GPU
- 802.11n/ac wireless or ethernet wired connection
    - It is recommended to use 802.11ac for the headset and ethernet for PC
        - You need to connect both to the same router
    - Lower the channel width to 40Mhz to prevent stuttering (sources: [here](https://www.reddit.com/r/OculusQuest/comments/ckx0qx/this_is_how_to_remove_periodic_frame/) and [here](https://otasyumi.site/vr/oculus-quest-steamvr-try-alvr-corresponding-to-the-latest-build-of-oculusquest) )
- SteamVR

## Install ALVR server for PC

1. Install SteamVR
2. Download latest release from [Releases](https://github.com/JackD83/ALVR/releases)
3. Unpack ALVR.zip
4. Run add_firewall_rules.bat as admin
4. Open ALVR Launcher

## Install ALVR client for headset

### For Quest: Install from apk

- Need to enable developer option for Quest.
- Download apk from [Releases](https://github.com/JackD83/ALVR/releases)
- Check [Installation](https://github.com/polygraphene/ALVR/wiki/Installation).


## Usage

- Launch ALVR.exe
- Press "Start Server" button or launch VR game
- SteamVR's small window will appear. You should see a headset icon in the SteamVR status window that looks like a green block with a bold S in the middle
- Launch ALVR Client in your headset
- IP Address of headset will appear in the server tab of ALVR.exe
- Press "Connect" button

## Troubleshoot

- If you got some error, please check [Troubleshooting](https://github.com/polygraphene/ALVR/wiki/Troubleshooting)

## Uninstallation

- Execute driver\_uninstall.bat in the driver folder
- Delete the install folder (ALVR does not use the registry)
- If you already deleted the folder without executing driver\_uninstall.bat:
    - Open C:\Users\\%USERNAME%\AppData\Local\openvr\openvrpaths.vrpath and check install directory
    - Execute
    `"C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrpathreg.exe" removedriver (install folder)`
    in Command Prompt

## Build

### ALVR Server and GUI (Launcher)

- Open ALVR.sln with Visual Studio 2017 and build
    - alvr\_server project is the driver for SteamVR written in C++
    - ALVR project is the launcher GUI written in C#
- Requires Cuda 9.2

### ALVR Client

- Clone [ALVR Client](https://github.com/JackD83/ALVRClient)
- Build with Android Studio 3.4, API Level 29. Requires LLDB and NDK to build
- Install apk via adb

## License

ALVR is licensed under MIT License.

## Donate to the original author

If you like this project, please donate to the original author!

#### Donate by paypal

[![Donate](https://img.shields.io/badge/Donate-PayPal-green.svg)](https://www.paypal.com/cgi-bin/webscr?cmd=_donations&business=polygraphene@gmail.com&lc=US&item_name=Donate+for+ALVR+developer&no_note=0&cn=&curency_code=USD&bn=PP-DonationsBF:btn_donateCC_LG.gif:NonHosted)
If you could not use this link, please try the following.
1. Login your paypal account
2. Open "Send and request" tab
3. Click "Pay for goods or services"
4. Put "polygraphene@gmail.com" (it is paypal account of the original author) and click next

#### Donate by bitcoin
bitcoin:1FCbmFVSjsmpnAj6oLx2EhnzQzzhyxTLEv
