#pragma once

#include "bindings.h"
#include "openvr_driver_wrap.h"
#include <condition_variable>
#include <map>
#include <mutex>
#include <optional>

enum class ActivationState {
    Pending,
    Success,
    Failure,
};

class TrackedDevice : vr::ITrackedDeviceServerDriver {
public:
    vr::TrackedDeviceIndex_t object_id = vr::k_unTrackedDeviceIndexInvalid;
    vr::PropertyContainerHandle_t prop_container = vr::k_ulInvalidPropertyContainer;
    vr::DriverPose_t last_pose;

    bool register_device();
    void set_prop(FfiOpenvrProperty prop);

protected:
    uint64_t device_id;
    vr::ETrackedDeviceClass device_class;

    TrackedDevice(uint64_t device_id, vr::ETrackedDeviceClass device_class);
    std::string get_serial_number();
    void submit_pose(vr::DriverPose_t pose);
    virtual bool activate() = 0;
    virtual void* get_component(const char*) = 0;

private:
    ActivationState activation_state = ActivationState::Pending;
    std::mutex activation_mutex = {};
    std::condition_variable activation_condvar = {};

    // ITrackedDeviceServerDriver
    vr::EVRInitError Activate(vr::TrackedDeviceIndex_t object_id) final;
    void Deactivate() final {
        this->device_id = vr::k_unTrackedDeviceIndexInvalid;
        this->prop_container = vr::k_ulInvalidPropertyContainer;
    }
    void EnterStandby() final { }
    void* GetComponent(const char* component_name_and_version) final {
        return get_component(component_name_and_version);
    }
    void DebugRequest(const char*, char* buffer, uint32_t buffer_size) final {
        if (buffer_size >= 1)
            buffer[0] = 0;
    }
    vr::DriverPose_t GetPose() final { return this->last_pose; }
};
