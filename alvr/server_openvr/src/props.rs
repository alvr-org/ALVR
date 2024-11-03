// Note: many properties are missing or are stubs.
// todo: fill out more properties for headset and controllers
// todo: add more emulation modes

use crate::{
    FfiOpenvrProperty, FfiOpenvrPropertyType_Bool, FfiOpenvrPropertyType_Double,
    FfiOpenvrPropertyType_Float, FfiOpenvrPropertyType_Int32, FfiOpenvrPropertyType_String,
    FfiOpenvrPropertyType_Uint64, FfiOpenvrPropertyType_Vector3, FfiOpenvrPropertyValue,
};
use alvr_common::{
    debug, settings_schema::Switch, warn, BODY_CHEST_ID, BODY_HIPS_ID, BODY_LEFT_ELBOW_ID,
    BODY_LEFT_FOOT_ID, BODY_LEFT_KNEE_ID, BODY_RIGHT_ELBOW_ID, BODY_RIGHT_FOOT_ID,
    BODY_RIGHT_KNEE_ID, DEVICE_ID_TO_PATH, HAND_LEFT_ID, HAND_RIGHT_ID, HAND_TRACKER_LEFT_ID,
    HAND_TRACKER_RIGHT_ID, HEAD_ID,
};
use alvr_session::{
    ControllersEmulationMode, HeadsetEmulationMode, OpenvrPropKey, OpenvrPropType, OpenvrProperty,
};
use std::{
    ffi::{c_char, CString},
    ptr,
};

pub fn set_openvr_prop(device_id: u64, prop: OpenvrProperty) {
    let key = prop.key as u32;
    let ty = alvr_session::openvr_prop_key_to_type(prop.key);
    let value = prop.value.clone();

    let device_name = DEVICE_ID_TO_PATH.get(&device_id).unwrap_or(&"Unknown");

    let (type_, maybe_value) = match ty {
        OpenvrPropType::Bool => (
            FfiOpenvrPropertyType_Bool,
            value
                .parse::<bool>()
                .ok()
                .map(|bool_| FfiOpenvrPropertyValue {
                    bool_: bool_.into(),
                }),
        ),
        OpenvrPropType::Float => (
            FfiOpenvrPropertyType_Float,
            value
                .parse::<f32>()
                .ok()
                .map(|float_| FfiOpenvrPropertyValue { float_ }),
        ),
        OpenvrPropType::Int32 => (
            FfiOpenvrPropertyType_Int32,
            value
                .parse::<i32>()
                .ok()
                .map(|int32| FfiOpenvrPropertyValue { int32 }),
        ),
        OpenvrPropType::Uint64 => (
            FfiOpenvrPropertyType_Uint64,
            value
                .parse::<u64>()
                .ok()
                .map(|uint64| FfiOpenvrPropertyValue { uint64 }),
        ),
        OpenvrPropType::Vector3 => (
            FfiOpenvrPropertyType_Vector3,
            serde_json::from_str::<[f32; 3]>(&value)
                .ok()
                .map(|vector3| FfiOpenvrPropertyValue { vector3 }),
        ),
        OpenvrPropType::Double => (
            FfiOpenvrPropertyType_Double,
            value
                .parse::<f64>()
                .ok()
                .map(|double_| FfiOpenvrPropertyValue { double_ }),
        ),
        OpenvrPropType::String => {
            let c_string = CString::new(value.clone()).unwrap();
            let mut string = [0; 256];

            unsafe {
                ptr::copy_nonoverlapping(
                    c_string.as_ptr(),
                    string.as_mut_ptr(),
                    c_string.as_bytes_with_nul().len(),
                );
            }

            (
                FfiOpenvrPropertyType_String,
                Some(FfiOpenvrPropertyValue { string }),
            )
        }
    };

    let Some(ffi_value) = maybe_value else {
        warn!("Failed to parse {device_name} value for OpenVR property: {key:?}={value}");

        return;
    };

    debug!("Setting {device_name} OpenVR prop: {key:?}={value}");

    unsafe {
        crate::SetOpenvrProperty(
            *HEAD_ID,
            FfiOpenvrProperty {
                key,
                type_,
                value: ffi_value,
            },
        );
    }
}

