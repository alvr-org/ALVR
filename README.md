# ALVR - Air Light VR

ALVR is an opensource remote VR display for Gear VR and Oculus Go. You can play SteamVR games in your standalone headset.

English [Japanese](https://github.com/polygraphene/ALVR/blob/master/README-ja.md)

## Description
ALVR streams VR display output from your PC to Gear VR / Oculus Go via Wi-Fi. This is similar to Riftcat or Trinus VR, but our purpose is optimization for Gear VR. ALVR provides smooth head-tracking compared to other apps in Wi-Fi environment using Asynchronous Timewarp.

## Requirements
ALVR requires following devices:
- Gear VR or Oculus Go
    - Only tested on Gear VR with Galaxy S8
    - If you tried on other devices, please feedback!
- High-end gaming PC with NVIDIA GPU which supports NVENC
    - Only Windows 10 is supported
- 802.11n/ac Wireless connection
- SteamVR

## Installation
- Install ALVR server for PC
    - Install SteamVR
    - Download zip from [Releases](https://github.com/polygraphene/ALVR/releases)
    - Extract zip on any folder
    - Execute driver\_install.bat in driver folder
    - Launch ALVR.exe
- Install ALVR client for headset
    - For Gear VR users
        - (Install apk from SideloadVR) Yet to be released. Please wait.
        - Get osig file from oculus website.
        - Put osig file on assets folder in apk
        - Run zipalign and jarsigner for apk
    - For Oculus Go users
        - Download apk from [Releases](https://github.com/polygraphene/ALVR/releases)
        - Install apk via adb

## Usage
- Install SteamVR
- Launch ALVR.exe
- Press "Start Server" button or launch VR game
- SteamVR's Small window will appears
- Launch ALVR Client in your headset
- IP Address of headset will appears in server tab of ALVR.exe
- Press "Connect" button

## Uninstallation
- Execute driver\_uninstall.bat in driver folder
- Delete install folder (ALVR does not use registry)
- If you already deleted folder without executing driver\_uninstall.bat
    - Open C:\Users\\%USERNAME%\AppData\Local\openvr\openvrpaths.vrpath and check install directory.
    - Execute
    `"C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrpathreg.exe" removedriver (install folder)`
    in Command Prompt.

## Future work
- Support the functinalty to change video bitrate
- Support streaming sound
- Support H.265 hevc encoding (Currently H.264 only)
- Support Gear VR / Oculus Go Controller
- Easy installer

## Build
### ALVR Server and GUI (Launcher)
- Open ALVR.sln with Visual Studio 2017 and build.
    - alvr\_server project is driver for SteamVR written in C++
    - ALVR project is launcher GUI written in C#

### ALVR Client
- Clone [ALVR Client](https://github.com/polygraphene/ALVRClient)
- Put your [osig file](https://developer.oculus.com/documentation/mobilesdk/latest/concepts/mobile-submission-sig-file/) on assets folder (only for Gear VR)
- Build with Android Studio
- Install apk via adb

## License
ALVR is licensed under MIT License.

## Donate
If you like this project, please donate!
