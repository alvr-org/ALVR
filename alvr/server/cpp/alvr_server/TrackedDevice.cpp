#include "TrackedDevice.h"
#include "Logger.h"

void TrackedDevice::set_prop(FfiOpenvrProperty prop) {
    auto key = (vr::ETrackedDeviceProperty)prop.key;

    auto vr_properties = vr::VRProperties();

    vr::ETrackedPropertyError result;

    if (prop.type == FfiOpenvrPropertyType::Bool) {
        result = vr_properties->SetBoolProperty(this->prop_container, key, prop.value.bool_);
    } else if (prop.type == FfiOpenvrPropertyType::Float) {
        result = vr_properties->SetFloatProperty(this->prop_container, key, prop.value.float_);
    } else if (prop.type == FfiOpenvrPropertyType::Int32) {
        result = vr_properties->SetInt32Property(this->prop_container, key, prop.value.int32);
    } else if (prop.type == FfiOpenvrPropertyType::Uint64) {
        result =
            vr_properties->SetUint64Property(this->prop_container, key, prop.value.uint64);
    } else if (prop.type == FfiOpenvrPropertyType::Vector3) {
        auto vec3 = vr::HmdVector3_t{};
        vec3.v[0] = prop.value.vector3[0];
        vec3.v[1] = prop.value.vector3[1];
        vec3.v[2] = prop.value.vector3[2];
        result = vr_properties->SetVec3Property(this->prop_container, key, vec3);
    } else if (prop.type == FfiOpenvrPropertyType::Double) {
        result =
            vr_properties->SetDoubleProperty(this->prop_container, key, prop.value.double_);
    } else if (prop.type == FfiOpenvrPropertyType::String) {
        result =
            vr_properties->SetStringProperty(this->prop_container, key, prop.value.string);
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