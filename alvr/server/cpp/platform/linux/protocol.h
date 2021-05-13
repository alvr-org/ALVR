#pragma once

#include <array>
#include <atomic>
#include <condition_variable>
#include <cstdint>
#include <cstdlib>
#include <mutex>
#include <vulkan/vulkan.h>

typedef enum { UNKNOWN_VENDOR = 0, NVIDIA, AMD } vendor_t;

struct init_packet {
    uint32_t num_images;
    std::array<char, VK_MAX_PHYSICAL_DEVICE_NAME_SIZE> device_name;
    VkImageCreateInfo image_create_info;
    size_t mem_index;
    pid_t source_pid;
    vendor_t pd_vendor;
};

struct present_info {
    float pose[3][4];
};

struct present_shm {
	std::mutex mutex;
	std::condition_variable cv;
	uint32_t next = none_id; // latest frame being offered by producer
	std::atomic<uint32_t> owned_by_consumer{none_id};
	uint32_t size;
	present_info info[];

	static const uint32_t none_id = -1;
};
