/*
 * Copyright (c) 2018-2021 Arm Limited.
 *
 * SPDX-License-Identifier: MIT
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to
 * deal in the Software without restriction, including without limitation the
 * rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
 * sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

#pragma once

#include "util/platform_set.hpp"
#include "wsi/display.hpp"

#include <vulkan/vk_icd.h>
#include <vulkan/vk_layer.h>
#include <vulkan/vulkan.h>

#include <cassert>
#include <memory>
#include <mutex>
#include <unordered_set>

using scoped_mutex = std::lock_guard<std::mutex>;

namespace layer {

/* List of device entrypoints in the layer's instance dispatch table.
 * Note that the Vulkan loader implements some of these entrypoints so the fact that these are
 * non-null doesn't guarantee than we can safely call them. We still mark the entrypoints with
 * REQUIRED() and OPTIONAL(). The layer fails if vkGetInstanceProcAddr returns null for entrypoints
 * that are REQUIRED().
 */
#define INSTANCE_ENTRYPOINTS_LIST(REQUIRED, OPTIONAL)                                              \
    REQUIRED(GetInstanceProcAddr)                                                                  \
    REQUIRED(DestroyInstance)                                                                      \
    REQUIRED(GetPhysicalDeviceProperties)                                                          \
    REQUIRED(GetPhysicalDeviceProperties2)                                                          \
    REQUIRED(GetPhysicalDeviceMemoryProperties)                                                    \
    REQUIRED(GetPhysicalDeviceImageFormatProperties)                                               \
    REQUIRED(EnumerateDeviceExtensionProperties)                                                   \
    OPTIONAL(GetPhysicalDeviceSurfaceCapabilitiesKHR)                                              \
    OPTIONAL(GetPhysicalDeviceSurfaceFormatsKHR)                                                   \
    OPTIONAL(GetPhysicalDeviceSurfacePresentModesKHR)                                              \
    OPTIONAL(GetPhysicalDeviceSurfaceSupportKHR)                                                   \
    OPTIONAL(GetPhysicalDeviceDisplayPropertiesKHR)                                                \
    OPTIONAL(GetDisplayModePropertiesKHR)                                                          \
    OPTIONAL(GetPhysicalDeviceDisplayPlanePropertiesKHR)                                           \
    OPTIONAL(AcquireXlibDisplayEXT)                                                                \
    OPTIONAL(GetDisplayPlaneSupportedDisplaysKHR)                                                  \
    OPTIONAL(CreateDisplayPlaneSurfaceKHR)                                                         \
    OPTIONAL(ReleaseDisplayEXT)                                                                    \
    OPTIONAL(DestroySurfaceKHR)                                                                    \
    OPTIONAL(CreateHeadlessSurfaceEXT)                                                             \
    OPTIONAL(GetPhysicalDeviceQueueFamilyProperties)                                               \
    OPTIONAL(CreateDisplayModeKHR)                                                                 \

struct instance_dispatch_table {
    VkResult populate(VkInstance instance, PFN_vkGetInstanceProcAddr get_proc);

#define DISPATCH_TABLE_ENTRY(x) PFN_vk##x x{};
    INSTANCE_ENTRYPOINTS_LIST(DISPATCH_TABLE_ENTRY, DISPATCH_TABLE_ENTRY)
#undef DISPATCH_TABLE_ENTRY
};

/* List of device entrypoints in the layer's device dispatch table.
 * The layer fails initializing a device instance when entrypoints marked with REQUIRED() are
 * retrieved as null. The layer will instead tolerate retrieving a null for entrypoints marked as
 * OPTIONAL(). Code in the layer needs to check these entrypoints are non-null before calling them.
 *
 * Note that we cannot rely on checking whether the physical device supports a particular extension
 * as the Vulkan loader currently aggregates all extensions advertised by all implicit layers (in
 * their JSON manifests) and adds them automatically to the output of
 * vkEnumeratePhysicalDeviceProperties.
 */
#define DEVICE_ENTRYPOINTS_LIST(REQUIRED, OPTIONAL)                                                \
    REQUIRED(GetDeviceProcAddr)                                                                    \
    REQUIRED(GetDeviceQueue)                                                                       \
    REQUIRED(QueueSubmit)                                                                          \
    REQUIRED(QueueWaitIdle)                                                                        \
    REQUIRED(CreateCommandPool)                                                                    \
    REQUIRED(DestroyCommandPool)                                                                   \
    REQUIRED(AllocateCommandBuffers)                                                               \
    REQUIRED(FreeCommandBuffers)                                                                   \
    REQUIRED(ResetCommandBuffer)                                                                   \
    REQUIRED(BeginCommandBuffer)                                                                   \
    REQUIRED(EndCommandBuffer)                                                                     \
    REQUIRED(CreateImage)                                                                          \
    REQUIRED(DestroyImage)                                                                         \
    REQUIRED(GetImageMemoryRequirements)                                                           \
    REQUIRED(BindImageMemory)                                                                      \
    REQUIRED(AllocateMemory)                                                                       \
    REQUIRED(FreeMemory)                                                                           \
    REQUIRED(CreateFence)                                                                          \
    REQUIRED(DestroyFence)                                                                         \
    REQUIRED(ResetFences)                                                                          \
    REQUIRED(WaitForFences)                                                                        \
    OPTIONAL(CreateSwapchainKHR)                                                                   \
    OPTIONAL(DestroySwapchainKHR)                                                                  \
    OPTIONAL(GetSwapchainImagesKHR)                                                                \
    OPTIONAL(AcquireNextImageKHR)                                                                  \
    OPTIONAL(QueuePresentKHR)                                                                      \
    OPTIONAL(GetSwapchainCounterEXT)                                                               \
    OPTIONAL(RegisterDisplayEventEXT)                                                              \
    OPTIONAL(GetFenceStatus)                                                                       \
    OPTIONAL(GetMemoryFdKHR)                                                                       \
    OPTIONAL(CreateSemaphore)                                                                      \
    OPTIONAL(GetSemaphoreFdKHR)

