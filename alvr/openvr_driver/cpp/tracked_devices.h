#pragma once

extern "C" {
#include "alvr_streamer.h"
}
#include "openvr_driver.h"
#include "openvr_properties_mapping.h"
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

    void set_prop(AlvrOpenvrProp prop) {
        auto key = tracked_device_property_name_to_key(prop.name);
        vr::ETrackedPropertyError result;

        if (prop.ty == ALVR_OPENVR_PROP_TYPE_BOOL) {
            result =
                vr::VRProperties()->SetBoolProperty(this->prop_container, key, prop.value.bool_);
        } else if (prop.ty == ALVR_OPENVR_PROP_TYPE_FLOAT) {
            result =
                vr::VRProperties()->SetFloatProperty(this->prop_container, key, prop.value.float_);
        } else if (prop.ty == ALVR_OPENVR_PROP_TYPE_INT32) {
            result =
                vr::VRProperties()->SetInt32Property(this->prop_container, key, prop.value.int32);
        } else if (prop.ty == ALVR_OPENVR_PROP_TYPE_UINT64) {
            result =
                vr::VRProperties()->SetUint64Property(this->prop_container, key, prop.value.uint64);
        } else if (prop.ty == ALVR_OPENVR_PROP_TYPE_VECTOR3) {
            auto vec3 = vr::HmdVector3_t{};
            vec3.v[0] = prop.value.vector3[0];
            vec3.v[1] = prop.value.vector3[1];
            vec3.v[2] = prop.value.vector3[2];
            result = vr::VRProperties()->SetVec3Property(this->prop_container, key, vec3);
        } else if (prop.ty == ALVR_OPENVR_PROP_TYPE_DOUBLE) {
            result = vr::VRProperties()->SetDoubleProperty(
                this->prop_container, key, prop.value.double_);
        } else if (prop.ty == ALVR_OPENVR_PROP_TYPE_STRING) {
            result =
                vr::VRProperties()->SetStringProperty(this->prop_container, key, prop.value.string);
        } else {
            alvr_popup_error("Unreachable");
            result = vr::TrackedProp_Success;
        }

        if (result != vr::TrackedProp_Success) {
            auto error_message = std::string("Error setting property") + prop.name + ": " +
                                 vr::VRPropertiesRaw()->GetPropErrorNameFromEnum(result);
            alvr_error(error_message.c_str());
        }
    }

    // Properties that are set by the user in the dashboard. This should be called last in Activate
    void set_static_props() {
        auto props_count = alvr_get_static_openvr_properties(this->device_path, nullptr);

        if (props_count > 0) {
            auto props = std::vector<AlvrOpenvrProp>(props_count);
            alvr_get_static_openvr_properties(device_path, &props[0]);

            for (auto prop : props) {
                this->set_prop(prop);
            }
        }
    }

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