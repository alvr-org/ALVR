# Linux Troubleshooting

## SteamVR

The steam runtimes SteamVR runs in break the alvr driver loaded by SteamVR.
This causes the screen to stay black on the headset or an error to be reported that the pipewire device is missing or can even result in SteamVR crashing.

### Fix

Add `~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` to the commandline options of SteamVR (SteamVR -> Manage/Right Click -> Properties -> General -> Launch Options).

This path might differ based on your Steam installation, in that case SteamVR will not start at all. If this is the case you can figure out the actual path by going to Steam Settings -> Storage.
Then pick the storage location with the star emoji (⭐) and take the path directly above the usage statistics. Prepend this path to `steamapps/common/SteamVR/bin/vrmonitor.sh`.
Finally put this entire path into the SteamVR commandline options instead of the other one.

## Amdvlk/AMD

If you have Amdvlk installed on your system, it overrides other vulkan drivers and causes SteamVR to break. Use the `vulkan-radeon` driver (aka radv) instead.

### Fix

Check if Amdvlk is installed by seeing if `ls /usr/share/vulkan/icd.d/ | grep amd_icd` shows anything. If so, uninstall Amdvlk from your system.

## Failed to create VAAPI encoder

Blocky or crashing streams of gameplay and then an error window on your desktop saying:
> Failed to create VAAPI encoder: Cannot open video encoder codec: Function not implemented. Please make sure you have installed VAAPI runtime.

This seems to be an issue for AMD GPU fedora 39+ users, but maybe others.

### Fix

Switch from `mesa-va-drivers` to `mesa-va-drivers-freeworld`. [Guide on how to do so.](https://fostips.com/hardware-acceleration-video-fedora/) Then reboot your machine.

## Nvidia

Alvr requires at least driver version 535 and CUDA version 12.1. If this is not the case SteamVR or the encoder might not work.

### Fix

Install at least the required versions of the driver and ensure you have CUDA installed with at least version 12.1.

If an error saying CUDA was not detected persists, try using the latest alvr nightly release.

## Hybrid graphics

### Amd/Intel integrated gpu + Amd/Intel discrete gpu

Put `DRI_PRIME=1 %command%` into SteamVR's commandline options and in those of all VR games you intend to play with ALVR.

### Amd/Intel integrated gpu + Nvidia discrete gpu

Put `__NV_PRIME_RENDER_OFFLOAD=1 __VK_LAYER_NV_optimus=NVIDIA_only __GLX_VENDOR_LIBRARY_NAME=nvidia %command%` into SteamVR's commandline options and in those of all VR games you intend to play with ALVR.

## Wayland

When using hyprland or Gnome Wayland you need to put `WAYLAND_DISPLAY='' %command%` into the SteamVR commandline options to force XWayland.

## SlimeVR

The view shakes.

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

## Arch AUR

The alvr driver doesn't get detected by SteamVR.

### Fix

Try using a portable .tar.gz release from the Releases page.

## Low AMDGPU performance and shutters

This might be caused by [[PERF] Subpar GPU performance due to wrong power profile mode · Issue #469 · ValveSoftware/SteamVR-for-Linux · GitHub](https://github.com/ValveSoftware/SteamVR-for-Linux/issues/469).

### Fix

Using CoreCtrl is highly advised (install it using your distribution package management) and in settings set your GPU to VR profile, as well as cpu to performance profile (if it's old Ryzen cpu).
