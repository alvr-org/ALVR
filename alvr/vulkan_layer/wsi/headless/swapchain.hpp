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

/**
 * @file swapchain.hpp
 *
 * @brief Contains the class definition for a headless swapchain.
 */

#pragma once

#include <vector>

#include <vulkan/vk_icd.h>
#include <vulkan/vulkan.h>
#include <wsi/swapchain_base.hpp>

#include "platform/linux/protocol.h"

namespace wsi {
namespace headless {

/**
 * @brief Headless swapchain class.
 *
 * This class is mostly empty, because all the swapchain stuff is handled by the swapchain class,
 * which we inherit. This class only provides a way to create an image and page-flip ops.
 */
class swapchain : public wsi::swapchain_base {
  public:
    explicit swapchain(layer::device_private_data &dev_data,
                       const VkAllocationCallbacks *pAllocator);

    ~swapchain();

  protected:
    /**
     * @brief Platform specific init
     */
    VkResult init_platform(VkDevice device, const VkSwapchainCreateInfoKHR *pSwapchainCreateInfo) {
        return VK_SUCCESS;
    };

    /**
     * @brief Creates a new swapchain image.
     *
     * @param image_create_info Data to be used to create the image.
     *
     * @param image Handle to the image.
     *
     * @return If image creation is successful returns VK_SUCCESS, otherwise
     * will return VK_ERROR_OUT_OF_DEVICE_MEMORY or VK_ERROR_INITIALIZATION_FAILED
     * depending on the error that occured.
     */
    VkResult create_image(const VkImageCreateInfo &image_create_info, wsi::swapchain_image &image);

    void submit_image(uint32_t pendingIndex);

    /**
     * @brief Method to perform a present - just calls unpresent_image on headless
     *
     * @param pendingIndex Index of the pending image to be presented.
     *
     */
    void present_image(uint32_t pendingIndex);

    /**
     * @brief Method to release a swapchain image
     *
     * @param image Handle to the image about to be released.
     */
    void destroy_image(wsi::swapchain_image &image);

  private:
    bool try_connect();
    int send_fds();
    int m_socket = -1;
    std::string m_socketPath;
    bool m_connected = false;
    std::vector<int> m_fds;
    VkImageCreateInfo m_create_info;
    size_t m_mem_index;
    display &m_display;
    uint32_t in_flight_index = UINT32_MAX;
};

} /* namespace headless */
} /* namespace wsi */
