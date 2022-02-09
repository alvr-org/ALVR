#include "tracked_device.h"

void TrackedDevice::set_prop(AlvrOpenvrProp prop) {
    vr::ETrackedPropertyError result;

    auto key = (vr::ETrackedDeviceProperty)prop.key;

    if (prop.value.tag == ALVR_OPENVR_PROP_VALUE_BOOL) {
        result = vr::VRProperties()->SetBoolProperty(this->prop_container, key, prop.value.bool_);
    } else if (prop.value.tag == ALVR_OPENVR_PROP_VALUE_FLOAT) {
        result = vr::VRProperties()->SetFloatProperty(this->prop_container, key, prop.value.float_);
    } else if (prop.value.tag == ALVR_OPENVR_PROP_VALUE_INT32) {
        result = vr::VRProperties()->SetInt32Property(this->prop_container, key, prop.value.int32);
    } else if (prop.value.tag == ALVR_OPENVR_PROP_VALUE_UINT64) {
        result =
            vr::VRProperties()->SetUint64Property(this->prop_container, key, prop.value.uint64);
    } else if (prop.value.tag == ALVR_OPENVR_PROP_VALUE_VECTOR3) {
        auto vec3 = vr::HmdVector3_t{};
        vec3.v[0] = prop.value.vector3[0];
        vec3.v[1] = prop.value.vector3[1];
        vec3.v[2] = prop.value.vector3[2];
        result = vr::VRProperties()->SetVec3Property(this->prop_container, key, vec3);
    } else if (prop.value.tag == ALVR_OPENVR_PROP_VALUE_DOUBLE) {
        result =
            vr::VRProperties()->SetDoubleProperty(this->prop_container, key, prop.value.double_);
    } else if (prop.value.tag == ALVR_OPENVR_PROP_VALUE_STRING) {
        result =
            vr::VRProperties()->SetStringProperty(this->prop_container, key, prop.value.string);
    } else {
        alvr_popup_error("Unreachable");
        result = vr::TrackedProp_Success;
    }

    if (result != vr::TrackedProp_Success) {
        auto error_message = std::string("Error setting property ") + std::to_string(prop.key) +
                             ": " + vr::VRPropertiesRaw()->GetPropErrorNameFromEnum(result);
        alvr_error(error_message.c_str());
    }

    auto event_data = vr::VREvent_Data_t{};
    event_data.property.container = this->prop_container;
    event_data.property.prop = key;
    vr::VRServerDriverHost()->VendorSpecificEvent(
        this->object_id, vr::VREvent_PropertyChanged, event_data, 0.);
}

void TrackedDevice::set_static_props() {
    auto props_count = alvr_get_static_openvr_properties(this->device_path, nullptr);

    if (props_count > 0) {
        auto props = std::vector<AlvrOpenvrProp>(props_count);
        alvr_get_static_openvr_properties(device_path, &props[0]);

        for (auto prop : props) {
            this->set_prop(prop);
        }
    }
}

void TrackedDevice::update_pose(AlvrMotionData motion, uint64_t timestamp_ns) {
    this->pose.vecPosition[0] = motion.position[0];
    this->pose.vecPosition[1] = motion.position[1];
    this->pose.vecPosition[2] = motion.position[2];

    this->pose.qRotation.w = motion.orientation.w;
    this->pose.qRotation.x = motion.orientation.x;
    this->pose.qRotation.y = motion.orientation.y;
    this->pose.qRotation.z = motion.orientation.z;

    if (motion.has_velocity) {
        this->pose.vecVelocity[0] = motion.linear_velocity[0];
        this->pose.vecVelocity[1] = motion.linear_velocity[1];
        this->pose.vecVelocity[2] = motion.linear_velocity[2];

        this->pose.vecAngularVelocity[0] = motion.angular_velocity[0];
        this->pose.vecAngularVelocity[1] = motion.angular_velocity[1];
        this->pose.vecAngularVelocity[2] = motion.angular_velocity[2];
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

void TrackedDevice::clear_pose() {
    auto pose = vr::DriverPose_t{};

    pose.qWorldFromDriverRotation = vr::HmdQuaternion_t{1.0, 0.0, 0.0, 0.0};
    pose.qDriverFromHeadRotation = vr::HmdQuaternion_t{1.0, 0.0, 0.0, 0.0};

    pose.result = vr::TrackingResult_Uninitialized;
    pose.poseIsValid = false;
    pose.deviceIsConnected = false;

    this->pose = pose;
}