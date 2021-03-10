# Work in progress linux port

## Current limitations
- audio streaming is not working
- foveated encoding is not implemented
- requires superuser access for setup
- mostly untested
- requires a free port on the graphic card
- TCP streaming seems not to be working
- vrserver does not quit properly and needs to be terminated (kill -9)

## Setup instructions
Build instructions:
```bash
#Build the main project
cargo xtask build-server --release
#Build the helper binaries
meson setup tools alvr/server/cpp/tools/ --buildtype release
meson compile -C tools
```

**WARNING the following lines need to be customized for your setup**
```bash
#generate display information (EDID) for your virtual headset
#for a Quest 1 and default rendering settings, it will be 2112x1184 72 Hz
tools/edid-gen <width> <height> <refresh rate> > edid.data
```

The following lines need to be ran as root, and 0/DP-1 must be replaced by the index of the GPU you want to use, and a display that is not currently in use
```bash
#force an output on the graphic card
cat edid.data > /sys/kernel/debug/dri/0/DP-1/edid_override
echo digital > /sys/kernel/debug/dri/0/DP-1/force
echo 1 > /sys/kernel/debug/dri/0/DP-1/trigger_hotplug
```

Install the grabber binary and add the `cap_sys_admin` capability.
This assumes that /usr/local/bin is in your PATH, it is currently required that the grabber binary is found via PATH.
**WARNING this gives the grabber binary root-like permissions, and it uses ffmpeg, which may have security vulnerabilities**
```bash
cp tools/grabber /usr/local/bin
setcap cap_sys_admin+pe /usr/local/bin/grabber
```

## Usage
Run `build/alvr_server_linux/ALVR Launcher`
On first setup, SteamVR will probably show the VR display on your screen, with the configuration window. If you have dual screen, you can move the configuration window to a visible area (with Alt + drag on most desktop environments).
In the setup, deactivate audio streaming, switch connection to UDP, and deactivate foveated encoding.
On the headset, launch the application, then click trust on the configuration window, which will quit.
The headset says that the server will restart, but it will not. You must relaunch it manually.

If you are here, once it is all restarted, you should be able to get the stream on the headset.
