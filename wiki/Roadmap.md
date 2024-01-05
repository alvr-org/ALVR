# Roadmap

This post will continue to evolve during ALVR development.

## Long-term goal

Create a universal bridge between XR devices. 

## What is coming next

* Compositor rewrite
  * **Purpose**: add Linux support for FFR and color correction, preparation for sliced encoding
  * **Status**: FFE and color correction done on all platforms
* Encoder rewrite
  * **Purpose**: support any OS and hardware with a single API, using [Vulkan video extensions](https://www.khronos.org/blog/an-introduction-to-vulkan-video)
  * **Status**: blocked by adoption by AMD and Intel, landing of the feature on stable Nvidia drivers
* Monado Driver
  * **Purpose**: support other runtimes with the streamer
  * **Status**: blocked on refactors

Due to the low development capacity, no ETA can be provided. New releases will not have a regular cadence and they do not have scheduled features.
