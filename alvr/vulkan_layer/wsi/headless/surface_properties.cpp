/*
 * Copyright (c) 2017-2019, 2021 Arm Limited.
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

#include <algorithm>
#include <array>
#include <cassert>
#include <cstdlib>
#include <map>
#include <mutex>

#include <vulkan/vk_icd.h>
#include <vulkan/vulkan.h>

#include <layer/private_data.hpp>
#include "layer/settings.h"

#include "surface_properties.hpp"

#define UNUSED(x) ((void)(x))

namespace wsi {
namespace headless {

surface_properties &surface_properties::get_instance() {
    static surface_properties instance;
    return instance;
}

VkResult
surface_properties::get_surface_capabilities(VkPhysicalDevice physical_device, VkSurfaceKHR surface,
                                             VkSurfaceCapabilitiesKHR *surface_capabilities) {
    UNUSED(surface);
    /* Image count limits */
    surface_capabilities->minImageCount = 1;
    /* There is no maximum theoretically speaking */
    surface_capabilities->maxImageCount = UINT32_MAX;

    /* Surface extents */
    surface_capabilities->currentExtent = surface_capabilities->maxImageExtent =
        surface_capabilities->minImageExtent = {Settings::Instance().m_renderWidth,
                                                Settings::Instance().m_renderHeight};
    /* Ask the device for max */
    VkPhysicalDeviceProperties dev_props;
    layer::instance_private_data::get(physical_device)
        .disp.GetPhysicalDeviceProperties(physical_device, &dev_props);

    surface_capabilities->maxImageArrayLayers = 1;

    /* Surface transforms */
    surface_capabilities->supportedTransforms = VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR;
    surface_capabilities->currentTransform = VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR;

    /* Composite alpha */
    surface_capabilities->supportedCompositeAlpha = static_cast<VkCompositeAlphaFlagBitsKHR>(
        VK_COMPOSITE_ALPHA_INHERIT_BIT_KHR | VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR |
        VK_COMPOSITE_ALPHA_PRE_MULTIPLIED_BIT_KHR | VK_COMPOSITE_ALPHA_POST_MULTIPLIED_BIT_KHR);

    /* Image usage flags */
    surface_capabilities->supportedUsageFlags =
        VK_IMAGE_USAGE_TRANSFER_SRC_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT |
        VK_IMAGE_USAGE_SAMPLED_BIT | VK_IMAGE_USAGE_STORAGE_BIT |
        VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_INPUT_ATTACHMENT_BIT;

    return VK_SUCCESS;
}

VkResult surface_properties::get_surface_formats(VkPhysicalDevice physical_device,
                                                 VkSurfaceKHR surface,
                                                 uint32_t *surface_format_count,
                                                 VkSurfaceFormatKHR *surface_formats) {
    UNUSED(surface);

    VkResult res = VK_SUCCESS;
    /* Construct a list of all formats supported by the driver - for color attachment */
    VkFormat formats[] = {
      VK_FORMAT_R8_UNORM,
      VK_FORMAT_R16_UNORM,
      VK_FORMAT_R8G8_UNORM,
      VK_FORMAT_R16G16_UNORM,
      VK_FORMAT_B8G8R8A8_UNORM,
      VK_FORMAT_R8G8B8A8_UNORM};
    uint32_t format_count = 0;

    for (size_t id = 0; id < std::size(formats); id++) {
        VkImageFormatProperties image_format_props;

        res = layer::instance_private_data::get(physical_device)
                  .disp.GetPhysicalDeviceImageFormatProperties(
                      physical_device, formats[id], VK_IMAGE_TYPE_2D,
                      VK_IMAGE_TILING_OPTIMAL, VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
                      VK_IMAGE_CREATE_MUTABLE_FORMAT_BIT, &image_format_props);

        if (res != VK_ERROR_FORMAT_NOT_SUPPORTED) {
            formats[format_count] = formats[id];
            format_count++;
        }
    }
    assert(format_count > 0);
    assert(surface_format_count != nullptr);
    res = VK_SUCCESS;
    if (nullptr == surface_formats) {
        *surface_format_count = format_count;
    } else {
        if (format_count > *surface_format_count) {
            res = VK_INCOMPLETE;
        }

        *surface_format_count = std::min(*surface_format_count, format_count);
        for (uint32_t i = 0; i < *surface_format_count; ++i) {
            surface_formats[i].format = formats[i];
            surface_formats[i].colorSpace = VK_COLORSPACE_SRGB_NONLINEAR_KHR;
        }
    }

    return res;
}

VkResult surface_properties::get_surface_present_modes(VkPhysicalDevice physical_device,
                                                       VkSurfaceKHR surface,
                                                       uint32_t *present_mode_count,
                                                       VkPresentModeKHR *present_modes) {
    UNUSED(physical_device);
    UNUSED(surface);

    VkResult res = VK_SUCCESS;
    static const std::array<VkPresentModeKHR, 2> modes = {VK_PRESENT_MODE_FIFO_KHR,
                                                          VK_PRESENT_MODE_FIFO_RELAXED_KHR};

    assert(present_mode_count != nullptr);

    if (nullptr == present_modes) {
        *present_mode_count = modes.size();
    } else {
        if (modes.size() > *present_mode_count) {
            res = VK_INCOMPLETE;
        }
        *present_mode_count = std::min(*present_mode_count, static_cast<uint32_t>(modes.size()));
        for (uint32_t i = 0; i < *present_mode_count; ++i) {
            present_modes[i] = modes[i];
        }
    }

    return res;
}

} /* namespace headless */
} /* namespace wsi */
