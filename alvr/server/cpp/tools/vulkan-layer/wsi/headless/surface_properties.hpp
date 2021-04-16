/*
 * Copyright (c) 2017-2019 Arm Limited.
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

#include <vulkan/vk_icd.h>
#include <vulkan/vulkan.h>
#include <wsi/surface_properties.hpp>

namespace wsi {
namespace headless {

class surface_properties : public wsi::surface_properties {
  public:
    VkResult get_surface_capabilities(VkPhysicalDevice physical_device, VkSurfaceKHR surface,
                                      VkSurfaceCapabilitiesKHR *pSurfaceCapabilities) override;

    VkResult get_surface_formats(VkPhysicalDevice physical_device, VkSurfaceKHR surface,
                                 uint32_t *surfaceFormatCount,
                                 VkSurfaceFormatKHR *surfaceFormats) override;

    VkResult get_surface_present_modes(VkPhysicalDevice physical_device, VkSurfaceKHR surface,
                                       uint32_t *pPresentModeCount,
                                       VkPresentModeKHR *pPresentModes) override;

    static surface_properties &get_instance();
};

} /* namespace headless */
} /* namespace wsi */
