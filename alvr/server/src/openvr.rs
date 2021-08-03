use alvr_common::prelude::*;
use alvr_session::OpenvrPropValue;
use openvr_driver_sys as vr;
use std::ffi::CString;

fn set_property(
    device_index: vr::TrackedDeviceIndex_t,
    key: vr::ETrackedDeviceProperty,
    value: OpenvrPropValue,
) -> vr::ETrackedPropertyError {
    unsafe {
        let container_handle = vr::vrTrackedDeviceToPropertyContainer(device_index);

        let res = match value {
            OpenvrPropValue::Bool(value) => vr::vrSetBoolProperty(container_handle, key, value),
            OpenvrPropValue::Float(value) => vr::vrSetFloatProperty(container_handle, key, value),
            OpenvrPropValue::Int32(value) => vr::vrSetInt32Property(container_handle, key, value),
            OpenvrPropValue::Uint64(value) => vr::vrSetUint64Property(container_handle, key, value),
            OpenvrPropValue::Vector3(value) => {
                vr::vrSetVec3Property(container_handle, key, &vr::HmdVector3_t { v: value })
            }
            OpenvrPropValue::Double(value) => vr::vrSetDoubleProperty(container_handle, key, value),
            OpenvrPropValue::String(value) => {
                // unwrap never fails
                let c_string = CString::new(value).unwrap();
                vr::vrSetStringProperty(container_handle, key, c_string.as_ptr())
            }
        };

        if res != vr::TrackedProp_Success {
            return res;
        }

        let event_data = vr::VREvent_Data_t {
            property: vr::VREvent_Property_t {
                container: container_handle,
                prop: vr::Prop_Audio_DefaultPlaybackDeviceId_String,
            },
        };
        vr::vrServerDriverHostVendorSpecificEvent(
            device_index,
            vr::VREvent_PropertyChanged,
            &event_data,
            0.,
        );

        vr::TrackedProp_Success
    }
}

fn set_custom_properties(
    device_index: vr::TrackedDeviceIndex_t,
    properties: Vec<(String, OpenvrPropValue)>,
) -> StrResult {
    for (name, value) in properties {
        let key = vr::tracked_device_property_name_to_key(&name)?;

        let res = set_property(device_index, key, value);
        if res != vr::TrackedProp_Success {
            return fmt_e!("Failed to set OpenVR property {} with code={}", name, res);
        }
    }

    Ok(())
}

pub fn set_game_output_audio_device_id(id: String) {
    set_property(
        vr::k_unTrackedDeviceIndex_Hmd,
        vr::Prop_Audio_DefaultPlaybackDeviceId_String,
        OpenvrPropValue::String(id),
    );
}

pub fn set_headset_microphone_audio_device_id(id: String) {
    set_property(
        vr::k_unTrackedDeviceIndex_Hmd,
        vr::Prop_Audio_DefaultRecordingDeviceId_String,
        OpenvrPropValue::String(id),
    );
}
