# ALVR capture vulkan layer

## Introduction

The ALVR capture vulkan layer is intended to overcome a limitation of SteamVR runtime on Linux: it does'nt allow software based output devices.
The layer is based on [vulkan wsi layer](https://gitlab.freedesktop.org/mesa/vulkan-wsi-layer), which is meant to implement window system integration as layers.

The ALVR layer adds a display to the vkGetPhysicalDeviceDisplayPropertiesKHR call, and implements all functions related to that device. It then allows images of the swapchain to be shared to an other process (the alvr server process), and communicates present calls.

There are unfortunately a few hacks that make it heavily dependent to SteamVR: requested extentions manipulation to enable the required ones, searching through the stack to find the headset position, and not fully implementing the advertised features.
