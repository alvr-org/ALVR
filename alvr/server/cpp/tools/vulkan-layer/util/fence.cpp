#include "fence.h"

void wsi::Fence::reset() {
    std::unique_lock<std::mutex> lock(m_mutex);
    m_signaled = false;
}

void wsi::Fence::signal() {
    std::unique_lock<std::mutex> lock(m_mutex);
    m_signaled = true;
    m_cv.notify_all();
}

bool wsi::Fence::wait(std::chrono::steady_clock::time_point until) {
    std::unique_lock<std::mutex> lock(m_mutex);
    if (not m_signaled) {
        m_cv.wait_until(lock, until);
    }
    return m_signaled;
}

bool wsi::Fence::get() {
    std::unique_lock<std::mutex> lock(m_mutex);
    return m_signaled;
}

wsi::Fence::operator VkFence() const { return (VkFence)this; }
