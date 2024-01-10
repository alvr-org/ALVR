**Warning:** This page is very outdated, see [Building From Source](https://github.com/alvr-org/ALVR/wiki/Building-From-Source) instead.

## 2022-01-04

An experimental NVENC fork has successfully been created by [Toxblh](https://github.com/Toxblh), helping fix one of the larger bottlenecks on NVIDIA GPUs. [Pull Request here](https://github.com/alvr-org/ALVR/pull/906)

## 2021-05-18

No special build steps are required for users who can acquire the correct ffmpeg version, read more [here](https://github.com/alvr-org/ALVR/wiki/Build-from-source#linux-experimental-build).

## 2021-04-22

The PR in the last log was proceeded by [#604](https://github.com/alvr-org/ALVR/pull/604) and this new PR was merged into the main branch. Build instructions remain the same, but the `vrenv.sh` patching is no longer needed.

## 2021-04-01

A [PR](https://github.com/alvr-org/ALVR/pull/569) has been made integrating Xytovl's vulkan layer into the main ALVR tree. It doesn't actually stream video yet but it provides a solid base for future work and is compatible with nVidia GPUs.

After you've checked the PR's branch out and [built the streamer](https://github.com/alvr-org/ALVR/wiki/Build-from-source#build-streamer), you can build and install the Vulkan layer like this:

```
cd alvr/server/cpp/tools/vulkan-layer
mkdir build && cd build
cmake ..
make -j
```

Add this line: `source "$(cat $XDG_RUNTIME_DIR/alvr_dir.txt | rev | cut -d'/' -f3- | rev)/alvr/server/cpp/tools/vulkan-layer/layer/vrenv.sh"` **before** the last one (`exec "$@"`) to `/path/to/your/SteamLibrary/steamapps/common/SteamVR/bin/vrenv.sh`.

## 2021-03-15

Xytovl's branch has been merged into the main repository. The build steps are unchanged.
Work has started towards a new frame capturing method using a Vulkan debug layer.

## 2021-03-10

An experimental branch is available at https://github.com/xytovl/ALVR/tree/linux-port-openvr with many limitations

### Adopted solution

We use SteamVR direct rendering mode on a fake screen, and capture the output of the screen. Current implementation only works for AMD (and probably Intel) open source drivers.

### Limitations

- audio streaming is not working
- foveated encoding is not implemented
- requires superuser access for setup
- mostly untested
- requires a free port on the graphic card
- TCP streaming seems not to be working
- position is stuttering
- only supports open source drivers

### Setup

See [build from source](Build-from-source)

## Usage

Run `build/alvr_streamer_linux/ALVR Dashboard`

On first setup, SteamVR will probably show the VR display on your screen, with the configuration window. If you have dual screen, you can move the configuration window to a visible area (with Alt + drag on most desktop environments).

In the setup, deactivate audio streaming, switch connection to UDP, and deactivate foveated encoding.

On the headset, launch the application, then click trust on the configuration window, which will quit.

The headset says that the streamer will restart, but it will not. You must relaunch it manually.

If you are here, once it is all restarted, you should be able to get the stream on the headset.

## 2021-01-15

The development road has been defined, but we are not completely sure everything will work.

* We can try to extract frames from the VR game using a custom Vulkan validation layer. Examples are:
  * Vulkan tools screenshot: https://github.com/LunarG/VulkanTools/blob/master/layersvt/screenshot.cpp
  * RenderDoc: https://github.com/baldurk/renderdoc
* For the compositor (layering, color correction and foveated rendering) we are going to use Vulkan as the underlying API. We can use the backend agnostic library gfx-hal, that supports Vulkan and DirectX. Reference: https://github.com/gfx-rs/gfx
* For the encoder we can use FFmpeg. FFmpeg's hardware acceleration API supports passing pointers to GPU memory buffers directly. FFmpeg supports various acceleration APIs (hardware agnostic or not) but to minimize the effort we can go with Vulkan for Linux and DirectX 11 for Windows. Reference: https://ffmpeg.org/doxygen/trunk/hwcontext_8h.html
* For audio we are going to use the Rust library CPAL, which is an audio backend abstraction layer. We can switch (maybe even at runtime) between ALSA and JACK. CPAL supports also Windows (WASAPI, ASIO), Android (OpenSL, AAudio), Web (Emscripten) and even macOS (Core Audio) if we need that in the future. Reference: https://github.com/RustAudio/cpal

## Earlier

We cannot find a way of obtaining the frames rendered by the VR game from SteamVR. The OpenVR API exposes methods to do this but they don't work on Linux (at least we were not able to make them work). The two methods to obtain frames with OpenVR are by implementing the interfaces `IVRVirtualDisplay` and `IVRDriverDirectModeComponent`. On Windows, ALVR uses `IVRDriverDirectModeComponent`. On Linux, `IVRVirtualDisplay` crashes on Nvidia GPUs and does nothing on AMD. Similarly `IVRDriverDirectModeComponent` does not work on Linux. We tried to get help from Valve through multiple channels but we were not successful.

References:

* OpenVR driver header: https://github.com/ValveSoftware/openvr/blob/master/headers/openvr_driver.h
* Main OpenVR issue tracker: https://github.com/ValveSoftware/openvr/issues
* Virtual display sample issue tracker: https://github.com/ValveSoftware/virtual_display/issues
* Linux SteamVR issue tracker: https://github.com/ValveSoftware/steam-for-linux/issues
