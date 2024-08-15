## Black screen even when SteamVR shows movement

The steam runtimes SteamVR runs in break the alvr driver loaded by SteamVR.
This causes the screen to stay black on the headset or an error to be reported that the pipewire device is missing or can even result in SteamVR crashing.

### Fix

Add `~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` to the commandline options of SteamVR (SteamVR -> Manage/Right Click -> Properties -> General -> Launch Options).

This path might differ based on your Steam installation, in that case SteamVR will not start at all. If this is the case you can figure out the actual path by going to Steam Settings -> Storage.
Then pick the storage location with the star emoji (⭐) and take the path directly above the usage statistics. Prepend this path to `steamapps/common/SteamVR/bin/vrmonitor.sh`.
Finally put this entire path into the SteamVR commandline options instead of the other one.

### Hyprland/Sway/Wlroots Fix

If you're on hyprland, sway, or other wlroots-based wayland compositor, you might have to prepend `QT_QPA_PLATFORM=xcb` before commandline, which results in full commandline for steamvr being something like this:
`QT_QPA_PLATFORM=xcb ~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%`.

Related issue:
[[BUG] No SteamVR UI on wlroots-based wayland compositors (sway, hyprland, ...) with workaround](https://github.com/ValveSoftware/SteamVR-for-Linux/issues/637).

## Artifacting, no SteamVR Overlay or graphical glitches in streaming view

Could be related to AMD amdvlk driver being present on your system.

If you have Amdvlk installed on your system, it overrides other vulkan drivers and causes SteamVR to break. Use the `vulkan-radeon` driver (aka radv) instead.

### Fix

Check if Amdvlk is installed by seeing if `ls /usr/share/vulkan/icd.d/ | grep amd_icd` shows anything. If so, uninstall Amdvlk from your system.

## Failed to create VAAPI encoder (fedora)

Blocky or crashing streams of gameplay and then an error window on your desktop saying:
> Failed to create VAAPI encoder: Cannot open video encoder codec: Function not implemented. Please make sure you have installed VAAPI runtime.

This seems to be an issue for AMD GPU fedora 39+ users, but maybe others.

### Fix

Switch from `mesa-va-drivers` to `mesa-va-drivers-freeworld`. [Guide on how to do so](https://fostips.com/hardware-acceleration-video-fedora/) or [the RPM docs](https://rpmfusion.org/Howto/Multimedia). Then reboot your machine.

## Nvidia driver version requirements

Alvr requires at least driver version 535 and CUDA version 12.1. If this is not the case SteamVR or the encoder might not work.

### Fix

Install at least the required versions of the driver and ensure you have CUDA installed with at least version 12.1.

If an error saying CUDA was not detected persists, try using the latest alvr nightly release.

## Hybrid graphics advices

### General advise

If you have PC and can disable your integrated gpu from BIOS/UEFI, it's highly advised to do so to avoid multiple problems of handling hybrid graphics.
If you're on laptop and it doesn't allow disabling integrated graphics (in most cases) you have to resort to methods bellow.

### Amd/Intel integrated gpu + Amd/Intel discrete gpu

Put `DRI_PRIME=1 %command%` into SteamVR's commandline options and in those of all VR games you intend to play with ALVR.

### Amd/Intel integrated gpu + Nvidia discrete gpu

Put `__NV_PRIME_RENDER_OFFLOAD=1 __VK_LAYER_NV_optimus=NVIDIA_only __GLX_VENDOR_LIBRARY_NAME=nvidia %command%` into SteamVR's commandline options and in those of all VR games you intend to play with ALVR.

## Wayland

When using hyprland or Gnome Wayland you need to put `WAYLAND_DISPLAY='' %command%` into the SteamVR commandline options to force XWayland.

## The view shakes

SlimeVR related, will be fixed in future updates of ALVR

### Fix

Start the SlimeVR Server only after you connected and got an image to alvr at least once.

## 109 Error

The 109 error or others appear.

### Fix

Start Steam first before starting SteamVR through alvr. If SteamVR is already started, restart it.

## No audio or microphone

Even though audio or microphone are enabled in presets, neither seems to appear in devices list

### Fix

Check if you have `pipewire` installed and it's at least version `0.3.49` by using command `pipewire --version`
For older (<=22.04 or debian <=11) ubuntu or debian based distributions you can check [pipewire-upstream](https://github.com/pipewire-debian/pipewire-debian) page for installing newer pipewire version

## The alvr driver doesn't get detected by SteamVR

Could be related to Arch AUR package.

### Fix

Try using a launcher or portable .tar.gz release from the Releases page.

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
