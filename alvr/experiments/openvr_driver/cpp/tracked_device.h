#pragma once

#include "bindings.h"
#include "openvr_driver.h"

class TrackedDevice : public vr::ITrackedDeviceServerDriver {
    virtual vr::EVRInitError Activate(uint32_t object_id) override;
    virtual void *GetComponent(const char *component_name_and_version) override { return nullptr; }
    virtual void Deactivate() override {}
    virtual void EnterStandby() override {}
    virtual void DebugRequest(const char *request,
                              char *response_buffer,
                              uint32_t response_buffer_size) override {}
    virtual vr::DriverPose_t GetPose() override { return this->pose; }

  protected:
    uint64_t device_index;
    vr::TrackedDeviceIndex_t object_id;
    vr::PropertyContainerHandle_t property_container;
    vr::DriverPose_t pose;

    virtual void activate_inner() {}

    TrackedDevice(uint64_t device_index) : device_index(device_index) {
        this->pose.result = vr::TrackingResult_Uninitialized;
    }

  public:
    void set_motion(MotionData motion, double time_offset_s);
    vr::PropertyContainerHandle_t get_container() { return this->property_container; }
};
