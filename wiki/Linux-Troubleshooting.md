# Linux Troubleshooting

## SteamVR
The steam runtimes SteamVR runs in break the alvr driver loaded by SteamVR.
This causes the screen to stay black on the client or an error to be reported that the pipewire device is missing or can even result in SteamVR crashing.

### Fix
Add `~/.steam/steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` to the commandline options of SteamVR (SteamVR -> Manage/Right Click -> Properties -> General -> Launch Options).

The path might differ based on your installation of Steam, for example on systems using the deb-package like Debian and Mint `~/.steam/debian-installation/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` is needed.

## Amdvlk/AMD
If you have Amdvlk installed on your system, it overrides other vulkan drivers and causes SteamVR to break. Use the `vulkan-radeon` driver (aka radv) instead.

### Fix
Check if Amdvlk is installed by seeing if `ls /usr/share/vulkan/icd.d/ | grep amd_icd` shows anything. If so, uninstall Amdvlk from your system.

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

## Arch AUR
The alvr driver doesn't get detected by SteamVR.

### Fix
Try using a portable .tar.gz release from the Releases page.
