/*
 * Copyright (c) 2017-2020 Arm Limited.
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
 * @file swapchain.cpp
 *
 * @brief Contains the implementation for a headless swapchain.
 */

#include <cassert>
#include <cstdlib>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <vulkan/vulkan.h>

#include "alvr_server/Logger.h"
#include <util/timed_semaphore.hpp>

#include "swapchain.hpp"

namespace wsi {
namespace headless {

struct image_data {
    /* Device memory backing the image. */
    VkDeviceMemory memory;
};

swapchain::swapchain(layer::device_private_data &dev_data, const VkAllocationCallbacks *pAllocator)
    : wsi::swapchain_base(dev_data, pAllocator) {}

swapchain::~swapchain() {
    /* Call the base's teardown */
    teardown();
}

VkResult swapchain::create_image(const VkImageCreateInfo &image_create,
                                 wsi::swapchain_image &image) {
    VkResult res = VK_SUCCESS;
    res = m_device_data.disp.CreateImage(m_device, &image_create, nullptr, &image.image);
    if (res != VK_SUCCESS) {
        return res;
    }

    VkMemoryRequirements memory_requirements;
    m_device_data.disp.GetImageMemoryRequirements(m_device, image.image, &memory_requirements);

    /* Find a memory type */
    size_t mem_type_idx = 0;
    for (; mem_type_idx < 8 * sizeof(memory_requirements.memoryTypeBits); ++mem_type_idx) {
        if (memory_requirements.memoryTypeBits & (1u << mem_type_idx)) {
            break;
        }
    }

    assert(mem_type_idx <= 8 * sizeof(memory_requirements.memoryTypeBits) - 1);

    VkMemoryAllocateInfo mem_info = {};
    mem_info.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    mem_info.allocationSize = memory_requirements.size;
    mem_info.memoryTypeIndex = mem_type_idx;
    image_data *data = nullptr;

    /* Create image_data */
    data = m_allocator.create<image_data>(1);
    if (data == nullptr) {
        m_device_data.disp.DestroyImage(m_device, image.image, get_allocation_callbacks());
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }
    image.data = reinterpret_cast<void *>(data);
    image.status = wsi::swapchain_image::FREE;

    res = m_device_data.disp.AllocateMemory(m_device, &mem_info, nullptr, &data->memory);
    assert(VK_SUCCESS == res);
    if (res != VK_SUCCESS) {
        destroy_image(image);
        return res;
    }

    res = m_device_data.disp.BindImageMemory(m_device, image.image, data->memory, 0);
    assert(VK_SUCCESS == res);
    if (res != VK_SUCCESS) {
        destroy_image(image);
        return res;
    }

    /* Initialize presentation fence. */
    VkFenceCreateInfo fence_info = {VK_STRUCTURE_TYPE_FENCE_CREATE_INFO, nullptr, 0};
    res = m_device_data.disp.CreateFence(m_device, &fence_info, nullptr, &image.present_fence);
    if (res != VK_SUCCESS) {
        destroy_image(image);
        return res;
    }

    // Export into a FD to send later
    VkMemoryGetFdInfoKHR fd_info = {};
    fd_info.sType = VK_STRUCTURE_TYPE_IMPORT_MEMORY_FD_INFO_KHR;
    fd_info.pNext = NULL;
    fd_info.memory = data->memory;
    fd_info.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;

    int fd;
    res = m_device_data.disp.GetMemoryFdKHR(m_device, &fd_info, &fd);
    if (res != VK_SUCCESS) {
        Error("GetMemoryFdKHR failed\n");
        destroy_image(image);
        return res;
    }
    m_fds.push_back(fd);
    Debug("GetMemoryFdKHR returned fd=%d\n", fd);

    return res;
}

int swapchain::send_fds() {
    // This function does the arcane magic for sending
    // file descriptors over unix domain sockets
    // Stolen from https://gist.github.com/kokjo/75cec0f466fc34fa2922
    //
    // There will always be 3 fds (for the 3 images created in the swapchain) so we can avoid
    // dynamic length. Initially, I tried to send the length in the normal data field (msg.msg_iov /
    // data) but for some reason it was emptied on arrival, no matter what I did.
    //
    struct msghdr msg;
    struct iovec iov[1];
    struct cmsghdr *cmsg = NULL;
    int fds[3];
    char ctrl_buf[CMSG_SPACE(sizeof(fds))];
    char data[1];

    assert(m_fds.size() == 3);
    std::copy(m_fds.begin(), m_fds.end(), fds);

    memset(&msg, 0, sizeof(struct msghdr));
    memset(ctrl_buf, 0, CMSG_SPACE(sizeof(fds)));

    iov[0].iov_base = data;
    iov[0].iov_len = sizeof(data);

    msg.msg_name = NULL;
    msg.msg_namelen = 0;
    msg.msg_iov = iov;
    msg.msg_iovlen = 1;
    msg.msg_controllen = CMSG_SPACE(sizeof(fds));
    msg.msg_control = ctrl_buf;

    cmsg = CMSG_FIRSTHDR(&msg);
    cmsg->cmsg_level = SOL_SOCKET;
    cmsg->cmsg_type = SCM_RIGHTS;
    cmsg->cmsg_len = CMSG_LEN(sizeof(fds));

    memcpy(CMSG_DATA(cmsg), fds, sizeof(fds));

    return sendmsg(m_socket, &msg, 0);
}

bool swapchain::try_connect(uint32_t current_index) {
    Debug("swapchain::try_connect\n");
    m_socketPath = getenv("XDG_RUNTIME_DIR");
    m_socketPath += "/alvr-ipc";

    int ret;
    m_socket = socket(AF_UNIX, SOCK_STREAM, 0);
    struct sockaddr_un name;
    if (m_socket == -1) {
        perror("socket");
        exit(1);
    }

    memset(&name, 0, sizeof(name));
    name.sun_family = AF_UNIX;
    strncpy(name.sun_path, m_socketPath.c_str(), sizeof(name.sun_path) - 1);

    ret = connect(m_socket, (const struct sockaddr *)&name, sizeof(name));
    if (ret == -1) {
        return false; // we will try again next frame
    }

    ret = send_fds();
    if (ret == -1) {
        perror("sendmsg");
        exit(1);
    }
    Debug("swapchain sent fds\n");

    close(m_socket);
    return true;
}

void swapchain::present_image(uint32_t pending_index) {
    if (!m_connected) {
        m_connected = try_connect();
    } else {
        int ret;
        ret = write(m_socket, pending_index, sizeof(pending_index));
        if (ret == -1) {
            Error("error while trying to send pending_index");
            perror("write");
            exit(1);
        }
    }
    unpresent_image(pending_index);
}

void swapchain::destroy_image(wsi::swapchain_image &image) {
    if (image.status != wsi::swapchain_image::INVALID) {
        if (image.present_fence != VK_NULL_HANDLE) {
            m_device_data.disp.DestroyFence(m_device, image.present_fence, nullptr);
            image.present_fence = VK_NULL_HANDLE;
        }

        if (image.image != VK_NULL_HANDLE) {
            m_device_data.disp.DestroyImage(m_device, image.image, get_allocation_callbacks());
            image.image = VK_NULL_HANDLE;
        }
    }

    if (image.data != nullptr) {
        auto *data = reinterpret_cast<image_data *>(image.data);
        if (data->memory != VK_NULL_HANDLE) {
            m_device_data.disp.FreeMemory(m_device, data->memory, nullptr);
            data->memory = VK_NULL_HANDLE;
        }
        m_allocator.destroy(1, data);
        image.data = nullptr;
    }

    image.status = wsi::swapchain_image::INVALID;
}

} /* namespace headless */
} /* namespace wsi */
