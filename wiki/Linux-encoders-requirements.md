# Linux encoders requirements

ALVR uses FFmpeg for all encoders (except AMF), so you will need to make sure the encoder of your choice works with FFmpeg.
Always consult Log tab in dashboard, it will tell you the reason why an encoder failed to initialize.

## VAAPI (AMD/Intel GPUs)

Requires *libva* and appropriate driver for your GPU. Check codec support with `vainfo`:

```sh
$ vainfo                                                                                                                                                                       130 ↵ !10090
Trying display: wayland
vainfo: VA-API version: 1.16 (libva 2.16.0)
vainfo: Driver version: Mesa Gallium driver 23.0.0-devel for Radeon RX 7900 XTX (gfx1100, LLVM 16.0.0, DRM 3.49, 6.1.1-zen1-1-zen)
vainfo: Supported profile and entrypoints
      VAProfileH264ConstrainedBaseline:	VAEntrypointVLD
      VAProfileH264ConstrainedBaseline:	VAEntrypointEncSlice
      VAProfileH264Main               :	VAEntrypointVLD
      VAProfileH264Main               :	VAEntrypointEncSlice
      VAProfileH264High               :	VAEntrypointVLD
      VAProfileH264High               :	VAEntrypointEncSlice
      VAProfileHEVCMain               :	VAEntrypointVLD
      VAProfileHEVCMain               :	VAEntrypointEncSlice
      VAProfileHEVCMain10             :	VAEntrypointVLD
      VAProfileHEVCMain10             :	VAEntrypointEncSlice
      VAProfileJPEGBaseline           :	VAEntrypointVLD
      VAProfileVP9Profile0            :	VAEntrypointVLD
      VAProfileVP9Profile2            :	VAEntrypointVLD
      VAProfileAV1Profile0            :	VAEntrypointVLD
      VAProfileNone                   :	VAEntrypointVideoProc
```

*VAProfileH264High, VAProfileHEVCMain, VAProfileHEVCMain10* encoders (VAEntrypointEncSlice) required. If you don't see those
in your output, your driver install is incorrect or your distribution decided to build *mesa* without non-free codecs.

**Test ffmpeg commands**

```sh
# H264
ffmpeg -vaapi_device /dev/dri/renderD128 -f lavfi -i testsrc -t 30 -vf 'format=nv12,hwupload' -c:v h264_vaapi vaapi-h264.mp4

# HEVC
ffmpeg -vaapi_device /dev/dri/renderD128 -f lavfi -i testsrc -t 30 -vf 'format=nv12,hwupload' -c:v hevc_vaapi vaapi-hevc.mp4
```

## AMF (AMD GPUs)

AMF requires proprietary Vulkan driver amd-pro. Troubleshooting AMF installation on your system is out of scope here, but you
can use [amf-test](https://github.com/nowrep/amf-test-linux). HEVC is only supported on RDNA and newer GPUs.

Make sure amf-test succeeds before you try to get it working with ALVR.
You will need to tell ALVR where to find amd-pro driver, edit your SteamVR launch command (change the path as appropriate
for your system):

    env ALVR_AMF_ICD=/path/to/amd_pro_icd64.json %command%

ALVR should now be able to use AMF.

## NVENC (NVidia)

Requires *libcuda*.

**Test ffmpeg commands**

```sh
# H264
ffmpeg -f lavfi -i testsrc -t 30 -vf 'format=nv12,hwupload' -c:v h264_nvenc nvenc-h264.mp4

# HEVC
ffmpeg -f lavfi -i testsrc -t 30 -vf 'format=nv12,hwupload' -c:v hevc_nvenc nvenc-hevc.mp4
```

## Software (all GPUs)

Software encoder is mainly used as a fallback and as such should work on all GPUs without any requirements.
Only H264 encoding is currently supported.