fn serial_number(device_id: u64) -> String {
    let settings = alvr_server_core::settings();

    if device_id == *HEAD_ID {
        match &settings.headset.emulation_mode {
            HeadsetEmulationMode::RiftS => "1WMGH000XX0000".into(),
            HeadsetEmulationMode::Quest2 => "1WMHH000X00000".into(),
            HeadsetEmulationMode::Vive => "HTCVive-001".into(),
            HeadsetEmulationMode::Custom { serial_number, .. } => serial_number.clone(),
        }
    } else if device_id == *HAND_LEFT_ID || device_id == *HAND_RIGHT_ID {
        if let Switch::Enabled(controllers) = &settings.headset.controllers {
            let serial_number = match &controllers.emulation_mode {
                ControllersEmulationMode::RiftSTouch => "ALVR Remote Controller",
                ControllersEmulationMode::Quest2Touch => "1WMHH000X00000_Controller",
                ControllersEmulationMode::Quest3Plus => "2G0YZX0X0000XX_Controller",
                ControllersEmulationMode::ValveIndex => "ALVR Remote Controller",
                ControllersEmulationMode::ViveWand => "ALVR Remote Controller",
                ControllersEmulationMode::ViveTracker => "ALVR Remote Controller",
                ControllersEmulationMode::Custom { serial_number, .. } => serial_number,
            };

            if device_id == *HAND_LEFT_ID {
                format!("{serial_number}_Left")
            } else {
                format!("{serial_number}_Right")
            }
        } else {
            "Unknown".into()
        }
    } else if device_id == *HAND_TRACKER_LEFT_ID {
        "ALVR_Left_Hand_Full_Skeletal".into()
    } else if device_id == *HAND_TRACKER_RIGHT_ID {
        "ALVR_Right_Hand_Full_Skeletal".into()
    } else if device_id == *BODY_CHEST_ID {
        "ALVR Tracker (chest)".into()
    } else if device_id == *BODY_HIPS_ID {
        "ALVR Tracker (waist)".into()
    } else if device_id == *BODY_LEFT_ELBOW_ID {
        "ALVR Tracker (left elbow)".into()
    } else if device_id == *BODY_RIGHT_ELBOW_ID {
        "ALVR Tracker (right elbow)".into()
    } else if device_id == *BODY_LEFT_KNEE_ID {
        "ALVR Tracker (left knee)".into()
    } else if device_id == *BODY_RIGHT_KNEE_ID {
        "ALVR Tracker (right knee)".into()
    } else if device_id == *BODY_LEFT_FOOT_ID {
        "ALVR Tracker (left foot)".into()
    } else if device_id == *BODY_RIGHT_FOOT_ID {
        "ALVR Tracker (right foot)".into()
    } else {
        "Unknown".into()
    }
}

#[no_mangle]
pub extern "C" fn get_serial_number(device_id: u64, out_str: *mut c_char) -> u64 {
    let string = serial_number(device_id);

    let cstring = CString::new(string).unwrap();

    let len = cstring.to_bytes_with_nul().len();

    if !out_str.is_null() {
        unsafe { ptr::copy_nonoverlapping(cstring.as_ptr(), out_str, len) };
    }

    len as u64
}