struct device_dispatch_table {
    VkResult populate(VkDevice dev, PFN_vkGetDeviceProcAddr get_proc);

#define DISPATCH_TABLE_ENTRY(x) PFN_vk##x x{};
    DEVICE_ENTRYPOINTS_LIST(DISPATCH_TABLE_ENTRY, DISPATCH_TABLE_ENTRY)
#undef DISPATCH_TABLE_ENTRY
};

/**
 * @brief Layer "mirror object" for VkInstance.
 */
class instance_private_data {
  public:
    instance_private_data() = delete;
    instance_private_data(const instance_private_data &) = delete;
    instance_private_data &operator=(const instance_private_data &) = delete;

    instance_private_data(const instance_dispatch_table &table,
                          PFN_vkSetInstanceLoaderData set_loader_data,
                          util::wsi_platform_set enabled_layer_platforms);
    static void set(VkInstance inst, std::unique_ptr<instance_private_data> inst_data);

    /**
     * @brief Get the mirror object that the layer associates to a given Vulkan instance.
     */
    static instance_private_data &get(VkInstance instance);

    /**
     * @brief Get the layer instance object associated to the VkInstance owning the specified
     * VkPhysicalDevice.
     */
    static instance_private_data &get(VkPhysicalDevice phys_dev);

    /**
     * @brief Get the set of enabled platforms that are also supported by the layer.
     */
    const util::wsi_platform_set &get_enabled_platforms() { return enabled_layer_platforms; }

    /**
     * @brief Check whether a surface command should be handled by the WSI layer.
     *
     * @param phys_dev Physical device involved in the Vulkan command.
     * @param surface The surface involved in the Vulkan command.
     *
     * @retval @c true if the layer should handle commands for the specified surface, which may mean
     * returning an error if the layer does not support @p surface 's platform.
     *
     * @retval @c false if the layer should call down to the layers and ICDs below to handle the
     * surface commands.
     */
    bool should_layer_handle_surface(VkSurfaceKHR surface);

    /**
     * @brief Check whether the given surface is supported for presentation via the layer.
     *
     * @param surface A VK_KHR_surface surface.
     *
     * @return Whether the WSI layer supports this surface.
     */
    bool does_layer_support_surface(VkSurfaceKHR surface);

    void add_surface(VkSurfaceKHR);

    static void destroy(VkInstance inst);

    const instance_dispatch_table disp;

  private:
    /**
     * @brief Check whether the given surface is already supported for presentation without the
     * layer.
     */
    bool do_icds_support_surface(VkPhysicalDevice phys_dev, VkSurfaceKHR surface);

    const PFN_vkSetInstanceLoaderData SetInstanceLoaderData;
    const util::wsi_platform_set enabled_layer_platforms;

    std::unordered_set<VkSurfaceKHR> surfaces;
};

class device_private_data {
  public:
    device_private_data() = delete;
    device_private_data(const device_private_data &) = delete;
    device_private_data &operator=(const device_private_data &) = delete;

    device_private_data(instance_private_data &inst_data, VkPhysicalDevice phys_dev, VkDevice dev,
                        const device_dispatch_table &table,
                        PFN_vkSetDeviceLoaderData set_loader_data);
    static void set(VkDevice dev, std::unique_ptr<device_private_data> dev_data);

    /**
     * @brief Get the mirror object that the layer associates to a given Vulkan device.
     */
    static device_private_data &get(VkDevice device);

    /**
     * @brief Get the layer device object associated to the VkDevice owning the specified VkQueue.
     */
    static device_private_data &get(VkQueue queue);

    void add_layer_swapchain(VkSwapchainKHR swapchain);

    /**
     * @brief Return whether all the provided swapchains are owned by us (the WSI Layer).
     */
    bool layer_owns_all_swapchains(const VkSwapchainKHR *swapchain, uint32_t swapchain_count) const;

    /**
     * @brief Check whether the given swapchain is owned by us (the WSI Layer).
     */
    bool layer_owns_swapchain(VkSwapchainKHR swapchain) const {
        return layer_owns_all_swapchains(&swapchain, 1);
    }

    /**
     * @brief Check whether the layer can create a swapchain for the given surface.
     */
    bool should_layer_create_swapchain(VkSurfaceKHR vk_surface);

    /**
     * @brief Check whether the ICDs or layers below support VK_KHR_swapchain.
     */
    bool can_icds_create_swapchain(VkSurfaceKHR vk_surface);

    static void destroy(VkDevice dev);

    const device_dispatch_table disp;
    instance_private_data &instance_data;
    const PFN_vkSetDeviceLoaderData SetDeviceLoaderData;
    const VkPhysicalDevice physical_device;
    const VkDevice device;

    std::unique_ptr<wsi::display> display;
  private:
    std::unordered_set<VkSwapchainKHR> swapchains;
    mutable std::mutex swapchains_lock;
};

} /* namespace layer */
