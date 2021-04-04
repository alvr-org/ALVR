#pragma once

#include <atomic>
#include <vulkan/vulkan.h>
#include <thread>

namespace wsi {

class display {
  public:
    static display &get(VkDevice device);

    VkFence &get_vsync_fence() { return vsync_fence; }

  private:
    std::atomic_bool m_exiting{false};
    std::thread m_vsync_thread;
    display(VkDevice device);
    ~display();
    VkFence vsync_fence;
};

} // namespace wsi
