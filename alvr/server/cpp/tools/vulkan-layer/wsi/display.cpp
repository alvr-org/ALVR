#include "display.hpp"
#include "layer/private_data.hpp"

#include "alvr_server/Settings.h"

#include <chrono>

wsi::display &wsi::display::get(VkDevice device) {
    static display instance(device);
    return instance;
}

wsi::display::display(VkDevice device) {
  auto &device_data = layer::device_private_data::get(device);
  VkFenceCreateInfo fence_info = {VK_STRUCTURE_TYPE_FENCE_CREATE_INFO, nullptr, 0};
  device_data.disp.CreateFence(device, &fence_info, nullptr, &vsync_fence);

  m_vsync_thread = std::thread([this, &device_data, device]()
      {
      VkQueue queue;
      auto refresh = Settings::Instance().m_refreshRate;
      auto next_frame = std::chrono::steady_clock::now();
      auto frame_time = std::chrono::duration_cast<decltype(next_frame)::duration>(std::chrono::duration<double>(1. / refresh));
      device_data.disp.GetDeviceQueue(device, 0, 0, &queue);
      while (not m_exiting) {
        if (device_data.disp.GetFenceStatus(device, vsync_fence) == VK_NOT_READY)
        {
          device_data.disp.QueueSubmit(queue, 0, nullptr, vsync_fence);
        }
        std::this_thread::sleep_until(next_frame);
        next_frame += frame_time;
      }
      });
}

wsi::display::~display() {
  m_exiting = true;
  m_vsync_thread.join();
}
