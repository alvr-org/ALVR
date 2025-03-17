#include "TrackedDevice.h"
#include "Logger.h"
#include "Utils.h"
#include <chrono>
#include <thread>

TrackedDevice::TrackedDevice(uint64_t device_id, vr::ETrackedDeviceClass device_class)
    : device_id(device_id)
    , device_class(device_class) {
    this->last_pose = vr::DriverPose_t {};
    this->last_pose.poseIsValid = false;
    this->last_pose.deviceIsConnected = false;
    this->last_pose.result = vr::TrackingResult_Uninitialized;

    this->last_pose.qDriverFromHeadRotation = HmdQuaternion_Init(1, 0, 0, 0);
    this->last_pose.qWorldFromDriverRotation = HmdQuaternion_Init(1, 0, 0, 0);
    this->last_pose.qRotation = HmdQuaternion_Init(1, 0, 0, 0);
}

std::string TrackedDevice::get_serial_number() {
    auto size = GetSerialNumber(this->device_id, nullptr);

    auto buffer = std::vector<char>(size);
    GetSerialNumber(this->device_id, &buffer[0]);

    return std::string(&buffer[0]);
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
        auto vec3 = vr::HmdVector3_t {};
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
        Error(
            "Error setting property %d: %s",
            key,
            vr::VRPropertiesRaw()->GetPropErrorNameFromEnum(result)
        );
    }

    auto event_data = vr::VREvent_Data_t {};
    event_data.property.container = this->prop_container;
    event_data.property.prop = key;
    vr::VRServerDriverHost()->VendorSpecificEvent(
        this->object_id, vr::VREvent_PropertyChanged, event_data, 0.
    );
}

void TrackedDevice::submit_pose(vr::DriverPose_t pose) {
    this->last_pose = pose;
    vr::VRServerDriverHost()->TrackedDevicePoseUpdated(
        this->object_id, pose, sizeof(vr::DriverPose_t)
    );
}

bool TrackedDevice::register_device() {
    if (!vr::VRServerDriverHost()->TrackedDeviceAdded(
            this->get_serial_number().c_str(),
            this->device_class,
            (vr::ITrackedDeviceServerDriver*)this
        )) {
        Error("Failed to register device (%s)", this->get_serial_number().c_str());

        return false;
    }

    auto lock = std::unique_lock<std::mutex>(this->activation_mutex);
    this->activation_condvar.wait_for(lock, std::chrono::seconds(1), [this] {
        return this->activation_state != ActivationState::Pending;
    });

    return this->activation_state == ActivationState::Success;
}

vr::EVRInitError TrackedDevice::Activate(vr::TrackedDeviceIndex_t object_id) {
    this->object_id = object_id;
    this->prop_container = vr::VRProperties()->TrackedDeviceToPropertyContainer(this->object_id);

    {
        auto guard = std::lock_guard<std::mutex>(this->activation_mutex);

        if (this->activate()) {
            this->activation_state = ActivationState::Success;
        } else {
            this->activation_state = ActivationState::Failure;
        }
    }
    this->activation_condvar.notify_one();

    return this->activation_state == ActivationState::Success ? vr::VRInitError_None
                                                              : vr::VRInitError_Driver_Failed;
}
