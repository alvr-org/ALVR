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

    virtual vr::EVRInitError Activate(uint32_t id) override {
        this->object_id = id;
        this->prop_container = vr::VRProperties()->TrackedDeviceToPropertyContainer(id);

        return vr::VRInitError_None;
    }
    virtual void *GetComponent(const char *component_name_and_version) override { return nullptr; }
    virtual void Deactivate() override {}
    virtual void EnterStandby() override {}
    virtual void DebugRequest(const char *request,
                              char *response_buffer,
                              uint32_t response_buffer_size) override {}
    virtual vr::DriverPose_t GetPose() override { return this->pose; }

    void update_pose(AlvrMotionData motion, uint64_t timestamp_ns) {
        this->pose.vecPosition[0] = motion.position.x;
        this->pose.vecPosition[1] = motion.position.y;
        this->pose.vecPosition[2] = motion.position.z;

        this->pose.qRotation.w = motion.orientation.w;
        this->pose.qRotation.x = motion.orientation.x;
        this->pose.qRotation.y = motion.orientation.y;
        this->pose.qRotation.z = motion.orientation.z;

        if (motion.has_velocity) {
            this->pose.vecVelocity[0] = motion.linear_velocity.x;
            this->pose.vecVelocity[1] = motion.linear_velocity.y;
            this->pose.vecVelocity[2] = motion.linear_velocity.z;

            this->pose.vecAngularVelocity[0] = motion.angular_velocity.x;
            this->pose.vecAngularVelocity[1] = motion.angular_velocity.y;
            this->pose.vecAngularVelocity[2] = motion.angular_velocity.z;
        }

        this->pose.result = vr::TrackingResult_Running_OK;
        this->pose.poseIsValid = true;
        this->pose.deviceIsConnected = true;

        // Note: poseTimeOffset is usually negative
        this->pose.poseTimeOffset =
            (float)(alvr_get_best_effort_client_time_ns(this->device_path) - timestamp_ns) /
            1'000'000'000;

        vr::VRServerDriverHost()->TrackedDevicePoseUpdated(
            this->object_id, this->pose, sizeof(vr::DriverPose_t));
    }

    void clear_pose() {
        auto pose = vr::DriverPose_t{};

        pose.qWorldFromDriverRotation = vr::HmdQuaternion_t{1.0, 0.0, 0.0, 0.0};
        pose.qDriverFromHeadRotation = vr::HmdQuaternion_t{1.0, 0.0, 0.0, 0.0};

        pose.result = vr::TrackingResult_Uninitialized;
        pose.poseIsValid = false;
        pose.deviceIsConnected = false;

        this->pose = pose;
    }

    TrackedDevice(uint64_t device_path) : device_path(device_path) { clear_pose(); }
};

void set_static_properties(uint64_t device_path, vr::PropertyContainerHandle_t container) {
    // auto props_count = alvr_get_static_openvr_properties(device_path, nullptr);

    // auto props = std::vector<AlvrOpenvrProp>(props_count);

    // alvr_get_static_openvr_properties(device_path, &props[0]);

    // for (auto prop : props) {
    //     if (prop.ty == AlvrOpenvrPropType::Bool) {
    //         // vr::VRProperties()->SetBoolProperty(container, )
    //     }
    //     // todo: generate prop map function
    // }
}