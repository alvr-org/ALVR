## Installing ALVR and using SteamVR on Linux through Distrobox

## Disclaimer

1. This is just an attempt to make things easier for Linux users to use ALVR, SteamVR. By no means it's a comprehensive, fully featured and 100% working guide. Please open new issues and pull requests to correct this guide, scripts, etc etc.

2. This guide is not adjusted for Intel gpu owners yet. Only NVIDIA and AMDGPU (Low priority TODO).

3. Slimevr, OpenVRAS, Open Space Calibrator are all possible to launch and use on Linux, but this guide/script is not adjusted yet (Medium priority TODO).

4. Firewall configuration is skipped entirely and setup firewall configuration is broken, so it might not work in case you have strict firewall (alvr docs should have info about that) (Low priority TODO).

5. This script unlikely to work on external disks.

## Installing alvr distrobox

For installing you only really need couple of dependencies on host:

1. `wget` + `curl` (to download podman/distrobox/alvr/etc)
2. `xhost` (on X11 to allow rootless podman to work with graphical applications)
3. `sed` (for removing color in logs)
4. `pipewire` for fully automatic microphone, `pulseaudio` for basic audio support (automatic microphone is unsupported with it)
5. For nvidia - `CUDA` (distrobox passes through it and driver as well into the container and CUDA contains NVENC encoder for streaming)

After you have installed required dependencies for your installation from above, open terminal in this repository folder and do:

1. `./setup.sh`
   
   That's it. **Follow all green and especially red text carefully from the scripts.**
   
   In case if have errors during installation, please report the full log as-is (remove private info if you happen to have some) as an Issue.
   
   After full installation, you can use `./start-alvr.sh` to launch alvr automatically.
   
   Script also downloads related apk file to install to headset into `installation` folder for you. Use Sidequest or ADB to install it.

## Post-install ALVR & SteamVR Configuration

After installing ALVR you may want to configure it and steamvr to run at best quality for your given hardware/gpu. Open up ALVR using `./start-alvr.sh` script and do the following (each field with input value needs enter to confirm):

### Common configuration:

1. **Resolution:** If you have 6600 XT level GPU you can select Low, and in case you don't mind lower FPS - Medium

2. **Preferred framerate:** If you know that you will have lower fps than usual (for instance, VRChat), run at lower fps. This is because when reprojection (this is what allows for smooth view despite being at low fps) goes lower than twice the amount of specified framerate - it fails to reproject and will look worse. So for example, you can run at 72hz if you know you're expecting low framerate, and 120hz if you are going to play something like Beat Saber, which is unlikely to run at low fps.

3. **Encoder preset:** Speed

4. **Bitrate:** Constant, bitrate: 350-450 mbps for h264 wireless/700 mbit-1 gbit cabled, 100-150 mbps for HEVC (tested on Pico 4 ).

5. **Foveated rendering:** This highly depends on given headset, but generally default settings should be OK for Quest 2. For **pico neo 3** i would recommend setting center region width to 0.8 and height to 0.75, shifts to 0 and edge ratios can be set at 6-7, and for the same **pico neo 3** disable oculus foveation level and dynamic oculus foveation.

6. **Color correction:** Set sharpening to 1and if you like oversaturated image, bump saturation to 0.6.

7. For **pico neo 3** left controller offsets (from top to bottom): Position -0.06, -0.03, -0.1; Rotation: 0, 3, 17.

8. **Connection -> Stream Protocol:** TCP. This ensures that there would be no heavy artifacts if packet loss happens (until it's too severe), only slowdowns.

### AMD-specific configuration:

1. Preferred codec: HEVC, h264 works too.

2. Reduce color banding: turn on, might make image smoother.

### Nvidia-specific configuration (needs feedback):

1. Preferred codec: h264, HEVC works too

After that, restart your headset using power button and it will automatically restart steamvr once, applying all changes.

### SteamVR configuration:

Inside SteamVR you also may need to change settings to improve experience. Open settings by clicking on triple stripe on SteamVR window and expand Advanced Settings (Hide -> Show)

1. **Disable SteamVR Home.** It can be laggy, crashes often and generally not working nice on linux, so it is recommend disabling it altogether.

2. **Render Resolution:** - Custom and keep it at 100%. This is to ensure that SteamVR won't try to supersample resolution given by ALVR

3. **Video tab: Fade To Grid** on app hang - this will lock your view to last frame when app hangs instead of dropping you into steamvr void, completely optional but you may prefer that.

4. **Video tab: Disable Advanced Supersample Filtering**

5. **Video tab: Per-application video settings** - Use Legacy Reprojection Mode for specific game. This can drastically change experience from being very uncomfortable, rubber-banding, to straight up perfect. This essentially disables reprojection on SteamVR side and leaves it to the client. Make sure to enable it for each game you will play.

6. **Developer tab: Set steamvr as openxr runtime** - this ensures that games using openxr (such as Bonelab, or Beat Saber) will use SteamVR.

### Distrobox note:

* Do note that `sudo` inside container doesn't have privliges to do anything as `root`, but that container has almost exactly the same rights as regular user, so deleting user files from that container **is** possible.

* You can add your steam library from outside the container after alvr installation as for container, `/home/user` folder is the same as on your host, so you can add it from inside distrobox steam.

* Do note though, there has been mentioned some issues with mounted devices, symlinks and containers, so in case you have them, please report them to discover if it's the case.

## Updating ALVR & WlxOverlay

In case there was an update for ALVR or WlxOverlay in the repository, you can run `./update-vr-apps.sh` with or without prefix. In case you want to manually update ALVR or WlxOverlay versions, you can change `links.sh` file accordingly and run the same script.

## Uninstalling

To uninstall this, simply run `./uninstall.sh` and it will automatically remove everything related to locally installed distrobox, it's containers, podman and everything inside in `installation` or prefixed folder.

## Additional info

Highly recommend using CoreCtrl (install it using your distribution package management) and setting settings to VR profile for **AMD** gpus, as well as cpu to performance profile (if it's a Ryzen cpu). Without setting those gpu profiles, it's highly likely you will have serious shutters/wobbles/possibly crashes (sway users) at random point while playing ([[PERF] Subpar GPU performance due to wrong power profile mode · Issue #469 · ValveSoftware/SteamVR-for-Linux · GitHub](https://github.com/ValveSoftware/SteamVR-for-Linux/issues/469)).
