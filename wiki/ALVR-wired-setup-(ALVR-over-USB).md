## ALVR native wired mode support
As of v20.12 ALVR supports wired connections directly through the dashboard.
Just enable the "Wired Connection" toggle on the Devices screen, plug in your headset
and accept the "Allow USB debugging?" popup displayed by the headset.

Note that your headset will need to have Developer Mode and USB Debugging enabled to use this feature.

For Quest headsets see [here](https://developers.meta.com/horizon/documentation/native/android/mobile-device-setup/) for instructions.
The last step about installing ADB should be skipped, as ALVR downloads a copy of ADB on it's own and uses that.

If you have successfully followed all those steps and it still isn't connecting,
ensure that the setting "Connection -> Wired Client Type" matches where you installed the client from (for the launcher also use the "Github" option).

## The DEPRECATED (and clunky) way:
The following sections list the old and deprecated way to get a wired connection and is only kept as reference.

This has exactly the same requirements as the native wired mode, but requires additional software and is more complex to setup, so native mode should be preferred.

## ALVR Streamer (PC) Configuration

* **Switch the connection streaming protocol to TCP** in Settings > Connection.
* If your headset is detected, click "Trust." Click "Edit", "Add new" and change the IP address to `127.0.0.1`.
* If your headset is not detected, click "Add device manually" and use the IP address `127.0.0.1`. Use the hostname displayed on your headset screen.

## Letting your PC communicate with your HMD

The Quest, Pico HMDs are Android devices, therefore, we can use [Android Device Bridge](https://developer.android.com/studio/command-line/adb) commands to tell the HMDs to look for data over USB, as well as Wi-Fi, using port forwarding.

You can accomplish this with some pre-made applications/scripts (just below), or run the commands manually with [SideQuest](https://sidequestvr.com/setup-howto)

If you haven't already, connect a USB cable from your PC to your headset. USB 2.0 will work fine but 3.0 and higher is best.

**Make sure to enable dev account and authorize the computer in your headset if you're on quest or enable USB Debug on Pico in settings.**

### Option 1 - Dedicated ADB Applications

The following programs serve to wrap and simplify the process of doing manual ADB commands, the first two will also automatically reconnect the headset if the USB connection is interrupted.

* [**ADBForwarder (Recommended)**](https://github.com/alvr-org/ADBForwarder)
  
  * Easy to use
  * Downloads ADB for you
  * Cross-platform (Windows & Linux)

* [**Python Script**](https://gist.github.com/Bad-At-Usernames/684784f42cbb69e22688a21173ec263d)
  
  * Lightweight and simple
  * Requires [Python 3](https://www.python.org/downloads/) and [PyWin32](https://pypi.org/project/pywin32/)
  * Requires [ADB Platform Tools](https://developer.android.com/studio/releases/platform-tools) to be in the same directory as `main.py`
    * Just extract `platform-tools` to your desktop and place `main.py` in that folder, should work when you run the script

* [**Batch Script**](https://gist.github.com/AtlasTheProto/1f03c3aeac70c4af5b4f2fcd9b9273c0)
  
  * Requires [ADB Platform Tools](https://developer.android.com/studio/releases/platform-tools), edit the path in line 2 to point to the directory where you extracted `platform-tools`
  * Needs to be run every time you (re)connect your headset

### Option 2 - [SideQuest](https://sidequestvr.com/setup-howto)

* Ensure SideQuest is running, and the headset has authorized the USB connection to the PC
* Open the 'Run ADB Commands' menu in SideQuest (top-right, box with an arrow inside it)
* Click 'Custom Command' and run these adb commands:
  * `adb forward tcp:9943 tcp:9943`
  * `adb forward tcp:9944 tcp:9944`
  * These commands will need to be run every time you (re)connect your headset.
* Keep SideQuest opened until you want to close the connection.

***

Once you are finished, the headset should now establish a connection over USB.