#[no_mangle]
pub extern "C" fn set_device_openvr_props(device_id: u64) {
    use OpenvrPropKey::*;

    let settings = alvr_server_core::settings();

    let set_prop = |key, value: &str| {
        set_openvr_prop(
            device_id,
            OpenvrProperty {
                key,
                value: value.into(),
            },
        );
    };

    if device_id == *HEAD_ID {
        match &settings.headset.emulation_mode {
            HeadsetEmulationMode::RiftS => {
                set_prop(TrackingSystemNameString, "oculus");
                set_prop(ModelNumberString, "Oculus Rift S");
                set_prop(ManufacturerNameString, "Oculus");
                set_prop(RenderModelNameString, "generic_hmd");
                set_prop(RegisteredDeviceTypeString, "oculus/1WMGH000XX0000");
                set_prop(DriverVersionString, "1.42.0");
                set_prop(
                    NamedIconPathDeviceOffString,
                    "{oculus}/icons/rifts_headset_off.png",
                );
                set_prop(
                    NamedIconPathDeviceSearchingString,
                    "{oculus}/icons/rifts_headset_searching.gif",
                );
                set_prop(
                    NamedIconPathDeviceSearchingAlertString,
                    "{oculus}/icons/rifts_headset_alert_searching.gif",
                );
                set_prop(
                    NamedIconPathDeviceReadyString,
                    "{oculus}/icons/rifts_headset_ready.png",
                );
                set_prop(
                    NamedIconPathDeviceReadyAlertString,
                    "{oculus}/icons/rifts_headset_ready_alert.png",
                );
                set_prop(
                    NamedIconPathDeviceStandbyString,
                    "{oculus}/icons/rifts_headset_standby.png",
                );
            }
            HeadsetEmulationMode::Quest2 => {
                set_prop(TrackingSystemNameString, "oculus");
                set_prop(ModelNumberString, "Miramar");
                set_prop(ManufacturerNameString, "Oculus");
                set_prop(RenderModelNameString, "generic_hmd");
                set_prop(RegisteredDeviceTypeString, "oculus/1WMHH000X00000");
                set_prop(DriverVersionString, "1.55.0");
                set_prop(
                    NamedIconPathDeviceOffString,
                    "{oculus}/icons/quest_headset_off.png",
                );
                set_prop(
                    NamedIconPathDeviceSearchingString,
                    "{oculus}/icons/quest_headset_searching.gif",
                );
                set_prop(
                    NamedIconPathDeviceSearchingAlertString,
                    "{oculus}/icons/quest_headset_alert_searching.gif",
                );
                set_prop(
                    NamedIconPathDeviceReadyString,
                    "{oculus}/icons/quest_headset_ready.png",
                );
                set_prop(
                    NamedIconPathDeviceReadyAlertString,
                    "{oculus}/icons/quest_headset_ready_alert.png",
                );
                set_prop(
                    NamedIconPathDeviceStandbyString,
                    "{oculus}/icons/quest_headset_standby.png",
                );
            }
            HeadsetEmulationMode::Vive => {
                set_prop(TrackingSystemNameString, "Vive Tracker");
                set_prop(ModelNumberString, "ALVR driver server");
                set_prop(ManufacturerNameString, "HTC");
                set_prop(RenderModelNameString, "generic_hmd");
                set_prop(RegisteredDeviceTypeString, "vive");
                set_prop(DriverVersionString, "");
                set_prop(
                    NamedIconPathDeviceOffString,
                    "{htc}/icons/vive_headset_status_off.png",
                );
                set_prop(
                    NamedIconPathDeviceSearchingString,
                    "{htc}/icons/vive_headset_status_searching.gif",
                );
                set_prop(
                    NamedIconPathDeviceSearchingAlertString,
                    "{htc}/icons/vive_headset_status_searching_alert.gif",
                );
                set_prop(
                    NamedIconPathDeviceReadyString,
                    "{htc}/icons/vive_headset_status_ready.png",
                );
                set_prop(
                    NamedIconPathDeviceReadyAlertString,
                    "{htc}/icons/vive_headset_status_ready_alert.png",
                );
                set_prop(
                    NamedIconPathDeviceStandbyString,
                    "{htc}/icons/vive_headset_status_standby.png",
                );
            }
            HeadsetEmulationMode::Custom { .. } => (),
        }

        set_prop(UserIpdMetersFloat, "0.063");
        set_prop(UserHeadToEyeDepthMetersFloat, "0.0");
        set_prop(SecondsFromVsyncToPhotonsFloat, "0.0");

        // return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
        set_prop(CurrentUniverseIdUint64, "2");

        if cfg!(windows) {
            // avoid "not fullscreen" warnings from vrmonitor
            set_prop(IsOnDesktopBool, "false");

            // We let SteamVR handle VSyncs. We just wait in PostPresent().
            set_prop(DriverDirectModeSendsVsyncEventsBool, "false");
        }
        set_prop(DeviceProvidesBatteryStatusBool, "true");
        set_prop(ContainsProximitySensorBool, "true");

        for prop in &settings.headset.extra_openvr_props {
            set_prop(prop.key, &prop.value);
        }
    } else if device_id == *HAND_LEFT_ID
        || device_id == *HAND_RIGHT_ID
        || device_id == *HAND_TRACKER_LEFT_ID
        || device_id == *HAND_TRACKER_RIGHT_ID
    {
        let left_hand = device_id == *HAND_LEFT_ID || device_id == *HAND_TRACKER_LEFT_ID;
        let right_hand = device_id == *HAND_RIGHT_ID || device_id == *HAND_TRACKER_RIGHT_ID;
        if let Switch::Enabled(config) = &settings.headset.controllers {
            match config.emulation_mode {
                ControllersEmulationMode::Quest2Touch => {
                    set_prop(TrackingSystemNameString, "oculus");
                    set_prop(ManufacturerNameString, "Oculus");
                    if left_hand {
                        set_prop(ModelNumberString, "Miramar (Left Controller)");
                        set_prop(RenderModelNameString, "oculus_quest2_controller_left");
                        set_prop(
                            RegisteredDeviceTypeString,
                            "oculus/1WMHH000X00000_Controller_Left",
                        );
                    } else if right_hand {
                        set_prop(ModelNumberString, "Miramar (Right Controller)");
                        set_prop(RenderModelNameString, "oculus_quest2_controller_right");
                        set_prop(
                            RegisteredDeviceTypeString,
                            "oculus/1WMHH000X00000_Controller_Right",
                        );
                    }
                    set_prop(ControllerTypeString, "oculus_touch");
                    set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");

                    if left_hand {
                        set_prop(
                            NamedIconPathDeviceOffString,
                            "{oculus}/icons/rifts_left_controller_off.png",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingString,
                            "{oculus}/icons/rifts_left_controller_searching.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingAlertString,
                            "{oculus}/icons/rifts_left_controller_searching_alert.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyString,
                            "{oculus}/icons/rifts_left_controller_ready.png",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyAlertString,
                            "{oculus}/icons/rifts_left_controller_ready_alert.png",
                        );
                        set_prop(
                            NamedIconPathDeviceAlertLowString,
                            "{oculus}/icons/rifts_left_controller_ready_low.png",
                        );
                    } else if right_hand {
                        set_prop(
                            NamedIconPathDeviceOffString,
                            "{oculus}/icons/rifts_right_controller_off.png",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingString,
                            "{oculus}/icons/rifts_right_controller_searching.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingAlertString,
                            "{oculus}/icons/rifts_right_controller_searching_alert.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyString,
                            "{oculus}/icons/rifts_right_controller_ready.png",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyAlertString,
                            "{oculus}/icons/rifts_right_controller_ready_alert.png",
                        );
                        set_prop(
                            NamedIconPathDeviceAlertLowString,
                            "{oculus}/icons/rifts_right_controller_ready_low.png",
                        );
                    }
                }
                ControllersEmulationMode::Quest3Plus => {
                    set_prop(TrackingSystemNameString, "oculus");
                    set_prop(ManufacturerNameString, "Oculus");

                    if left_hand {
                        set_prop(ModelNumberString, "Meta Quest 3 (Left Controller)");
                        set_prop(RenderModelNameString, "oculus_quest_plus_controller_left");
                        set_prop(
                            RegisteredDeviceTypeString,
                            "oculus/1WMHH000X00000_Controller_Left",
                        );
                    } else if right_hand {
                        set_prop(ModelNumberString, "Meta Quest 3 (Right Controller)");
                        set_prop(RenderModelNameString, "oculus_quest_plus_controller_right");
                        set_prop(
                            RegisteredDeviceTypeString,
                            "oculus/1WMHH000X00000_Controller_Right",
                        );
                    }
                    set_prop(ControllerTypeString, "oculus_touch");
                    set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");

                    if left_hand {
                        set_prop(
                            NamedIconPathDeviceOffString,
                            "{oculus}/icons/rifts_left_controller_off.png",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingString,
                            "{oculus}/icons/rifts_left_controller_searching.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingAlertString,
                            "{oculus}/icons/rifts_left_controller_searching_alert.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyString,
                            "{oculus}/icons/rifts_left_controller_ready.png",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyAlertString,
                            "{oculus}/icons/rifts_left_controller_ready_alert.png",
                        );
                        set_prop(
                            NamedIconPathDeviceAlertLowString,
                            "{oculus}/icons/rifts_left_controller_ready_low.png",
                        );
                    } else if right_hand {
                        set_prop(
                            NamedIconPathDeviceOffString,
                            "{oculus}/icons/rifts_right_controller_off.png",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingString,
                            "{oculus}/icons/rifts_right_controller_searching.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingAlertString,
                            "{oculus}/icons/rifts_right_controller_searching_alert.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyString,
                            "{oculus}/icons/rifts_right_controller_ready.png",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyAlertString,
                            "{oculus}/icons/rifts_right_controller_ready_alert.png",
                        );
                        set_prop(
                            NamedIconPathDeviceAlertLowString,
                            "{oculus}/icons/rifts_right_controller_ready_low.png",
                        );
                    }
                }
                ControllersEmulationMode::RiftSTouch => {
                    set_prop(TrackingSystemNameString, "oculus");
                    set_prop(ManufacturerNameString, "Oculus");
                    if left_hand {
                        set_prop(ModelNumberString, "Oculus Rift S (Left Controller)");
                        set_prop(RenderModelNameString, "oculus_rifts_controller_left");
                        set_prop(
                            RegisteredDeviceTypeString,
                            "oculus/1WMGH000XX0000_Controller_Left",
                        );
                    } else if right_hand {
                        set_prop(ModelNumberString, "Oculus Rift S (Right Controller)");
                        set_prop(RenderModelNameString, "oculus_rifts_controller_right");
                        set_prop(
                            RegisteredDeviceTypeString,
                            "oculus/1WMGH000XX0000_Controller_Right",
                        );
                    }
                    set_prop(ControllerTypeString, "oculus_touch");
                    set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");

                    if left_hand {
                        set_prop(
                            NamedIconPathDeviceOffString,
                            "{oculus}/icons/rifts_left_controller_off.png",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingString,
                            "{oculus}/icons/rifts_left_controller_searching.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingAlertString,
                            "{oculus}/icons/rifts_left_controller_searching_alert.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyString,
                            "{oculus}/icons/rifts_left_controller_ready.png",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyAlertString,
                            "{oculus}/icons/rifts_left_controller_ready_alert.png",
                        );
                        set_prop(
                            NamedIconPathDeviceAlertLowString,
                            "{oculus}/icons/rifts_left_controller_ready_low.png",
                        );
                    } else if right_hand {
                        set_prop(
                            NamedIconPathDeviceOffString,
                            "{oculus}/icons/rifts_right_controller_off.png",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingString,
                            "{oculus}/icons/rifts_right_controller_searching.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceSearchingAlertString,
                            "{oculus}/icons/rifts_right_controller_searching_alert.gif",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyString,
                            "{oculus}/icons/rifts_right_controller_ready.png",
                        );
                        set_prop(
                            NamedIconPathDeviceReadyAlertString,
                            "{oculus}/icons/rifts_right_controller_ready_alert.png",
                        );
                        set_prop(
                            NamedIconPathDeviceAlertLowString,
                            "{oculus}/icons/rifts_right_controller_ready_low.png",
                        );
                    }
                }
                ControllersEmulationMode::ValveIndex => {
                    set_prop(TrackingSystemNameString, "indexcontroller");
                    set_prop(ManufacturerNameString, "Valve");
                    if left_hand {
                        set_prop(ModelNumberString, "Knuckles (Left Controller)");
                        set_prop(
                            RenderModelNameString,
                            "{indexcontroller}valve_controller_knu_1_0_left",
                        );
                        set_prop(
                            RegisteredDeviceTypeString,
                            "valve/index_controllerLHR-E217CD00_Left",
                        );
                    } else if right_hand {
                        set_prop(ModelNumberString, "Knuckles (Right Controller)");
                        set_prop(
                            RenderModelNameString,
                            "{indexcontroller}valve_controller_knu_1_0_right",
                        );
                        set_prop(
                            RegisteredDeviceTypeString,
                            "valve/index_controllerLHR-E217CD00_Right",
                        );
                    }
                    set_prop(ControllerTypeString, "knuckles");
                    set_prop(
                        InputProfilePathString,
                        "{indexcontroller}/input/index_controller_profile.json",
                    );
                }
                ControllersEmulationMode::ViveWand => {
                    set_prop(TrackingSystemNameString, "htc");
                    set_prop(ManufacturerNameString, "HTC");
                    set_prop(RenderModelNameString, "vr_controller_vive_1_5");
                    if left_hand {
                        set_prop(
                            ModelNumberString,
                            "ALVR Remote Controller (Left Controller)",
                        );
                        set_prop(RegisteredDeviceTypeString, "vive_controller_Left");
                    } else if right_hand {
                        set_prop(
                            ModelNumberString,
                            "ALVR Remote Controller (Right Controller)",
                        );
                        set_prop(RegisteredDeviceTypeString, "oculus/vive_controller_Right");
                    }
                    set_prop(ControllerTypeString, "vive_controller");
                    set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");
                }
                ControllersEmulationMode::ViveTracker => {
                    set_prop(TrackingSystemNameString, "lighthouse");
                    set_prop(RenderModelNameString, "{htc}vr_tracker_vive_1_0");
                    if left_hand {
                        set_prop(ModelNumberString, "Vive Tracker Pro MV (Left Controller)");
                        set_prop(RegisteredDeviceTypeString, "ALVR/tracker/left_foot");
                        set_prop(ControllerTypeString, "vive_tracker_left_foot");
                    } else if right_hand {
                        set_prop(ModelNumberString, "Vive Tracker Pro MV (Right Controller)");
                        set_prop(RegisteredDeviceTypeString, "ALVR/tracker/right_foot");
                        set_prop(ControllerTypeString, "vive_tracker_right_foot");
                    }
                    set_prop(
                        InputProfilePathString,
                        "{htc}/input/vive_tracker_profile.json",
                    );

                    // All of these property values were dumped from real a vive tracker via
                    // https://github.com/SDraw/openvr_dumper and were copied from
                    // https://github.com/SDraw/driver_kinectV2
                    set_prop(ResourceRootString, "htc");
                    set_prop(WillDriftInYawBool, "false");
                    set_prop(
                        TrackingFirmwareVersionString,
                        "1541800000 RUNNER-WATCHMAN$runner-watchman@runner-watchman 2018-01-01 FPGA 512(2.56/0/0) BL 0 VRC 1541800000 Radio 1518800000",
                    );
                    set_prop(
                        HardwareRevisionString,
                        "product 128 rev 2.5.6 lot 2000/0/0 0",
                    );
                    set_prop(ConnectedWirelessDongleString, "D0000BE000");
                    set_prop(DeviceIsWirelessBool, "true");
                    set_prop(DeviceIsChargingBool, "false");
                    set_prop(ControllerHandSelectionPriorityInt32, "-1");
                    // vr::HmdMatrix34_t l_transform = {
                    //     {{-1.f, 0.f, 0.f, 0.f}, {0.f, 0.f, -1.f, 0.f}, {0.f, -1.f, 0.f, 0.f}}};
                    // vr_properties->SetProperty(this->prop_container,
                    //                            vr::Prop_StatusDisplayTransform_Matrix34,
                    //                            &l_transform,
                    //                            sizeof(vr::HmdMatrix34_t),
                    //                            vr::k_unHmdMatrix34PropertyTag);
                    set_prop(FirmwareUpdateAvailableBool, "false");
                    set_prop(FirmwareManualUpdateBool, "false");
                    set_prop(
                        FirmwareManualUpdateURLString,
                        "https://developer.valvesoftware.com/wiki/SteamVR/HowTo_Update_Firmware",
                    );
                    set_prop(HardwareRevisionUint64, "2214720000");
                    set_prop(FirmwareVersionUint64, "1541800000");
                    set_prop(FPGAVersionUint64, "512");
                    set_prop(VRCVersionUint64, "1514800000");
                    set_prop(RadioVersionUint64, "1518800000");
                    set_prop(DongleVersionUint64, "8933539758");
                    set_prop(DeviceCanPowerOffBool, "true");
                    // vr_properties->SetStringProperty(this->prop_container,
                    //                                  vr::Prop_Firmware_ProgrammingTargetString,
                    //                                  GetSerialNumber().c_str());
                    set_prop(FirmwareForceUpdateRequiredBool, &serial_number(device_id));
                    set_prop(FirmwareRemindUpdateBool, "false");
                    set_prop(HasDisplayComponentBool, "false");
                    set_prop(HasCameraComponentBool, "false");
                    set_prop(HasDriverDirectModeComponentBool, "false");
                    set_prop(HasVirtualDisplayComponentBool, "false");

                    // icons
                    set_prop(
                        NamedIconPathDeviceOffString,
                        "{htc}/icons/tracker_status_off.png",
                    );
                    set_prop(
                        NamedIconPathDeviceSearchingString,
                        "{htc}/icons/tracker_status_searching.gif",
                    );
                    set_prop(
                        NamedIconPathDeviceSearchingAlertString,
                        "{htc}/icons/tracker_status_searching_alert.gif",
                    );
                    set_prop(
                        NamedIconPathDeviceReadyString,
                        "{htc}/icons/tracker_status_ready.png",
                    );
                    set_prop(
                        NamedIconPathDeviceReadyAlertString,
                        "{htc}/icons/tracker_status_ready_alert.png",
                    );
                    set_prop(
                        NamedIconPathDeviceNotReadyString,
                        "{htc}/icons/tracker_status_error.png",
                    );
                    set_prop(
                        NamedIconPathDeviceStandbyString,
                        "{htc}/icons/tracker_status_standby.png",
                    );
                    set_prop(
                        NamedIconPathDeviceAlertLowString,
                        "{htc}/icons/tracker_status_ready_low.png",
                    );
                }
                ControllersEmulationMode::Custom { .. } => {}
            }

            set_prop(SerialNumberString, &serial_number(device_id));
            set_prop(AttachedDeviceIdString, &serial_number(device_id));

            set_prop(SupportedButtonsUint64, "0xFFFFFFFFFFFFFFFF");

            // OpenXR does not support controller battery
            set_prop(DeviceProvidesBatteryStatusBool, "false");

            // k_eControllerAxis_Joystick = 2
            set_prop(Axis0TypeInt32, "2");

            if matches!(config.emulation_mode, ControllersEmulationMode::ViveTracker) {
                // TrackedControllerRole_Invalid
                set_prop(ControllerRoleHintInt32, "0");
            } else if left_hand {
                // TrackedControllerRole_LeftHand
                set_prop(ControllerRoleHintInt32, "1");
            } else if right_hand {
                // TrackedControllerRole_RightHand
                set_prop(ControllerRoleHintInt32, "2");
            }

            for prop in &config.extra_openvr_props {
                set_prop(prop.key, &prop.value);
            }
        }
    }
}
