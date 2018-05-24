# ALVR - Air Light VR
====

ALVR is a opensource remote VR display for Gear VR and Oculus Go. You can play SteamVR games in your standalone headset.

## Description
ALVR streams VR display output from your PC to Gear VR / Oculus Go via Wi-Fi. This is similar to Riftcat or Trinus VR, but our purpose is optimization for Gear VR. We achieved smooth head-tracking compared to Riftcat in Wi-Fi environment.

## Requirements
ALVR requires following devices:
- Gear VR or Oculus Go
 - Currently, only tested on Gear VR with Galaxy S8
 - If you tried on other devices, please feedback!
- High-end gaming PC with NVIDIA GPU which supports NVENC
 - Currently, only Windows 10 is supported
- 802.11n/ac Wireless connection

## Installation
- Install ALVR server for PC
 - Download zip from release page
 - Extract zip on any folder
 - Launch driver\_install.bat
 - Launch ALVR.exe
- Install ALVR client for headset
 - For Gear VR users
  - Install apk from SideloadVR
 - For Oculus Go users
  - Download apk from release page
  - Install apk via adb

## Future work
- Support streaming sound
- Support H.265 hevc encoding (Currently H.264 only)
- Support Gear VR / Oculus Go Controller
- Easy installer

## Build
### ALVR Server and GUI(Launcher)
- Open ALVR.sln with Visual Studio 2017 and build.
 - alvr\_server project is driver for SteamVR written in C++
 - ALVR project is launcher GUI written in C#

### ALVR Client
- Clone [ALVR Client](https://polygraphene.github.com/ALVRClient/)
- Put your [osig file](https://developer.oculus.com/documentation/mobilesdk/latest/concepts/mobile-submission-sig-file/) on assets folder (only for Gear VR)
- Build with Android Studio
- Install apk via adb

## License
ALVR is licensed under MIT License.

## Donate
If you like this project, please donate!
