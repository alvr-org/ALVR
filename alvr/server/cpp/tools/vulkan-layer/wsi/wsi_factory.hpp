/*
 * Copyright (c) 2019, 2021 Arm Limited.
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
 * @brief Contains the factory methods for obtaining the specific surface and swapchain
 * implementations.
 */

#pragma once

#include "surface_properties.hpp"
#include "swapchain_base.hpp"
#include "util/platform_set.hpp"

#include <unordered_map>

namespace wsi {

/**
 * @brief Obtains the surface properties for the specific surface type.
 *
 * @param surface The surface for which to get the properties.
 *
 * @return nullptr if surface type is unsupported.
 */
surface_properties *get_surface_properties(VkSurfaceKHR surface);

/**
 * @brief Allocates a surface specific swapchain.
 *
 * @param surface    The surface for which a swapchain is allocated.
 * @param dev_data   The device specific data.
 * @param pAllocator The allocator from which to allocate any memory.
 *
 * @return nullptr on failure.
 */
swapchain_base *allocate_surface_swapchain(VkSurfaceKHR surface,
                                           layer::device_private_data &dev_data,
                                           const VkAllocationCallbacks *pAllocator);

/**
 * @brief Destroys a swapchain and frees memory. Used with @ref allocate_surface_swapchain.
 *
 * @param swapchain  Pointer to the swapchain to destroy.
 * @param pAllocator The allocator to use for freeing memory.
 */
void destroy_surface_swapchain(swapchain_base *swapchain, const VkAllocationCallbacks *pAllocator);

/**
 * @brief Return which platforms the layer can handle for an instance constructed in the specified
 * way.
 *
 * @details This function looks at the extensions specified in @p pCreateInfo and based on this
 * returns a list of platforms that the layer can support. For example, if the @c
 * pCreateInfo.ppEnabledExtensionNames contains the string "VK_EXT_headless_surface" then the
 * returned platform set will contain @c VK_ICD_WSI_PLATFORM_HEADLESS.
 *
 * @param pCreateInfo Structure used when creating the instance in vkCreateInstance().
 *
 * @return A list of WS platforms supported by the layer.
 */
util::wsi_platform_set find_enabled_layer_platforms(const VkInstanceCreateInfo *pCreateInfo);

/**
 * @brief Add extra extensions that the layer requires to support the specified list of enabled
 * platforms.
 *
 * @details Check whether @p phys_dev has support for the extensions required by the layer in order
 * to support the platforms it implements. The extensions that the layer requires to operate are
 * added to @p extensions_to_enable.
 *
 * @param[in] phys_dev The physical device to check.
 * @param[in] enabled_platforms All the platforms that the layer must enable for @p phys_dev.
 * @param[in,out] extensions_to_enable All the extensions required by the layer are added to this
 * list.
 *
 * @retval @c VK_SUCCESS if the operation was successful.
 */
VkResult add_extensions_required_by_layer(VkPhysicalDevice phys_dev,
                                          const util::wsi_platform_set enabled_platforms,
                                          util::extension_list &extensions_to_enable);

} // namespace wsi
