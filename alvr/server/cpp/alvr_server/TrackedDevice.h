#pragma once

#include "bindings.h"
#include "openvr_driver.h"
#include <map>

class TrackedDevice : public vr::ITrackedDeviceServerDriver {
  public:
    uint64_t device_path;
    vr::TrackedDeviceIndex_t object_id = vr::k_unTrackedDeviceIndexInvalid;
    vr::PropertyContainerHandle_t prop_container = vr::k_ulInvalidPropertyContainer;

    vr::DriverPose_t pose;

    void set_prop(OpenvrProperty prop);

    void clear_pose();

    TrackedDevice(uint64_t device_path) : device_path(device_path) { clear_pose(); }
};