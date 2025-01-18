## (! Mandatory, apply fix if not applied yet !) Black screen even when SteamVR shows movement, Dashboard not detecting launched ALVR/SteamVR

The steam runtimes SteamVR runs in break the alvr driver loaded by SteamVR.
This causes the screen to stay black on the headset or an error to be reported that the pipewire device is missing or can even result in SteamVR crashing.

### Fix

Add `~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` to the commandline options of SteamVR (SteamVR -> Manage/Right Click -> Properties -> General -> Launch Options).

This path might differ based on your Steam installation, in that case SteamVR will not start at all. If this is the case you can figure out the actual path by going to Steam Settings -> Storage.
Then pick the storage location with the star emoji (⭐) and take the path directly above the usage statistics. Prepend this path to `steamapps/common/SteamVR/bin/vrmonitor.sh`.
Finally put this entire path into the SteamVR commandline options instead of the other one.

### Hyprland/Sway/Wlroots Qt fix

If you're on hyprland, sway, or other wlroots-based wayland compositor, you might have to prepend `QT_QPA_PLATFORM=xcb` before commandline, which results in full commandline for steamvr being something like this:
`QT_QPA_PLATFORM=xcb ~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%`.

Related issue:
[[BUG] No SteamVR UI on wlroots-based wayland compositors (sway, hyprland, ...) with workaround](https://github.com/ValveSoftware/SteamVR-for-Linux/issues/637).


## The alvr driver doesn't get detected by SteamVR (even after vrmonitor fix)

Could be related to Arch AUR package (either installed not for nvidia on nvidia based system (`alvr-nvidia`), or just in general).

### Fix

Try using a launcher or portable .tar.gz release from the Releases page.

## Artifacting, no SteamVR Overlay or graphical glitches in streaming view

Could be related to AMD amdvlk or amdgpu-pro driver being present on your system.

If you have Amdvlk installed on your system, it overrides other vulkan drivers and causes SteamVR to break. Use the `vulkan-radeon` driver (aka radv) instead.

### Fix

Check if amdvlk or amdgpu-pro are installed by seeing if `ls /usr/share/vulkan/icd.d/ | grep -e amd_icd -e amd_pro` shows anything.
If so, uninstall amdvlk and/or the amdgpu-pro drivers from your system. (This method may not catch all installations due to distro variations)

On arch, first install `vulkan-radeon` and uninstall other drivers.

## Failed to create VAAPI encoder

Blocky or crashing streams of gameplay and then an error window on your desktop saying:
> Failed to create VAAPI encoder: Cannot open video encoder codec: Function not implemented. Please make sure you have installed VAAPI runtime.

### Fix

For fedora:
 * Switch from `mesa-va-drivers` to `mesa-va-drivers-freeworld`. [Guide on how to do so](https://fostips.com/hardware-acceleration-video-fedora/) or [the RPM docs](https://rpmfusion.org/Howto/Multimedia)
For arch (don't use vaapi for nvidia):
 * Follow through [this](https://wiki.archlinux.org/title/Hardware_video_acceleration#Installation) page
Then reboot your machine.

For other distros (e.g. Manjaro):
 * Install the nonfree version of the mesa/vaapi drivers that include the proprietary codecs needed for h264/hevc encoding

## Nvidia driver version requirements

Alvr requires at least driver version 535 and CUDA version 12.1. If this is not the case SteamVR or the encoder might not work.

### Fix

Install at least the required versions of the driver and ensure you have CUDA installed with at least version 12.1.

If an error saying CUDA was not detected persists, try using the latest alvr nightly release.

## Using ALVR with only integrated graphics

Beware that using **only** integrated graphics for running ALVR is highly inadvisable as in most cases it will lead to very poor performance (even on more powerful devices like Steam Deck, it's still very slow).
Don't expect things to work perfectly in this case too, as some older integrated graphics simply might not have the best vulkan support and might fail to work at all. 

## Hybrid graphics advices

### General advise

If you have PC and can disable your integrated gpu from BIOS/UEFI, it's highly advised to do so to avoid multiple problems of handling hybrid graphics.
If you're on laptop and it doesn't allow disabling integrated graphics (in most cases) you have to resort to methods bellow.

### Amd/Intel integrated gpu + Amd/Intel discrete gpu

Put `DRI_PRIME=1 ~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` (adjust vrmonitor path to your distro) into SteamVR's commandline options and in those of all VR games you intend to play with ALVR.

### Amd/Intel integrated gpu + Nvidia discrete gpu

Put `__NV_PRIME_RENDER_OFFLOAD=1 __VK_LAYER_NV_optimus=NVIDIA_only __GLX_VENDOR_LIBRARY_NAME=nvidia ~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` (adjust vrmonitor path to your distro) into SteamVR's commandline options and in those of all VR games you intend to play with ALVR.

### SteamVR Dashboard not rendering in VR on Nvidia discrete GPU
If you encounter issues with the SteamVR dashboard not rendering in VR you may need to run the entire steam client itself via PRIME render offload. First close the steam client completey if you have it open already, you can do so by clicking the Steam dropdown in the top left and choosing exit. Then from a terminal run: `__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia steam-runtime`

## Wayland

When using old Gnome (< 47 version) under Wayland you might need to put `WAYLAND_DISPLAY='' ~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` (adjust vrmonitor path to your distro) into the SteamVR commandline options to force XWayland on SteamVR. This fixes issue with drm leasing not being available.

## The view shakes

SlimeVR related, might be fixed in future updates of ALVR

### Fix

Start the SlimeVR Server only after you connected and got an image to alvr at least once.

## 109 Error

The 109 error or others appear.

### Fix

Start Steam first before starting SteamVR through alvr. If SteamVR is already started, restart it.

## No audio or microphone

Even though audio or microphone are enabled in presets, still can't hear audio or no one can hear me

### Fix

Make sure you select `ALVR Audio` and `ALVR Microphone` in device list as default **after** connecting headset. As soon as headset disconnected, devices will be removed. If you set it as default, they will be automatically chosen whenever they show up and you don't need to do it manually ever again.
If you don't appear to have audio devices, or have pipewire errors in logs, check if you have `pipewire` installed and it's at least version `0.3.49` by using command `pipewire --version`
For older (<=22.04 or debian <=11) ubuntu or debian based distributions you can check [pipewire-upstream](https://github.com/pipewire-debian/pipewire-debian) page for installing newer pipewire version

## Low AMDGPU performance and shutters

This might be caused by [[PERF] Subpar GPU performance due to wrong power profile mode · Issue #469 · ValveSoftware/SteamVR-for-Linux · GitHub](https://github.com/ValveSoftware/SteamVR-for-Linux/issues/469).

### Fix

Using CoreCtrl is highly advised (install it using your distribution package management) and in settings set your GPU to VR profile, as well as cpu to performance profile (if it's old Ryzen cpu).

## OVR Advanced Settings

Disable the OVR Advanced Settings driver and don't use it with ALVR.
It's incompatible and will produce ladder-like latency graph with very bad shifting vision.


## Bindings not working/high cpu usage due to bindings ui

Steamvr can't properly update bindings, open menus, and possibly eats too much cpu.

This issue is caused by SteamVR's webserver spamming requests that stall the chromium ui and causes it to use a lot of cpu.

### Fix

Apply the following patch: `https://github.com/alvr-org/ALVR-Distrobox-Linux-Guide/blob/main/patch_bindings_spam.sh`
Assuming default path for Arch, Fedora - one-liner: `curl -s https://raw.githubusercontent.com/alvr-org/ALVR-Distrobox-Linux-Guide/main/patch_bindings_spam.sh | sh -s ~/.steam/steam/steamapps/common/SteamVR`
