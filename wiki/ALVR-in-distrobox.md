## Installing ALVR and using SteamVR on Linux through Distrobox

## Disclaimer

1. This is just an attempt to make things easier for Linux users to use ALVR, SteamVR. By no means it's a comprehensive, fully featured and 100% working guide. Please open new issues and pull requests to correct this guide, scripts, etc etc.

2. This guide is not adjusted for Intel gpu owners yet. Only NVIDIA and AMDGPU (Low priority TODO).

3. Slimevr, OpenVRAS, Open Space Calibrator are all possible to launch and use on Linux, but this guide/script is not adjusted yet (Medium priority TODO).

4. Firewall configuration is skipped entirely and setup firewall configuration is broken, so it might not work in case you have strict firewall (alvr docs should have info about that) (Low priority TODO).

5. Don't use SteamVR Beta (1.26+) version, it contains a lot of breaking changes that are yet to be tested to work with.

6. Some nvidia users might experience 307 steamvr crash - seemingly related to 530 driver, update or downgrade, should resolve the issue.

7. At the moment, most of the podman portability issues were fixed, but it's not done, so if you happen to have any kind of issues while running both this and other distroboxes from your system, please report them. I created some issues at podman side ([documentation issue](https://github.com/containers/podman/issues/18375), [storage configuration issue, which prevents complete isolation from runtime containers at the moment](https://github.com/containers/storage/issues/1587)) and [created PR](https://github.com/89luca89/distrobox/pull/718) for distrobox (WIP) to upstream the changes.

8. This script unlikely to work on external disks.

## Installing alvr distrobox

For installing you only really need couple of dependencies on host:

1. `wget` + `curl` (to download podman/distrobox/alvr/etc)
2. `xhost` (on X11 to allow rootless podman to work with graphical applications)
3. `sed` (for removing color in logs)
4. `pipewire` for fully automatic microphone, `pulseaudio` for basic audio support (automatic microphone is unsupported with it)
5. For nvidia - `CUDA` (distrobox passes through it and driver as well into the container and CUDA contains NVENC encoder for streaming)
6. If you don't have `newgidmap` command available, you need to install `shadow-utils`. (packages ubuntu-like: `newuidmap`, arch-like: `shadow`, mint: `uidmap`)

After you have installed required dependencies for your installation from above, open terminal in this repository folder and do:

1. `./setup.sh`
   
   That's it. **Follow all green and especially red text carefully from the scripts.**
   
   In case if have errors during installation, please report the full log as-is (remove private info if you happen to have some) as an Issue.
   
   But if you are just reinstalling it, please run `./uninstall.sh` first before trying to install again.
   
   After full installation, you can use `./start-alvr.sh` to launch alvr automatically.
   
   Script also downloads related apk file to install to headset into `installation` folder for you. Use Sidequest or ADB to install it.
   
   **Experimental:** Prefixed installation is now available, which allows you to specify where to install relative to this folder. Use --prefix and --container-name to specify folder name and container name (you should specify both for this to work)

## Post-install ALVR & SteamVR Configuration

After installing ALVR you may want to configure it and steamvr to run at best quality for your given hardware/gpu. Open up ALVR using `./start-alvr.sh` script and do the following (each field with input value needs enter to confirm):

### Common configuration:

1. **Resolution:** If you have 6600 XT level GPU you can select Low, and in case you don't mind lower FPS - Medium

2. **Preferred framerate:** If you know that you will have lower fps than usual (for instance, VRChat), run at lower fps. This is because when reprojection (this is what allows for smooth view despite being at low fps) goes lower than twice the amount of specified framerate - it fails to reproject and will look worse. So for example, you can run at 72hz if you know you're expecting low framerate, and 120hz if you are going to play something like Beat Saber, which is unlikely to run at low fps.

3. **Encoder preset:** Quality

4. **Game Audio & Microphone:** Set to pipewire as has been said in installation. ALVR uses default audio sink for audio, and doesn't care about volume (it's controlled only onboard audio on headset), and automatic audio scripts mutes default audio by default to prevent mirroring. As for microphone, every time ALVR is connected on headset, on connect script creates audio source named **ALVR-MIC-Source**, which can be used in game as default microphone, if you set it at least once. No need to un-set it back to default. And when headset disconnects it automatically clears audio source and restores back to previous microphone.

