/*
 * Copyright (c) 2018-2019, 2021 Arm Limited.
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
 * @file surface_api.hpp
 *
 * @brief Contains the Vulkan entrypoints for the VkSurfaceKHR.
 */
#pragma once

#include <vulkan/vulkan.h>

extern "C" {

/**
 * @brief Implements vkGetPhysicalDeviceSurfaceCapabilitiesKHR Vulkan entrypoint.
 */
VKAPI_ATTR VkResult wsi_layer_vkGetPhysicalDeviceSurfaceCapabilitiesKHR(
    VkPhysicalDevice physicalDevice, VkSurfaceKHR surface,
    VkSurfaceCapabilitiesKHR *pSurfaceCapabilities);

/**
 * @brief Implements vkGetPhysicalDeviceSurfaceFormatsKHR Vulkan entrypoint.
 */
VKAPI_ATTR VkResult wsi_layer_vkGetPhysicalDeviceSurfaceFormatsKHR(
    VkPhysicalDevice physicalDevice, VkSurfaceKHR surface, uint32_t *pSurfaceFormatCount,
    VkSurfaceFormatKHR *pSurfaceFormats);

/**
 * @brief Implements vkGetPhysicalDeviceSurfacePresentModesKHR Vulkan entrypoint.
 */
VKAPI_ATTR VkResult wsi_layer_vkGetPhysicalDeviceSurfacePresentModesKHR(
    VkPhysicalDevice physicalDevice, VkSurfaceKHR surface, uint32_t *pPresentModeCount,
    VkPresentModeKHR *pPresentModes);

/**
 * @brief Implements vkGetPhysicalDeviceSurfaceSupportKHR Vulkan entrypoint.
 */
VKAPI_ATTR VkResult wsi_layer_vkGetPhysicalDeviceSurfaceSupportKHR(VkPhysicalDevice physicalDevice,
                                                                   uint32_t queueFamilyIndex,
                                                                   VkSurfaceKHR surface,
                                                                   VkBool32 *pSupported);
}
