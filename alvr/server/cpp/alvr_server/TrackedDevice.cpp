#include "TrackedDevice.h"
#include "Logger.h"

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