5. **Bitrate:** Adaptive, maximum bitrate: 150 mbps, minimum bitrate: 100 mbps.

6. **Foveated rendering:** This highly depends on given headset, but generally default settings should be OK for Quest 2. For **pico neo 3** i would recommend setting center region width to 0.8 and height to 0.75, shifts to 0 and edge ratios can be set at 6-7, and for the same **pico neo 3** disable oculus foveation level and dynamic oculus foveation.

7. **Color correction:** Set sharpening to 1and if you like oversaturated image, bump saturation to 0.6.

8. For **pico neo 3** left controller offsets (from top to bottom): Position -0.06, -0.03, -0.1; Rotation: 0, 3, 17.

9. **Connection -> Stream Protocol:** TCP. This ensures that there would be no heavy artifacts if packet loss happens (until it's too severe), only slowdowns.

10. **Linux async reprojection:** keep it off, it's not needed anymore and client does reprojection better

### AMD-specific configuration:

1. Preferred codec: HEVC, h264 looked choppy and has visual "blocking".

2. Reduce color banding: turn on, makes image smoother.

### Nvidia-specific configuration (needs feedback):

1. Preferred codec: h264

After that, restart your headset using power button and it will automatically restart steamvr once, applying all changes.

### SteamVR configuration:

Inside SteamVR you also may need to change settings to improve experience. Open settings by clicking on triple stripe on SteamVR window and expand Advanced Settings (Hide -> Show)

1. **Disable SteamVR Home.** It can be laggy, crashes often and generally not working nice on linux, so it is recommend disabling it altogether.

2. **Render Resolution:** - Custom and keep it at 100%. This is to ensure that SteamVR won't try to supersample resolution given by ALVR

3. **Video tab: Fade To Grid** on app hang - this will lock your view to last frame when app hangs instead of dropping you into steamvr void, completely optional but you may prefer that.

4. **Video tab: Disable Advanced Supersample Filtering** 

5. **Video tab: Per-application video settings** - Use Legacy Reprojection Mode for specific game. This can drastically change experience from being very uncomfortable, rubber-banding, to straight up perfect. This essentially disables reprojection on SteamVR side and leaves it to the client. Make sure to enable it for each game you will play. If you don't see that button, it is possible that  you didn't apply patch from installation script, which means that opening each game video settings will take a while and may not even catch up at all after multiple minutes. If you want to apply that patch, get into the container (read bellow in the end how to do that) and run the `./patch_bindings_spam.sh` with absolute path to your steamvr inside container directory (relative to scripts folder, `installation/arch-alvr/.local/share/Steam/steamapps/SteamVR`).

6. **Developer tab: Set steamvr as openxr runtime** - this ensures that games using openxr (such as Bonelab, or Beat Saber) will use SteamVR.

### Distrobox note:

* To enter container (for updating archlinux, installing additional software, and other reasons) you need to do the following:
1. `source ./setup-env.sh`

2. `distrobox-enter arch-alvr`

        This enters the container. Do note that `sudo` inside container doesn't have privliges to do anything as `root`, but that container has almost exactly the same rights as regular user, so deleting user files from that container **is** possible.

* You can add your steam library from outside the container after alvr installation as for container, `/home/user` folder is the same as on your host, so you can add it from inside distrobox steam.

* Do note though, there has been mentioned some issues with mounted devices, symlinks and containers, so in case you have them, please report them to discover if it's the case.

## Updating ALVR & WlxOverlay

In case there was an update for ALVR or WlxOverlay in the repository, you can run `./update-vr-apps.sh` with or without prefix. In case you want to manually update ALVR or WlxOverlay versions, you can change `links.sh` file accordingly and run the same script.

## Uninstalling

To uninstall this, simply run `./uninstall.sh` and it will automatically remove everything related to locally installed distrobox, it's containers, podman and everything inside in `installation` or prefixed folder.

## Additional info

Highly recommend using CoreCtrl (install it using your distribution package management) and setting settings to VR profile for **AMD** gpus, as well as cpu to performance profile (if it's a Ryzen cpu). Without setting those gpu profiles, it's highly likely you will have serious shutters/wobbles/possibly crashes (sway users) at random point while playing ([[PERF] Subpar GPU performance due to wrong power profile mode · Issue #469 · ValveSoftware/SteamVR-for-Linux · GitHub](https://github.com/ValveSoftware/SteamVR-for-Linux/issues/469)).
