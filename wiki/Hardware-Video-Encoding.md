# FFmpeg Hardware Encoding Testing

FFmpeg hardware video offloading test commands to validate hardware encoding offloading is working.

Learn more at: https://trac.ffmpeg.org/wiki/HWAccelIntro

### Codecs

* **Advanced Video Coding (AVC/h264)** - https://en.wikipedia.org/wiki/Advanced_Video_Coding

	Advanced Video Coding (AVC), also referred to as H.264 or MPEG-4 Part 10, is a video compression standard based on block-oriented, motion-compensated coding. It is by far the most commonly used format for the recording, compression, and distribution of video content. It supports a maximum resolution of 8K UHD. Hardware encoding support is widely available.

* **High Efficiency Video Coding (HEVC/h265)** - https://en.wikipedia.org/wiki/High_Efficiency_Video_Coding

	High Efficiency Video Coding (HEVC), also known as H.265 and MPEG-H Part 2, is a video compression standard designed as part of the MPEG-H project as a successor to the widely used Advanced Video Coding (AVC, H.264, or MPEG-4 Part 10). In comparison to AVC, HEVC offers from 25% to 50% better data compression at the same level of video quality, or substantially improved video quality at the same bit rate. It supports resolutions up to 8192×4320, including 8K UHD, and unlike the primarily 8-bit AVC, HEVC's higher fidelity Main 10 profile has been incorporated into nearly all supporting hardware. Hardware encoding support is widely available.

* **AOMedia Video 1 (AV1)** - https://en.wikipedia.org/wiki/AV1

	AOMedia Video 1 (AV1) is an open, royalty-free video coding format initially designed for video transmissions over the Internet. It was developed as a successor to VP9 by the Alliance for Open Media (AOMedia). The AV1 bitstream specification includes a reference video codec. Hardware encoding support is limited to latest generation hardware.

### Graphics Encoding APIs

* **Video Acceleration API** - https://en.wikipedia.org/wiki/Video_Acceleration_API

	 Video Acceleration API (VA-API) is an open source application programming interface that allows applications such as VLC media player or GStreamer to use hardware video acceleration capabilities, usually provided by the graphics processing unit (GPU). It is implemented by the free and open-source library libva, combined with a hardware-specific driver, usually provided together with the GPU driver.
	 
	 Check your current VA-API status with `vainfo`.

* **Vulkan Video** - https://en.wikipedia.org/wiki/Vulkan

	Vulkan is a low-level low-overhead, cross-platform API and open standard for 3D graphics and computing. It was intended to address the shortcomings of OpenGL, and allow developers more control over the GPU. It is designed to support a wide variety of GPUs, CPUs and operating systems, it is also designed to work with modern multi-core CPUs. Support is upcoming. See: https://www.khronos.org/blog/khronos-releases-vulkan-video-av1-decode-extension-vulkan-sdk-now-supports-h.264-h.265-encode

* **NVENC** - https://en.wikipedia.org/wiki/Nvidia_NVENC

	Nvidia NVENC is a feature in Nvidia graphics cards that performs video encoding, offloading this compute-intensive task from the CPU to a dedicated part of the GPU.

* **AMD Advanced Media Framework**- https://gpuopen.com/advanced-media-framework/

	AMD AMF is a SDK for optimal access to AMD GPUs for multimedia processing.

### Test Source Input Generation
Please note that using the test source for input generation induces CPU load. When monitoring for proper GPU offloading, there will still be expected CPU load from FFmpeg.

```
ffmpeg -hide_banner -f lavfi -i testsrc2=duration=30:size=1280x720:rate=90
```

* **lavfi** - https://ffmpeg.org/ffmpeg-devices.html#toc-lavfi

	Libavfilter input virtual device. This input device reads data from the open output pads of a libavfilter filtergraph. For each filtergraph open output, the input device will create a corresponding stream which is mapped to the generated output. The filtergraph is specified through the option graph.

* **testsrc2** - https://ffmpeg.org/ffmpeg-filters.html#allrgb_002c-allyuv_002c-color_002c-colorchart_002c-colorspectrum_002c-haldclutsrc_002c-nullsrc_002c-pal75bars_002c-pal100bars_002c-rgbtestsrc_002c-smptebars_002c-smptehdbars_002c-testsrc_002c-testsrc2_002c-yuvtestsrc

	The `testsrc2` source generates a test video pattern, showing a color pattern, a scrolling gradient and a timestamp. This is mainly intended for testing purposes. The `testsrc2` source is similar to `testsrc`, but supports more pixel formats instead of just `rgb24`. This allows using it as an input for other tests without requiring a format conversion.
	
		1) duration - how long of a clip in seconds
		2) size - dimensions of the video
		3) rate - frame rate per second

### Render Playback
Use your favorite video player to verify the video was rendered correctly.

* MPV - https://en.wikipedia.org/wiki/Mpv_(media_player)

	mpv is free and open-source media player software based on MPlayer, mplayer2 and FFmpeg. It runs on several operating systems, including Unix-like operating systems (Linux, BSD-based, macOS) and Microsoft Windows, along with having an Android port called mpv-android. It is cross-platform, running on ARM, PowerPC, x86/IA-32, x86-64, and MIPS architecture.

