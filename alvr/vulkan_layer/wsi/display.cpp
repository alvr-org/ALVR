#include "display.hpp"

#include"layer/settings.h"

#include <chrono>

wsi::display::display()
{
}

VkFence wsi::display::get_vsync_fence()
{
  if (not std::atomic_exchange(&m_thread_running, true))
  {
  vsync_fence = reinterpret_cast<VkFence>(this);
  m_vsync_thread = std::thread([this]()
      {
      auto refresh = Settings::Instance().m_refreshRate;
      auto next_frame = std::chrono::steady_clock::now();
      auto frame_time = std::chrono::duration_cast<decltype(next_frame)::duration>(std::chrono::duration<double>(1. / refresh));
      while (not m_exiting) {
        std::this_thread::sleep_until(next_frame);
        m_signaled = true;
        m_cond.notify_all();
        m_vsync_count += 1;
        next_frame += frame_time;
      }
      });
  }
  m_signaled = false;
  return vsync_fence;
}

wsi::display::~display() {
  std::unique_lock<std::mutex> lock(m_mutex);
  m_exiting = true;
  if (m_vsync_thread.joinable())
    m_vsync_thread.join();
}

bool wsi::display::wait_for_vsync(uint64_t timeoutNs)
{
  if (!m_signaled) {
    std::unique_lock<std::mutex> lock(m_mutex);
    return m_cond.wait_for(lock, std::chrono::nanoseconds(timeoutNs)) == std::cv_status::no_timeout;
  }
  return true;
}
