# Real time video upscaling experiments

# Why?

The Quest can display a resolution close to 4k. Rendering a game, encoding and decoding these kinds of resolutions is very taxing on both the PC and the Quest. So usually a lower resolution image displayed on the Quest.

Ideally the output of such an upscaled image should match the screens pixels 1:1. But because of the Asynchronous Timewarp step this is not possible in the Quest. OVR only accepts undistorted frames.

Currently ALVR does no upscaling prior to the image being mapped to an OpenGL texture. This texture gets interpolated to match the screen pixels by OVR. For this process video resolutions above 100% it use bilinear interpolation and for resolutions below 100% it uses nearest neighbor.

There's a lot of good info on this topic in this issue: https://github.com/alvr-org/ALVR/issues/39

# Lanczos resampling

This traditional upscaling method seems like a good step up from basic bilinear interpolation and is relatively light on GPU resources.

A GPL 2 implementation of a Lanczos shader can be found here: https://github.com/obsproject/obs-studio/blob/6943d9a973aa3dc935b39f99d06f4540ea79da61/libobs/data/lanczos_scale.effect

# Neural net image super resolution

I did some basic investigations on the feasibility of using AI upscalers to get even better results than traditional signal processing methods.

## Hardware acceleration on the XR2

There seem to be 3 paths towards getting fast NNs running on the Quest's SoC.
There is the [Qualcomm Neural Processing SDK](https://developer.qualcomm.com/software/qualcomm-neural-processing-sdk/tools), which automatically detects what the capabilities of the system are and picks the right hardware to run the NN on (GPU, DSP, AI accelerator).

The [TensorFlow Lite NNAPI delegate](https://www.tensorflow.org/lite/performance/nnapi) relies on hardware and driver support for the Android Neural Networks API.

Then there is also the [TensorFlow Lite Hexagon delegate](https://www.tensorflow.org/lite/performance/hexagon_delegate) which specifically targets the Snapdragon DSP.

I only tested an example image super-resolution app from the [tensorflow respository](https://github.com/tensorflow/examples/tree/master/lite/examples/super_resolution) in CPU and generic GPU (OpenCL) accelerated modes. Even upscaling tiny 50x50 images took around 500ms with this. Even though better hardware acceleration could improve this I do not expect 100x improvements. The only hope for NN super-resolution to be viable would be to find a significantly faster neural net, which leads us into the next topic.

## Existing neural nets

A well established real-time upscaler is [Anime4K](https://github.com/bloc97/Anime4K/). It states that it can achieve 1080p to 2160p upscaling in 3ms on a Vega64 GPU. A [rough estimate](https://uploadvr.com/oculus-quest-2-benchmarks/) puts the Quest 2 at a 10x performance disadvantage compared to such high end desktop GPUs. It doesn't seem entirely impossible to get this to work with some optimizations and lowering of the upscaling quality, but there is more bad news. Anime4K has a rather bad Peak signal-to-noise ratio (PSNR). It can get away with this because the stylized look anime is quite forgiving in being heavily filtered.

For an upscaler that has a better PSNR there are many options but very few that can run real-time. The smallest neural net that I could find is [SubPixel-BackProjection](https://github.com/supratikbanerjee/SubPixel-BackProjection_SuperResolution). Tt gets nice results but in my testing took 3 seconds to upscale from 720p to 1080p with CUDA acceleration. Way out of the ballpark for XR2 the chip.

So in conclusion, it does not seem like there is enough performance to squeeze out of the XR2 to do do real-time NN upscaling at such high resolutions. We will more likely get better results out of classical techniques.