* VLC - https://en.wikipedia.org/wiki/VLC_media_player

	VLC media player (previously the VideoLAN Client and commonly known as simply VLC) is a free and open-source, portable, cross-platform media player software and streaming media server developed by the VideoLAN project. VLC is available for desktop operating systems and mobile platforms, such as Android, iOS and iPadOS. VLC is also available on digital distribution platforms such as Apple's App Store, Google Play, and Microsoft Store.

### Nvidia GPU
Test the Nvidia hardware encoding pipeline. Only NVENC is supported as the current Nvidia VA-API driver (https://github.com/elFarto/nvidia-vaapi-driver) only supports NVDEC. Check your hardware support at https://developer.nvidia.com/video-encode-and-decode-gpu-support-matrix-new for NVENC support.

Monitoring utilities:

* **nvtop** - https://github.com/Syllo/nvtop

	NVTOP stands for Neat Videocard TOP, a (h)top like task monitor for AMD, Intel and NVIDIA GPUs. It can handle multiple GPUs and print information about them in a htop-familiar way.

* **nvidia-smi pmon** - https://developer.nvidia.com/nvidia-system-management-interface

	The NVIDIA System Management Interface (nvidia-smi) is a command line utility, based on top of the NVIDIA Management Library (NVML), intended to aid in the management and monitoring of NVIDIA GPU devices. The `pmon` command lists the statistics for all the compute and graphics processes running on each device.


Nvenc AVC (h264) hardware encoding:

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-c:v h264_nvenc -qp 18 \
nvidia-h264_nvec-90fps-300s.mp4
```

Nvenc HEVC (h265) hardware encoding:

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-c:v hevc_nvenc -qp 18 \
nvidia-hevc_nvec-90fps-300s.mp4
```

Nvenc AV1 hardware encoding (Ada Lovelace or newer hardware):

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-c:v av1_nvenc -qp 18 \
nvidia-av1_nvec-90fps-300s.mp4
```

### Intel GPU
Test the Intel hardware encoding pipeline. Only VA-API is supported with the intel-media-driver (https://github.com/intel/media-driver) on GEN based graphics hardware. Check your hardware support at https://www.intel.com/content/www/us/en/docs/onevpl/developer-reference-media-intel-hardware/1-1/overview.html for encoding codec support.

Monitoring utilities:

* **nvtop** - https://github.com/Syllo/nvtop

	NVTOP stands for Neat Videocard TOP, a (h)top like task monitor for AMD, Intel and NVIDIA GPUs. It can handle multiple GPUs and print information about them in a htop-familiar way.

VA-API AVC (h264) hardware encoding:

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-vaapi_device /dev/dri/renderD128 -vf 'format=nv12,hwupload' \
-c:v h264_vaapi -qp 18 \
intel-h264_vaapi-90fps-300s.mp4
```

VA-API HEVC (h265) hardware encoding:

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-vaapi_device /dev/dri/renderD128 -vf 'format=nv12,hwupload' \
-c:v hevc_vaapi -qp 18 \
intel-hevc_vaapi-90fps-300s.mp4
```

VA-API AV1 hardware encoding (Arc A-Series only):

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-vaapi_device /dev/dri/renderD128 -vf 'format=nv12,hwupload' \
-c:v av1_vaapi -qp 18 \
intel-av1_vaapi-90fps-300s.mp4
```

### AMD GPU
Test the AMD hardware encoding pipeline. Only VA-API is supported with the mesa-va-drivers (https://mesa3d.org/) on AMD based graphics hardware. Check your hardware support at https://en.wikipedia.org/wiki/Unified_Video_Decoder for encoding codec support. Video Core Next (VCN) hardware is required for hardware encoding.

Monitoring utilities:

* **nvtop** - https://github.com/Syllo/nvtop

	NVTOP stands for Neat Videocard TOP, a (h)top like task monitor for AMD, Intel and NVIDIA GPUs. It can handle multiple GPUs and print information about them in a htop-familiar way.

VA-API AVC (h264) hardware encoding:

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-vaapi_device /dev/dri/renderD128 -vf 'format=nv12,hwupload' \
-c:v h264_vaapi -qp 18 \
amd-h264_vaapi-90fps-300s.mp4
```

VA-API HEVC (h265) hardware encoding:

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-vaapi_device /dev/dri/renderD128 -vf 'format=nv12,hwupload' \
-c:v hevc_vaapi -qp 18 \
amd-hevc_vaapi-90fps-300s.mp4
```

VA-API AV1 hardware encoding (VCN 4.0+, Navi 3x only):

```
ffmpeg -hide_banner \
-f lavfi -i testsrc2=duration=300:size=1280x720:rate=90 \
-vaapi_device /dev/dri/renderD128 -vf 'format=nv12,hwupload' \
-c:v av1_vaapi -qp 18 \
amd-av1_vaapi-90fps-300s.mp4
```

