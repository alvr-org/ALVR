# Vulkan® Window System Integration Layer

## Introduction

This project is a Vulkan® layer which implements some of the Vulkan® window system
integration extensions such as `VK_KHR_swapchain`. The layer is designed to be
GPU vendor agnostic when used as part of the Vulkan® ICD/loader architecture.

Our vision for the project is to become the de facto implementation for Vulkan®
window system integration extensions so that they need not be implemented in the
ICD; instead, the implementation of these extensions are shared across vendors
for mutual benefit.

The project currently implements support for `VK_EXT_headless_surface` and
its dependencies. We hope to extend support for further platforms such as Wayland
and direct-to-display rendering in the future.

## Building

### Dependencies

* [CMake](https://cmake.org) version 2.8.11 or above.
* C++11 compiler.
* Vulkan® loader and associated headers with support for the
  `VK_EXT_headless_surface` extension.

### Building the Vulkan® loader

This step is not necessary if your system already has a loader and associated
headers with support for the `VK_EXT_headless_surface` extension. We include
these instructions for completeness.

```
git clone https://github.com/KhronosGroup/Vulkan-Loader.git
mkdir Vulkan-Loader/build
cd Vulkan-Loader/build
../scripts/update_deps.py
cmake -C helper.cmake ..
make
make install
```

### Building the layer

The layer requires a version of the loader and headers that includes support for
the `VK_EXT_headless_surface` extension. By default, the build system will use
the system Vulkan® headers as reported by `pkg-config`. This may be overriden by
specifying `VULKAN_CXX_INCLUDE` in the CMake configuration, for example:

```
cmake . -DVULKAN_CXX_INCLUDE="path/to/vulkan-headers"
```

If the loader and associated headers already meet the requirements of the layer
then the build steps are straightforward:

```
cmake . -Bbuild
make -C build
```

## Installation

Copy the shared library `libVkLayer_window_system_integration.so` and JSON
configuration `VkLayer_window_system_integration.json` into a Vulkan®
[implicit layer directory](https://vulkan.lunarg.com/doc/view/1.0.39.0/windows/layers.html#user-content-configuring-layers-on-linux).

## Contributing

We are open for contributions.

 * The software is provided under the MIT license. Contributions to this project
   are accepted under the same license.
 * Please also ensure that each commit in the series has at least one
   `Signed-off-by:` line, using your real name and email address. The names in
   the `Signed-off-by:` and `Author:` lines must match. If anyone else
   contributes to the commit, they must also add their own `Signed-off-by:`
   line. By adding this line the contributor certifies the contribution is made
   under the terms of the [Developer Certificate of Origin (DCO)](DCO.txt).
 * Questions, bug reports, et cetera are raised and discussed on the issues page.
 * Please make merge requests into the master branch.
 * Code should be formatted with clang-format using the project's .clang-format
   configuration.

Contributors are expected to abide by the
[freedesktop.org code of conduct](https://www.freedesktop.org/wiki/CodeOfConduct/).

## Khronos® Conformance

This software is based on a published Khronos® Specification and is expected to
pass the relevant parts of the Khronos® Conformance Testing Process when used as
part of a conformant Vulkan® implementation.
