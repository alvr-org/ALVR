# ALVR - Air Light VR

ALVR is an open source remote VR display for Gear VR and Oculus Go/Quest. With it, you can play SteamVR games in your standalone headset.

English | [Japanese](https://github.com/polygraphene/ALVR/blob/master/README-ja.md)

## Description

ALVR streams VR display output from your PC to Gear VR / Oculus Go / Oculus Quest via Wi-Fi. This is similar to Riftcat or Trinus VR, but our purpose is optimization for Gear VR. ALVR provides smooth head-tracking compared to other apps in a Wi-Fi environment using Asynchronous Timewarp.

Note that many PCVR games require 6DoF controller or multiple buttons, so you might not able to play those games.
You can find playable games in [List of tested VR games and experiences](https://github.com/polygraphene/ALVR/wiki/List-of-tested-VR-games-and-experiences).

## Requirements

ALVR requires any of the following devices:

- Gear VR
- Oculus Go
- Oculus Quest

|Device|Working?|
|---|---|
|Oculus Quest|OK(alpha)|
|Oculus Go|OK|
|GalaxyS9/S9+|OK|
|GalaxyS8/S8+|OK|
|Galaxy Note 8|OK|
|GalaxyS7|OK|
|GalaxyS6(Edge)|OK|

- High-end gaming PC
    - with NVIDIA GPU which supports NVENC ([Supported GPUs](https://github.com/polygraphene/ALVR/wiki/Supported-GPU))
    - (or with AMD GPU which supports AMF VCE)
    - Windows 10 is recommended
    - Currently only NVIDIA GPU is supported on Windows 7
- 802.11n/ac wireless or ethernet wired connection
    - It is recommended to use 802.11ac for the headset and ethernet for PC
        - You need to connect both to the same router
- SteamVR

## Install ALVR server for PC

1. Install SteamVR
2. Download installer from [Releases](https://github.com/polygraphene/ALVR/releases)
3. Run the installer
4. Open ALVR Launcher

## Install ALVR client for headset

### For Quest: Install from apk

- Need to enable developer option for Quest.
- Download apk from [Releases](https://github.com/polygraphene/ALVR/releases)
- Check [Installation](https://github.com/polygraphene/ALVR/wiki/Installation).

### For GearVR and Go: Oculus Store

- You can download ALVR Client from Oculus Store with key.
- Open [the key distribution page](https://alvr-dist.appspot.com/) on your smartphone and follow the instruction.

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

## Future work

- SteamVR dashboard to control ALVR
- Cloud streaming

## Build

### ALVR Server and GUI (Launcher)

- Open ALVR.sln with Visual Studio 2017 and build
    - alvr\_server project is the driver for SteamVR written in C++
    - ALVR project is the launcher GUI written in C#

### ALVR Client

- Clone [ALVR Client](https://github.com/polygraphene/ALVRClient)
- Put your [osig file](https://developer.oculus.com/documentation/mobilesdk/latest/concepts/mobile-submission-sig-file/) on assets folder (only for Gear VR)
- Build with Android Studio
- Install apk via adb

## License

ALVR is licensed under MIT License.

## Donate

If you like this project, please donate!

#### Donate by paypal

[![Donate](https://img.shields.io/badge/Donate-PayPal-green.svg)](https://www.paypal.com/cgi-bin/webscr?cmd=_donations&business=polygraphene@gmail.com&lc=US&item_name=Donate+for+ALVR+developer&no_note=0&cn=&curency_code=USD&bn=PP-DonationsBF:btn_donateCC_LG.gif:NonHosted)
If you could not use this link, please try the following.
1. Login your paypal account
2. Open "Send and request" tab
3. Click "Pay for goods or services"
4. Put "polygraphene@gmail.com" (it is my paypal account) and click next

#### Donate by bitcoin

bitcoin:1FCbmFVSjsmpnAj6oLx2EhnzQzzhyxTLEv
