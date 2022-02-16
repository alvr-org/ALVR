#pragma once

#include "bindings.h"
#include "openvr_driver.h"
#include <map>

class TrackedDevice {
  public:
    uint64_t device_path;
    vr::TrackedDeviceIndex_t object_id = vr::k_unTrackedDeviceIndexInvalid;
    vr::PropertyContainerHandle_t prop_container = vr::k_ulInvalidPropertyContainer;

    void set_prop(OpenvrProperty prop);

    TrackedDevice(uint64_t device_path) : device_path(device_path) {}
};