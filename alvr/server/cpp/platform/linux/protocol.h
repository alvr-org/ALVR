#pragma once

#include <cstdint>
#include <vulkan/vulkan.h>
#include <array>

struct present_packet {
    uint32_t image;
    uint32_t frame;
};

struct init_packet {
    uint32_t num_images;
		std::array<char, VK_MAX_PHYSICAL_DEVICE_NAME_SIZE> device_name;
		VkImageCreateInfo image_create_info;
		size_t mem_index;
};
