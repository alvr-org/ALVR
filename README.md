# Virtual Display
An example OpenVR driver for demonstrating the IVRVirtualDisplay interface:
https://github.com/ValveSoftware/openvr/blob/master/headers/openvr_driver.h#L2372

The IVRVirtualDisplay interface is provided to allow OpenVR driver authors access to the final composited backbuffer intended for the headset’s display. The primary expected use case is for wireless transport, though this could also be used for saving output to disk or streaming video. From the perspective of the runtime, the VR compositor is interfacing with a _virtual_ rather than an _actual_ display. 

## Prerequisites
Both Steam and SteamVR must be installed and working properly.  Steam can be downloaded here: http://store.steampowered.com/
This sample relies on a second video card installed in the machine to act as the "remote display".  An HTC Vive headset should be connected to this secondary graphics adapter.  See the included PDF for more details.

## Supported platforms
This example is currently Windows-only.

## Installation
Two bat files are included in the root for registering the current directory with SteamVR.  This must be done for the SteamVR runtime to find the driver.
These files have hardcoded paths to the default Steam instalation location which you may need to edit by hand if you've installed Steam in a different location.

## Building
The provided solution and project files were built using Visual Studio 2013.  You will need to build both the Win32 and x64 project configurations in either Release or Debug.

## Running
Once built, ensure your headset is plugged into the secondary graphics card and that it is in Extended Mode (i.e. shows up as part of th Windows desktop).  Then launch SteamVR.

## Troubleshooting
Check Steam/logs/vrserver.txt or step through the code.  The parameters used to launch virtual_display.exe are printed to the log, which can be helpful for launching from the debugger.  For debugging drivers, you can connect to Steam and then use the Child Process Debugging Addon from Microsoft.

