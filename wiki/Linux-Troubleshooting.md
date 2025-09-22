## (! Mandatory, apply fix if not applied yet !) Black screen even when SteamVR shows movement, Dashboard not detecting launched ALVR/SteamVR

The Steam runtimes SteamVR runs in break the ALVR driver loaded by SteamVR. This causes the screen to stay black on the headset, an error to be reported that the PipeWire device is missing, or can even result in SteamVR crashing.

### Fix

Add `~/.local/share/Steam/steamapps/common/SteamVR/bin/vrmonitor.sh %command%` to the command-line options of SteamVR (SteamVR -> Manage/Right Click -> Properties -> General -> Launch Options).

This path might differ based on your Steam installation; in that case, SteamVR will not start at all. If this is the case, you can figure out the actual path by going to Steam Settings -> Storage.
Then pick the storage location with the star emoji (⭐) and take the path directly above the usage statistics. Prepend this path to `steamapps/common/SteamVR/bin/vrmonitor.sh`. Finally, put the resulting path into the SteamVR command-line options instead of the original one.

### Hyprland/Sway/wlroots Qt fix

If you're on Hyprland, Sway, or another wlroots-based Wayland compositor, you might have to prepend `QT_QPA_PLATFORM=xcb` to your command-line.

Related issue:
[[BUG] No SteamVR UI on wlroots-based wayland compositors (sway, hyprland, ...) with workaround](https://github.com/ValveSoftware/SteamVR-for-Linux/issues/637).

## The ALVR driver doesn't get detected by SteamVR (even after vrmonitor fix)

This can be related to use of an AUR package on Arch.

### Fix

If you're using an Nvidia-based system, ensure you are using a package which supports this (e.g. `alvr-nvidia`). Also try using a launcher (e.g. `alvr-launcher-bin`) or portable .tar.gz release from the Releases page.

## Artifacting, no SteamVR Overlay or graphical glitches in streaming view

This could be related to the AMD AMDVLK or AMDGPU-PRO drivers being present on your system. AMDVLK overrides other Vulkan drivers and can cause SteamVR to break. Also to note is that AMD has discontinued the AMDVLK driver, so limited support should be expected if using it.

### Fix

First check if AMDVLK or AMDGPU-PRO are installed by seeing if `ls /usr/share/vulkan/icd.d/ | grep -e amd_icd -e amd_pro` shows anything. If so, uninstall AMDVLK and/or the AMDGPU-PRO drivers from your system to use the RADV driver instead. (This method may not catch all installations due to distro variations.)

On Arch, first install `vulkan-radeon`, then uninstall other drivers.

## "Failed to create VAAPI encoder" error

Gameplay stream appears blocky or crashes, then an error window appears on your desktop saying:
> Failed to create VAAPI encoder: Cannot open video encoder codec: Function not implemented. Please make sure you have installed VAAPI runtime.

### Fix

For Fedora:
 * Switch from `mesa-va-drivers` to `mesa-va-drivers-freeworld`. [Guide on how to do so](https://fostips.com/hardware-acceleration-video-fedora/) or [the RPM docs](https://rpmfusion.org/Howto/Multimedia).

For Arch (don't use VAAPI for Nvidia):
 * Follow the steps on [this](https://wiki.archlinux.org/title/Hardware_video_acceleration#Installation) page, then reboot your machine.

For other distros (e.g. Manjaro):
 * Install the nonfree version of the Mesa/VAAPI drivers that include the proprietary codecs needed for H264/HEVC encoding.

## Nvidia driver version requirements

ALVR requires the Nvidia driver version >=535 and CUDA version >=12.1. If your configuration doesn't meet these requirements, SteamVR or the encoder might not work.

### Fix

Install the minimum or newer versions of the Nvidia and CUDA drivers.

If errors saying CUDA was not detected persist, try using the latest ALVR nightly release.

## Using ALVR with only integrated graphics

Beware that using **only** integrated graphics for running ALVR is highly inadvisable, as in most cases it will lead to very poor performance (even on more powerful devices like the Steam Deck, it's still very slow). Don't expect things to work perfectly in this case either, as some older integrated graphics may simply not have the best Vulkan support and might fail to work at all. 

## Hybrid graphics advice

### General advice

If you're using a PC and can disable your integrated GPU from the BIOS/UEFI, it's highly advised to do so to avoid multiple problems with handling hybrid graphics. If you're using a laptop and it doesn't allow disabling integrated graphics (in most cases), you'll have to resort to the methods below.

### AMD/Intel integrated GPU + AMD/Intel discrete GPU

Prepend `DRI_PRIME=1` to the command-line options of SteamVR and of all VR games you intend to play with ALVR.

### AMD/Intel integrated GPU + Nvidia discrete GPU

Prepend `__NV_PRIME_RENDER_OFFLOAD=1 __VK_LAYER_NV_optimus=NVIDIA_only __GLX_VENDOR_LIBRARY_NAME=nvidia` to the command-line options of SteamVR and of all VR games you intend to play with ALVR.

If this results in errors such as `error in encoder thread: Failed to initialize vulkan frame context: Invalid argument`, then try adding `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json` to the above command-line options.

- Go to `/usr/share/vulkan/icd.d` and ensure `nvidia_icd.json` exists. It may also be under the name `nvidia_icd.x86_64.json`, in which case you should adjust `VK_DRIVER_FILES` accordingly.
- On older distributions, `VK_DRIVER_FILES` may not be available, in which case you should use the deprecated but equivalent `VK_ICD_FILENAMES`.

### SteamVR Dashboard not rendering in VR on Nvidia discrete GPU
You may need to run the entire Steam client itself via PRIME render offload. First, ensure the Steam client is completely closed. If Steam is already open, you can do so by clicking the Steam dropdown in the top left and choosing "Exit". Then from a terminal run: `__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia steam-runtime`.

## Wayland

When using older Gnome versions (<47) under Wayland, issues may be caused by DRM leasing not being available.

### Fix

Prepend `WAYLAND_DISPLAY=''` to the SteamVR command-line options to force XWayland on SteamVR.

## The view shakes when using SlimeVR

This might be fixed in future updates of ALVR.

### Fix

Start the SlimeVR Server only after you have connected and gotten an image to ALVR at least once.

## Error 109

SteamVR displays 109 or other errors.

### Fix

Start Steam first before starting SteamVR through ALVR. If SteamVR is already started, restart it.

## No audio or microphone

Audio and/or microphone are enabled in presets, but you still can't hear audio or no one can hear you.

### Fix

Make sure you select `ALVR Audio` and/or `ALVR Microphone` in your device list as default **after** connecting the headset. As soon as the headset is disconnected, the devices will be removed. If you set them as default, they will be automatically selected whenever they show up, and you won't need to do it manually ever again. If you don't appear to have the audio devices, or have PipeWire errors in your logs, ensure you have `pipewire` version >=0.3.49 installed by using the command `pipewire --version`. For older Debian (<=11) or Ubuntu-based (<=22.04) distributions, you can check the [pipewire-upstream](https://github.com/pipewire-debian/pipewire-debian) page for instructions on installing newer PipeWire versions.

## Low AMDGPU performance and shutters

This might be caused by [[PERF] Subpar GPU performance due to wrong power profile mode · Issue #469 · ValveSoftware/SteamVR-for-Linux · GitHub](https://github.com/ValveSoftware/SteamVR-for-Linux/issues/469).

### Fix

Using CoreCtrl is highly advised (install it using your distribution's package management system). In its settings, set your GPU to the VR profile, as well as CPU to the performance profile (if it's an old Ryzen CPU).

## OVR Advanced Settings

OVR Advanced Settings is incompatible with ALVR, and will produce a ladder-like latency graph with very bad shifting vision. Disable the OVR Advanced Settings driver, and don't use it with ALVR.

## Bindings not working/high CPU usage due to bindings UI

SteamVR can't properly update bindings, open menus, and/or eats too much CPU.

This issue is caused by SteamVR's webserver spamming requests that stall the Chromium UI and cause it to use a lot of CPU.

### Fix

Apply the following patch: `https://github.com/alvr-org/ALVR-Distrobox-Linux-Guide/blob/main/patch_bindings_spam.sh`.


One-liner assuming the default Steam path for Arch, Fedora: `curl -s https://raw.githubusercontent.com/alvr-org/ALVR-Distrobox-Linux-Guide/main/patch_bindings_spam.sh | sh -s ~/.steam/steam/steamapps/common/SteamVR`.
