#pragma once

#include "bindings.h"
#include "openvr_driver.h"
#include <map>

class TrackedDevice {
  public:
    uint64_t device_id;
    vr::TrackedDeviceIndex_t object_id = vr::k_unTrackedDeviceIndexInvalid;
    vr::PropertyContainerHandle_t prop_container = vr::k_ulInvalidPropertyContainer;

    TrackedDevice(uint64_t device_id) : device_id(device_id) {}

    std::string get_serial_number();

    void set_prop(FfiOpenvrProperty prop);
};