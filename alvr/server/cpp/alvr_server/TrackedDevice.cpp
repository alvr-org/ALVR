#include "TrackedDevice.h"
#include "Logger.h"
#include "Utils.h"

TrackedDevice::TrackedDevice(uint64_t device_id) : device_id(device_id) {
    pose = vr::DriverPose_t{};
    pose.poseIsValid = false;
    pose.deviceIsConnected = false;
    pose.result = vr::TrackingResult_Uninitialized;

    pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
    pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
    pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);
}

bool TrackedDevice::register_device() {
    auto size = GetSerialNumber(this->device_id, nullptr);

    auto serial_number = std::vector<char>(size);
    GetSerialNumber(this->device_id, &serial_number[0]);

    vr::VRServerDriverHost()->TrackedDeviceAdded(&serial_number[0], device_class(), this);

    return true;
}

void TrackedDevice::set_prop(FfiOpenvrProperty prop) {
    if (this->object_id == vr::k_unTrackedDeviceIndexInvalid) {
        return;
    }

    auto key = (vr::ETrackedDeviceProperty)prop.key;

    auto props = vr::VRProperties();

    vr::ETrackedPropertyError result;

    if (prop.type == FfiOpenvrPropertyType::Bool) {
        result = props->SetBoolProperty(this->prop_container, key, prop.value.bool_);
    } else if (prop.type == FfiOpenvrPropertyType::Float) {
        result = props->SetFloatProperty(this->prop_container, key, prop.value.float_);
    } else if (prop.type == FfiOpenvrPropertyType::Int32) {
        result = props->SetInt32Property(this->prop_container, key, prop.value.int32);
    } else if (prop.type == FfiOpenvrPropertyType::Uint64) {
        result = props->SetUint64Property(this->prop_container, key, prop.value.uint64);
    } else if (prop.type == FfiOpenvrPropertyType::Vector3) {
        auto vec3 = vr::HmdVector3_t{};
        vec3.v[0] = prop.value.vector3[0];
        vec3.v[1] = prop.value.vector3[1];
        vec3.v[2] = prop.value.vector3[2];
        result = props->SetVec3Property(this->prop_container, key, vec3);
    } else if (prop.type == FfiOpenvrPropertyType::Double) {
        result = props->SetDoubleProperty(this->prop_container, key, prop.value.double_);
    } else if (prop.type == FfiOpenvrPropertyType::String) {
        result = props->SetStringProperty(this->prop_container, key, prop.value.string);
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

bool TrackedDevice::set_motion(FfiDeviceMotion motion) {
    auto pose = vr::DriverPose_t{};

    pose.poseIsValid = motion.is_tracked;
    pose.deviceIsConnected = motion.is_tracked;
    pose.result =
        motion.is_tracked ? vr::TrackingResult_Running_OK : vr::TrackingResult_Uninitialized;

    pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
    pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);

    pose.qRotation = HmdQuaternion_Init(
        motion.orientation.w, motion.orientation.x, motion.orientation.y, motion.orientation.z);

    pose.vecPosition[0] = motion.position[0];
    pose.vecPosition[1] = motion.position[1];
    pose.vecPosition[2] = motion.position[2];

    pose.vecVelocity[0] = motion.linear_velocity[0];
    pose.vecVelocity[1] = motion.linear_velocity[1];
    pose.vecVelocity[2] = motion.linear_velocity[2];

    pose.vecAngularVelocity[0] = motion.angular_velocity[0];
    pose.vecAngularVelocity[1] = motion.angular_velocity[1];
    pose.vecAngularVelocity[2] = motion.angular_velocity[2];

    pose.poseTimeOffset = motion.prediction_s;

    this->pose = pose;

    if (this->object_id != vr::k_unTrackedDeviceIndexInvalid) {
        vr::VRServerDriverHost()->TrackedDevicePoseUpdated(
            this->object_id, pose, sizeof(vr::DriverPose_t));

        return true;
    } else {
        return false;
    }
}

vr::EVRInitError TrackedDevice::Activate(vr::TrackedDeviceIndex_t object_id) {
    this->object_id = object_id;
    this->prop_container = vr::VRProperties()->TrackedDeviceToPropertyContainer(object_id);

    SetOpenvrProps(object_id);
}

void TrackedDevice::Deactivate() {
    this->object_id = vr::k_unTrackedDeviceIndexInvalid;
    this->prop_container = vr::k_ulInvalidPropertyContainer;
}

void TrackedDevice::DebugRequest(const char *request,
                                 char *resp_buffer,
                                 uint32_t resp_buffer_size) {
    if (resp_buffer_size >= 1)
        resp_buffer[0] = 0;
}