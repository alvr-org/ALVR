#include "TrackedDevice.h"
#include "Logger.h"

void TrackedDevice::set_prop(OpenvrProperty prop) {
    auto key = (vr::ETrackedDeviceProperty)prop.key;

    vr::ETrackedPropertyError result;

    if (prop.type == OpenvrPropertyType::Bool) {
        result = vr::VRProperties()->SetBoolProperty(this->prop_container, key, prop.value.bool_);
    } else if (prop.type == OpenvrPropertyType::Float) {
        result = vr::VRProperties()->SetFloatProperty(this->prop_container, key, prop.value.float_);
    } else if (prop.type == OpenvrPropertyType::Int32) {
        result = vr::VRProperties()->SetInt32Property(this->prop_container, key, prop.value.int32);
    } else if (prop.type == OpenvrPropertyType::Uint64) {
        result =
            vr::VRProperties()->SetUint64Property(this->prop_container, key, prop.value.uint64);
    } else if (prop.type == OpenvrPropertyType::Vector3) {
        auto vec3 = vr::HmdVector3_t{};
        vec3.v[0] = prop.value.vector3[0];
        vec3.v[1] = prop.value.vector3[1];
        vec3.v[2] = prop.value.vector3[2];
        result = vr::VRProperties()->SetVec3Property(this->prop_container, key, vec3);
    } else if (prop.type == OpenvrPropertyType::Double) {
        result =
            vr::VRProperties()->SetDoubleProperty(this->prop_container, key, prop.value.double_);
    } else if (prop.type == OpenvrPropertyType::String) {
        result =
            vr::VRProperties()->SetStringProperty(this->prop_container, key, prop.value.string);
    } else {
        Error("Unreachable");
        result = vr::TrackedProp_Success;
    }

    if (result != vr::TrackedProp_Success) {
        Error("Error setting property %d: %s",
              key,
              vr::VRPropertiesRaw()->GetPropErrorNameFromEnum(result));
    }

    auto event_data = vr::VREvent_Data_t{};
    event_data.property.container = this->prop_container;
    event_data.property.prop = key;
    vr::VRServerDriverHost()->VendorSpecificEvent(
        this->object_id, vr::VREvent_PropertyChanged, event_data, 0.);
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