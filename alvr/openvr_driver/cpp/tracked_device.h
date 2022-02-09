#pragma once

#include "alvr_streamer.h"
#include "openvr_driver.h"
#include <map>

class TrackedDevice : public vr::ITrackedDeviceServerDriver {
  public:
    uint64_t device_path;
    vr::TrackedDeviceIndex_t object_id = vr::k_unTrackedDeviceIndexInvalid;
    vr::PropertyContainerHandle_t prop_container = vr::k_ulInvalidPropertyContainer;
    vr::DriverPose_t pose;

    virtual void *GetComponent(const char *component_name_and_version) override { return nullptr; }
    virtual void Deactivate() override {
        this->object_id = vr::k_unTrackedDeviceIndexInvalid;
        this->prop_container = vr::k_ulInvalidPropertyContainer;
    }
    virtual void EnterStandby() override {}
    virtual void DebugRequest(const char *request,
                              char *response_buffer,
                              uint32_t response_buffer_size) override {}
    virtual vr::DriverPose_t GetPose() override { return this->pose; }

    void set_prop(AlvrOpenvrProp prop);

    // Properties that are set by the user in the dashboard. This should be called last in Activate
    void set_static_props();

    void update_pose(AlvrMotionData motion, uint64_t timestamp_ns);

    void clear_pose();

    TrackedDevice(uint64_t device_path) : device_path(device_path) { clear_pose(); }
};