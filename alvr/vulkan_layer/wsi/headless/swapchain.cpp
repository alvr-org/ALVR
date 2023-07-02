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
#include <new>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <vulkan/vulkan.h>
#include <sys/mman.h>

#include <util/timed_semaphore.hpp>

#include "util/logger.h"
#include "platform/linux/protocol.h"
#include "swapchain.hpp"
#include "wsi/display.hpp"

namespace wsi {
namespace headless {

struct image_data {
    /* Device memory backing the image. */
    VkDeviceMemory memory;
};

swapchain::swapchain(layer::device_private_data &dev_data, const VkAllocationCallbacks *pAllocator)
    : wsi::swapchain_base(dev_data, pAllocator), m_display(*dev_data.display) {}

swapchain::~swapchain() {
    /* Call the base's teardown */
    close(m_socket);
    teardown();
}

VkResult swapchain::create_image(const VkImageCreateInfo &image_create,
                                 wsi::swapchain_image &image) {
    VkResult res = VK_SUCCESS;
    m_create_info = image_create;
    m_create_info.usage |= VK_IMAGE_USAGE_TRANSFER_SRC_BIT
      | VK_IMAGE_USAGE_TRANSFER_DST_BIT
      | VK_IMAGE_USAGE_SAMPLED_BIT
      | VK_IMAGE_USAGE_STORAGE_BIT;
    res = m_device_data.disp.CreateImage(m_device, &m_create_info, nullptr, &image.image);
    if (res != VK_SUCCESS) {
        return res;
    }
    m_create_info.pNext = nullptr;
    m_create_info.pQueueFamilyIndices = nullptr;

    VkMemoryRequirements memory_requirements;
    m_device_data.disp.GetImageMemoryRequirements(m_device, image.image, &memory_requirements);

    /* Find a memory type */
    size_t mem_type_idx = 0;
    VkMemoryPropertyFlags memFlags = VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;
    VkPhysicalDeviceMemoryProperties prop;
    m_device_data.instance_data.disp.GetPhysicalDeviceMemoryProperties(m_device_data.physical_device, &prop);
    for (; mem_type_idx < prop.memoryTypeCount; ++mem_type_idx) {
        if ((prop.memoryTypes[mem_type_idx].propertyFlags & memFlags) == memFlags && memory_requirements.memoryTypeBits & (1 << mem_type_idx)) {
            break;
        }
    }
    assert(mem_type_idx < prop.memoryTypeCount);

    VkExportMemoryAllocateInfo export_info = {};
    export_info.sType = VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO;
    export_info.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;

    VkMemoryDedicatedAllocateInfo ded_info = {};
    ded_info.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO;
    ded_info.image = image.image;
    ded_info.pNext = &export_info;

    VkMemoryAllocateInfo mem_info = {};
    mem_info.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    mem_info.allocationSize = memory_requirements.size;
    mem_info.memoryTypeIndex = mem_type_idx;
    mem_info.pNext = &ded_info;
    m_mem_index = mem_type_idx;
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
    fd_info.sType = VK_STRUCTURE_TYPE_MEMORY_GET_FD_INFO_KHR;
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

    VkExportSemaphoreCreateInfo exp_info = {};
    exp_info.sType = VK_STRUCTURE_TYPE_EXPORT_SEMAPHORE_CREATE_INFO;
    exp_info.handleTypes = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT;

    VkSemaphoreTypeCreateInfo tim_info = {};
    tim_info.sType = VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO;
    tim_info.pNext = &exp_info;
    tim_info.semaphoreType = VK_SEMAPHORE_TYPE_TIMELINE;

    VkSemaphoreCreateInfo sem_info = {};
    sem_info.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
    sem_info.pNext = &tim_info;

    res = m_device_data.disp.CreateSemaphore(m_device, &sem_info, nullptr, &image.semaphore);
    if (res != VK_SUCCESS) {
        Error("CreateSemaphore failed\n");
        destroy_image(image);
        return res;
    }

    VkSemaphoreGetFdInfoKHR sem_fd_info = {};
    sem_fd_info.sType = VK_STRUCTURE_TYPE_SEMAPHORE_GET_FD_INFO_KHR;
    sem_fd_info.semaphore = image.semaphore;
    sem_fd_info.handleType = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT;

    res = m_device_data.disp.GetSemaphoreFdKHR(m_device, &sem_fd_info, &fd);
    if (res != VK_SUCCESS) {
        Error("GetSemaphoreFdKHR failed\n");
        destroy_image(image);
        return res;
    }
    m_fds.push_back(fd);
    Debug("GetSemaphoreFdKHR returned fd=%d\n", fd);

    return res;
}

int swapchain::send_fds() {
    // This function does the arcane magic for sending
    // file descriptors over unix domain sockets
    // Stolen from https://gist.github.com/kokjo/75cec0f466fc34fa2922
    //
    // There will always be 6 fds (for the 3 images and sempahores created in the swapchain) so we can avoid
    // dynamic length. Initially, I tried to send the length in the normal data field (msg.msg_iov /
    // data) but for some reason it was emptied on arrival, no matter what I did.
    //
    struct msghdr msg;
    struct iovec iov[1];
    struct cmsghdr *cmsg = NULL;
    assert(m_fds.size() == 6);
    int fds[6];
    char ctrl_buf[CMSG_SPACE(sizeof(fds))];
    char data[1];

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

    int ret = sendmsg(m_socket, &msg, 0);

    for (auto fd: m_fds)
      close(fd);

    return ret;
}

bool swapchain::try_connect() {
    Debug("swapchain::try_connect\n");
    m_socketPath = getenv("XDG_RUNTIME_DIR");
    m_socketPath += "/alvr-ipc";

    int ret;
    if (m_socket == -1) {
        m_socket = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
        if (m_socket == -1) {
            perror("socket");
            exit(1);
        }
    }

    struct sockaddr_un name;
    memset(&name, 0, sizeof(name));
    name.sun_family = AF_UNIX;
    strncpy(name.sun_path, m_socketPath.c_str(), sizeof(name.sun_path) - 1);

    ret = connect(m_socket, (const struct sockaddr *)&name, sizeof(name));
    if (ret == -1) {
        return false; // we will try again next frame
    }

    VkPhysicalDeviceVulkan11Properties props11 = {};
    props11.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_1_PROPERTIES;

    VkPhysicalDeviceProperties2 props = {};
    props.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2;
    props.pNext = &props11;
    m_device_data.instance_data.disp.GetPhysicalDeviceProperties2(m_device_data.physical_device,
                                                                  &props);

    init_packet init{.num_images = uint32_t(m_swapchain_images.size()),
      .device_uuid = {},
      .image_create_info = m_create_info,
      .mem_index = m_mem_index,
      .source_pid = getpid()};
    memcpy(init.device_uuid.data(), props11.deviceUUID, VK_UUID_SIZE);
    ret = write(m_socket, &init, sizeof(init));
    if (ret == -1) {
        perror("write");
        exit(1);
    }

    ret = send_fds();
    if (ret == -1) {
        perror("sendmsg");
        exit(1);
    }
    Debug("swapchain sent fds\n");

    return true;
}

void swapchain::submit_image(uint32_t pending_index) {
    const auto & pose = m_swapchain_images[pending_index].pose.mDeviceToAbsoluteTracking.m;
    if (!m_connected) {
        m_connected = try_connect();
    }
    if (m_connected) {
        int ret;
        present_packet packet;
        packet.image = pending_index;
        packet.frame = m_display.m_vsync_count;
        packet.semaphore_value = m_swapchain_images[pending_index].semaphore_value;
        memcpy(&packet.pose, pose, sizeof(packet.pose));
        ret = write(m_socket, &packet, sizeof(packet));
        if (ret == -1) {
            //FIXME: try to reconnect?
        }
    }
}

void swapchain::present_image(uint32_t pending_index) {
    if (in_flight_index != UINT32_MAX)
        unpresent_image(in_flight_index);
    in_flight_index = pending_index;
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
