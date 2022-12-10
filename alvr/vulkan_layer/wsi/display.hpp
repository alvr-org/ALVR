#pragma once

#include <atomic>
#include <vulkan/vulkan.h>
#include <thread>
#include <condition_variable>

namespace wsi {

class display {
  public:
    display();
    ~display();

    VkFence get_vsync_fence();
    VkFence peek_vsync_fence() { return vsync_fence;};

    bool is_signaled() const { return m_signaled; }
    bool wait_for_vsync(uint64_t timeoutNs);

    std::atomic<uint64_t> m_vsync_count{0};

  private:
    std::atomic_bool m_thread_running{false};
    std::atomic_bool m_exiting{false};
    std::thread m_vsync_thread;
    VkFence vsync_fence = VK_NULL_HANDLE;
    std::mutex m_mutex;
    std::condition_variable m_cond;
    std::atomic_bool m_signaled = false;
};

} // namespace wsi
