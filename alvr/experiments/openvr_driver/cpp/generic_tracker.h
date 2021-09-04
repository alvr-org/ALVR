#pragma once

#include "tracked_device.h"

class GenericTracker : public TrackedDevice {
  public:
    GenericTracker(uint64_t device_index) : TrackedDevice(device_index){};
};