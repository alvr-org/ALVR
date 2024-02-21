#pragma once

#include "bindings.h"
#include "openvr_driver.h"
#include <map>

class TrackedDevice : public vr::ITrackedDeviceServerDriver {
  public:
    uint64_t device_id;
    vr::TrackedDeviceIndex_t object_id = vr::k_unTrackedDeviceIndexInvalid;
    vr::PropertyContainerHandle_t prop_container = vr::k_ulInvalidPropertyContainer;
    vr::DriverPose_t pose;

    TrackedDevice(uint64_t device_id);

    bool register_device();

    void set_prop(FfiOpenvrProperty prop);

    bool set_motion(FfiDeviceMotion motion);

    virtual vr::ETrackedDeviceClass device_class() = 0;

    //
    // ITrackedDeviceServerDriver
    //

    // To be called at the beginning of Activate override
    virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t object_id);

    virtual void Deactivate();

    virtual void EnterStandby(){};

    virtual void *GetComponent(const char *comp_name_and_version) { return nullptr; }

    virtual void DebugRequest(const char *request, char *resp_buffer, uint32_t resp_buffer_size);

    vr::DriverPose_t GetPose() final { return pose; }
};