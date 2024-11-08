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
        warn!(
            "Failed to parse {device_name} value for OpenVR property: {:?}={value}",
            prop.key
        );

        return;
    };

    debug!("Setting {device_name} OpenVR prop: {:?}={value}", prop.key);

    unsafe {
        crate::SetOpenvrProperty(
            device_id,
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
            HeadsetEmulationMode::QuestPro => "230YC0XXXX00XX".into(),
            HeadsetEmulationMode::Vive => "HTCVive-001".into(),
            HeadsetEmulationMode::Custom { serial_number, .. } => serial_number.clone(),
        }
    } else if device_id == *HAND_LEFT_ID || device_id == *HAND_RIGHT_ID {
        if let Switch::Enabled(controllers) = &settings.headset.controllers {
            let serial_number = match &controllers.emulation_mode {
                ControllersEmulationMode::Quest2Touch => "1WMHH000X00000_Controller",
                ControllersEmulationMode::Quest3Plus => "2G0YXX0X0000XX_Controller", // 2G0YY Left 2G0YZ Right
                ControllersEmulationMode::QuestPro => "230YXXXXXXXXXX_Controller", // 230YT left, 230YV right
                ControllersEmulationMode::RiftSTouch
                | ControllersEmulationMode::ValveIndex
                | ControllersEmulationMode::ViveWand
                | ControllersEmulationMode::ViveTracker => "ALVR Remote Controller",
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

    let set_icons = |base_path: &str| {
        set_prop(
            NamedIconPathDeviceOffString,
            format!("{base_path}_off.png").as_str(),
        );
        set_prop(
            NamedIconPathDeviceSearchingString,
            format!("{base_path}_searching.gif").as_str(),
        );
        set_prop(
            NamedIconPathDeviceSearchingAlertString,
            format!("{base_path}_searching_alert.gif").as_str(),
        );
        set_prop(
            NamedIconPathDeviceReadyString,
            format!("{base_path}_ready.png").as_str(),
        );
        set_prop(
            NamedIconPathDeviceReadyAlertString,
            format!("{base_path}_ready_alert.png").as_str(),
        );
        set_prop(
            NamedIconPathDeviceAlertLowString,
            format!("{base_path}_ready_low.png").as_str(),
        );
        set_prop(
            NamedIconPathDeviceStandbyString,
            format!("{base_path}_standby.png").as_str(),
        );
        set_prop(
            NamedIconPathDeviceStandbyAlertString,
            format!("{base_path}_standby_alert.gif").as_str(),
        );
    };

    let device_serial = &serial_number(device_id);
    let headset_serial = &serial_number(*HEAD_ID);

    if device_id == *HEAD_ID {
        // Closure for all the common Quest headset properties
        let set_oculus_common_headset_props = || {
            set_prop(
                RegisteredDeviceTypeString,
                format!("oculus/{headset_serial}").as_str(),
            );
            set_icons("{oculus}/icons/quest_headset");
        };

        // Per-device props
        match &settings.headset.emulation_mode {
            HeadsetEmulationMode::RiftS => {
                set_prop(TrackingSystemNameString, "oculus");
                set_prop(ModelNumberString, "Oculus Rift S");
                set_prop(ManufacturerNameString, "Oculus");
                set_prop(RenderModelNameString, "generic_hmd");
                set_prop(DriverVersionString, "1.42.0");
                set_icons("{oculus}/icons/rifts_headset");
            }
            HeadsetEmulationMode::Quest2 => {
                set_prop(TrackingSystemNameString, "oculus");
                set_prop(ModelNumberString, "Miramar");
                set_prop(ManufacturerNameString, "Oculus");
                set_prop(RenderModelNameString, "generic_hmd");
                set_prop(DriverVersionString, "1.55.0");
                set_oculus_common_headset_props();
            }
            HeadsetEmulationMode::QuestPro => {
                set_prop(TrackingSystemNameString, "oculus");
                set_prop(ModelNumberString, "Meta Quest Pro");
                set_prop(ManufacturerNameString, "Oculus");
                set_prop(RenderModelNameString, "generic_hmd");
                set_prop(DriverVersionString, "1.55.0");
                set_oculus_common_headset_props();
            }
            HeadsetEmulationMode::Vive => {
                set_prop(TrackingSystemNameString, "Vive Tracker");
                set_prop(ModelNumberString, "ALVR driver server");
                set_prop(ManufacturerNameString, "HTC");
                set_prop(RenderModelNameString, "generic_hmd");
                set_prop(RegisteredDeviceTypeString, "vive");
                set_prop(DriverVersionString, "");
                set_icons("{htc}/icons/vive_headset");
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
        let full_skeletal_hand =
            device_id == *HAND_TRACKER_LEFT_ID || device_id == *HAND_TRACKER_RIGHT_ID;
        if let Switch::Enabled(config) = &settings.headset.controllers {
            // Closure for all the common Oculus/Meta controller properties
            let set_oculus_common_props = || {
                set_prop(TrackingSystemNameString, "oculus");

                set_prop(ControllerTypeString, "oculus_touch");
                set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");
                if left_hand {
                    set_prop(
                        RegisteredDeviceTypeString,
                        format!("oculus/{headset_serial}_Controller_Left").as_str(),
                    );
                    set_icons("{oculus}/icons/rifts_left_controller");
                } else if right_hand {
                    set_prop(
                        RegisteredDeviceTypeString,
                        format!("oculus/{headset_serial}_Controller_Right").as_str(),
                    );
                    set_icons("{oculus}/icons/rifts_right_controller");
                }
            };

            // Controller-specific properties, not shared
            match config.emulation_mode {
                ControllersEmulationMode::RiftSTouch => {
                    set_prop(ManufacturerNameString, "Oculus");
                    if left_hand {
                        set_prop(ModelNumberString, "Oculus Rift S (Left Controller)");
                        set_prop(RenderModelNameString, "oculus_rifts_controller_left");
                    } else if right_hand {
                        set_prop(ModelNumberString, "Oculus Rift S (Right Controller)");
                        set_prop(RenderModelNameString, "oculus_rifts_controller_right");
                    }
                    set_prop(ControllerTypeString, "oculus_touch");
                    set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");
                    set_oculus_common_props();
                }
                ControllersEmulationMode::Quest2Touch => {
                    set_prop(ManufacturerNameString, "Oculus");
                    if left_hand {
                        set_prop(ModelNumberString, "Miramar (Left Controller)");
                        set_prop(RenderModelNameString, "oculus_quest2_controller_left");
                    } else if right_hand {
                        set_prop(ModelNumberString, "Miramar (Right Controller)");
                        set_prop(RenderModelNameString, "oculus_quest2_controller_right");
                    }
                    set_prop(ControllerTypeString, "oculus_touch");
                    set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");
                    set_oculus_common_props();
                }
                ControllersEmulationMode::Quest3Plus => {
                    set_prop(ManufacturerNameString, "Meta");

                    if left_hand {
                        set_prop(ModelNumberString, "Meta Quest 3 (Left Controller)");
                        set_prop(RenderModelNameString, "oculus_quest_plus_controller_left");
                    } else if right_hand {
                        set_prop(ModelNumberString, "Meta Quest 3 (Right Controller)");
                        set_prop(RenderModelNameString, "oculus_quest_plus_controller_right");
                    }
                    set_prop(ControllerTypeString, "oculus_touch");
                    set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");
                    set_oculus_common_props();
                }
                ControllersEmulationMode::QuestPro => {
                    set_prop(ManufacturerNameString, "Meta");

                    if left_hand {
                        set_prop(ModelNumberString, "Meta Quest Pro (Left Controller)");
                        set_prop(RenderModelNameString, "oculus_quest_pro_controller_left");
                    } else if right_hand {
                        set_prop(ModelNumberString, "Meta Quest Pro (Right Controller)");
                        set_prop(RenderModelNameString, "oculus_quest_pro_controller_right");
                    }
                    set_oculus_common_props();
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
                        set_icons("{indexcontroller}/icons/left_controller_status");
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
                        set_icons("{indexcontroller}/icons/right_controller_status");
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
                        set_prop(RegisteredDeviceTypeString, "htc/vive_controller_Left");
                    } else if right_hand {
                        set_prop(
                            ModelNumberString,
                            "ALVR Remote Controller (Right Controller)",
                        );
                        set_prop(RegisteredDeviceTypeString, "htc/vive_controller_Right");
                    }
                    set_prop(ControllerTypeString, "vive_controller");
                    set_prop(InputProfilePathString, "{oculus}/input/touch_profile.json");
                    set_icons("{htc}/icons/controller");
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
                    set_icons("{htc}/icons/tracker");

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
                    set_prop(FirmwareForceUpdateRequiredBool, device_serial);
                    set_prop(FirmwareRemindUpdateBool, "false");
                    set_prop(HasDisplayComponentBool, "false");
                    set_prop(HasCameraComponentBool, "false");
                    set_prop(HasDriverDirectModeComponentBool, "false");
                    set_prop(HasVirtualDisplayComponentBool, "false");
                }
                ControllersEmulationMode::Custom { .. } => {}
            }

            set_prop(SerialNumberString, device_serial);
            set_prop(AttachedDeviceIdString, device_serial);

            if full_skeletal_hand {
                set_prop(TrackingSystemNameString, "vrlink");
                set_prop(ManufacturerNameString, "VRLink");

                set_prop(RenderModelNameString, "{vrlink}/rendermodels/shuttlecock");
                set_prop(ControllerTypeString, "svl_hand_interaction_augmented");
                set_prop(
                    InputProfilePathString,
                    "{vrlink}/input/svl_hand_interaction_augmented_input_profile.json",
                );

                if left_hand {
                    set_prop(ModelNumberString, "VRLink Hand Tracker (Left Hand)");
                    set_prop(
                        RegisteredDeviceTypeString,
                        "vrlink/VRLINKQ_HandTracker_Left",
                    );
                    set_prop(SerialNumberString, "VRLINKQ_Hand_Left");
                    set_prop(AttachedDeviceIdString, "VRLINKQ_Hand_Left");
                    set_icons("{vrlink}/icons/left_handtracking");
                } else if right_hand {
                    set_prop(ModelNumberString, "VRLink Hand Tracker (Right Hand)");
                    set_prop(
                        RegisteredDeviceTypeString,
                        "vrlink/VRLINKQ_HandTracker_Right",
                    );
                    set_prop(SerialNumberString, "VRLINKQ_Hand_Right");
                    set_prop(AttachedDeviceIdString, "VRLINKQ_Hand_Right");
                    set_icons("{vrlink}/icons/right_handtracking");
                }
            }

            set_prop(
                SupportedButtonsUint64,
                0xFFFFFFFFFFFFFFFF_u64.to_string().as_str(),
            );

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
