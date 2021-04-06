#pragma once

#include <atomic>
#include <vulkan/vulkan.h>
#include <thread>

namespace layer
{
class device_private_data;
}

namespace wsi {

class display {
  public:
    display(VkDevice device, layer::device_private_data& device_data, uint32_t queue_family_index, uint32_t queue_index);
    ~display();

    VkFence &get_vsync_fence() { return vsync_fence; }

  private:
    std::atomic_bool m_exiting{false};
    std::thread m_vsync_thread;
    VkFence vsync_fence;
};

} // namespace wsi
