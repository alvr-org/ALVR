#include <chrono>
#include <condition_variable>
#include <mutex>
#include <vulkan/vulkan.h>

namespace wsi {

class Fence {
  public:
    Fence(bool signaled = false) : m_signaled(signaled) {}

    void signal();
    void reset();
    bool wait(std::chrono::steady_clock::time_point until);
    bool get();

    operator VkFence() const;

  private:
    bool m_signaled;
    std::mutex m_mutex;
    std::condition_variable m_cv;
};

} // namespace wsi
