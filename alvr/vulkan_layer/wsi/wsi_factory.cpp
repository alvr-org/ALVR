/*
 * Copyright (c) 2019-2021 Arm Limited.
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

/**
 * @file
 * @brief Implements factory methods for obtaining the specific surface and swapchain
 * implementations.
 */

#include "wsi_factory.hpp"
#include "headless/surface_properties.hpp"
#include "headless/swapchain.hpp"

#include <cassert>
#include <cstdlib>
#include <cstring>
#include <new>
#include <vulkan/vk_icd.h>
#include <vulkan/vulkan_core.h>

namespace wsi {

static struct wsi_extension {
    VkExtensionProperties extension;
    VkIcdWsiPlatform platform;
} const supported_wsi_extensions[] = {
    {{VK_KHR_DISPLAY_EXTENSION_NAME, VK_KHR_DISPLAY_SPEC_VERSION}, VK_ICD_WSI_PLATFORM_HEADLESS}};

static surface_properties *get_surface_properties(VkIcdWsiPlatform platform) {
    switch (platform) {
    case VK_ICD_WSI_PLATFORM_HEADLESS:
        return &headless::surface_properties::get_instance();
    default:
        return nullptr;
    }
}

surface_properties *get_surface_properties(VkSurfaceKHR) {
    return get_surface_properties(VK_ICD_WSI_PLATFORM_HEADLESS);
}

template <typename swapchain_type>
static swapchain_base *allocate_swapchain(layer::device_private_data &dev_data,
                                          const VkAllocationCallbacks *pAllocator) {
    if (!pAllocator) {
        return new swapchain_type(dev_data, pAllocator);
    }
    void *memory =
        pAllocator->pfnAllocation(pAllocator->pUserData, sizeof(swapchain_type),
                                  alignof(swapchain_type), VK_SYSTEM_ALLOCATION_SCOPE_INSTANCE);
    return new (memory) swapchain_type(dev_data, pAllocator);
}

swapchain_base *allocate_surface_swapchain(VkSurfaceKHR,
                                           layer::device_private_data &dev_data,
                                           const VkAllocationCallbacks *pAllocator) {
    return allocate_swapchain<wsi::headless::swapchain>(dev_data, pAllocator);
}

util::wsi_platform_set find_enabled_layer_platforms(const VkInstanceCreateInfo *pCreateInfo) {
    util::wsi_platform_set ret;
    for (const auto &ext_provided_by_layer : supported_wsi_extensions) {
        for (uint32_t i = 0; i < pCreateInfo->enabledExtensionCount; i++) {
            const char *ext_requested_by_user = pCreateInfo->ppEnabledExtensionNames[i];
            if (strcmp(ext_requested_by_user, ext_provided_by_layer.extension.extensionName) == 0) {
                ret.add(ext_provided_by_layer.platform);
            }
        }
    }
    return ret;
}

VkResult add_extensions_required_by_layer(VkPhysicalDevice phys_dev,
                                          const util::wsi_platform_set enabled_platforms,
                                          util::extension_list &extensions_to_enable) {
    util::allocator allocator{extensions_to_enable.get_allocator(),
                              VK_SYSTEM_ALLOCATION_SCOPE_COMMAND};
    util::extension_list device_extensions{allocator};
    VkResult res = device_extensions.add(phys_dev);
    if (res != VK_SUCCESS) {
        return res;
    }

    for (const auto &wsi_ext : supported_wsi_extensions) {
        /* Skip iterating over platforms not enabled in the instance. */
        if (!enabled_platforms.contains(wsi_ext.platform)) {
            continue;
        }

        surface_properties *props = get_surface_properties(wsi_ext.platform);
        const auto &extensions_required_by_layer = props->get_required_device_extensions();
        bool supported = device_extensions.contains(extensions_required_by_layer);
        if (!supported) {
            /* Can we accept failure? The layer unconditionally advertises support for this platform
             * and the loader uses this information to enable its own support of the
             * vkCreate*SurfaceKHR entrypoints. The rest of the Vulkan stack may not support this
             * extension so we cannot blindly fall back to it. For now treat this as an error.
             */
            return VK_ERROR_INITIALIZATION_FAILED;
        }

        res = extensions_to_enable.add(extensions_required_by_layer);
        if (res != VK_SUCCESS) {
            return res;
        }
    }
    return VK_SUCCESS;
}

void destroy_surface_swapchain(swapchain_base *swapchain, const VkAllocationCallbacks *pAllocator) {
    assert(swapchain);

    if (!pAllocator) {
        delete swapchain;
    } else {
        swapchain->~swapchain_base();
        pAllocator->pfnFree(pAllocator->pUserData, reinterpret_cast<void *>(swapchain));
    }
}

} // namespace wsi
