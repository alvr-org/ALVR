#pragma once

#include "bindings.h"
#include "openvr_driver.h"
#include "tracked_device.h"

class Controller : public TrackedDevice {
    vr::ETrackedControllerRole role;

  public:
    Controller(uint64_t device_index, vr::ETrackedControllerRole role)
        : TrackedDevice(device_index), role(role) {}
};
