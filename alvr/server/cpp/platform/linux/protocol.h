#pragma once

#include <array>
#include <atomic>
#include <condition_variable>
#include <cstdint>
#include <cstdlib>
#include <mutex>
#include <vulkan/vulkan.h>

struct present_packet {
    uint32_t image;
    uint32_t frame;
    float pose[3][4];
};

struct init_packet {
    uint32_t num_images;
    std::array<char, VK_MAX_PHYSICAL_DEVICE_NAME_SIZE> device_name;
    VkImageCreateInfo image_create_info;
    size_t mem_index;
    pid_t source_pid;
};
