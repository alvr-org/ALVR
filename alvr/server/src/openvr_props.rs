// Note: many properties are missing or are stubs.
// todo: fill out more properties for headset and controllers
// todo: add more emulation modes

use crate::{FfiOpenvrProperty, FfiOpenvrPropertyValue, SERVER_DATA_MANAGER};
use alvr_common::{prelude::*, settings_schema::Switch, HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID};
use alvr_session::{
    ControllersEmulationMode, HeadsetEmulationMode, OpenvrPropValue,
    OpenvrPropertyKey::{self, *},
};
use std::{
    ffi::{c_char, CString},
    ptr,
};

pub fn to_ffi_openvr_prop(key: OpenvrPropertyKey, value: OpenvrPropValue) -> FfiOpenvrProperty {
    let type_ = match value {
        OpenvrPropValue::Bool(_) => crate::FfiOpenvrPropertyType_Bool,
        OpenvrPropValue::Float(_) => crate::FfiOpenvrPropertyType_Float,
        OpenvrPropValue::Int32(_) => crate::FfiOpenvrPropertyType_Int32,
        OpenvrPropValue::Uint64(_) => crate::FfiOpenvrPropertyType_Uint64,
        OpenvrPropValue::Vector3(_) => crate::FfiOpenvrPropertyType_Vector3,
        OpenvrPropValue::Double(_) => crate::FfiOpenvrPropertyType_Double,
        OpenvrPropValue::String(_) => crate::FfiOpenvrPropertyType_String,
    };

    let value = match value {
        OpenvrPropValue::Bool(bool_) => FfiOpenvrPropertyValue {
            bool_: bool_.into(),
        },
        OpenvrPropValue::Float(float_) => FfiOpenvrPropertyValue { float_ },
        OpenvrPropValue::Int32(int32) => FfiOpenvrPropertyValue { int32 },
        OpenvrPropValue::Uint64(uint64) => FfiOpenvrPropertyValue { uint64 },
        OpenvrPropValue::Vector3(vector3) => FfiOpenvrPropertyValue { vector3 },
        OpenvrPropValue::Double(double_) => FfiOpenvrPropertyValue { double_ },
        OpenvrPropValue::String(value) => {
            let c_string = CString::new(value).unwrap();
            let mut string = [0; 256];

            unsafe {
                ptr::copy_nonoverlapping(
                    c_string.as_ptr(),
                    string.as_mut_ptr(),
                    c_string.as_bytes_with_nul().len(),
                );
            }

            FfiOpenvrPropertyValue { string }
        }
    };

    FfiOpenvrProperty {
        key: key as u32,
        type_,
        value,
    }
}

fn serial_number(device_id: u64) -> String {
    let data_manager_lock = SERVER_DATA_MANAGER.read();
    let settings = data_manager_lock.settings();

    if device_id == *HEAD_ID {
        match &settings.headset.emulation_mode {
            HeadsetEmulationMode::RiftS => "1WMGH000XX0000".into(),
            HeadsetEmulationMode::Vive => "HTCVive-001".into(),
            HeadsetEmulationMode::Quest2 => "1WMHH000X00000".into(),
            HeadsetEmulationMode::Custom { serial_number, .. } => serial_number.clone(),
        }
    } else if device_id == *LEFT_HAND_ID || device_id == *RIGHT_HAND_ID {
        if let Switch::Enabled(controllers) = &settings.headset.controllers {
            let serial_number = match &controllers.emulation_mode {
                ControllersEmulationMode::RiftSTouch => "1WMGH000XX0000_Controller",
                ControllersEmulationMode::ValveIndex => "ALVR Remote Controller",
                ControllersEmulationMode::ViveWand => "ALVR Remote Controller",
                ControllersEmulationMode::Quest2Touch => "1WMHH000X00000_Controller",
                ControllersEmulationMode::ViveTracker => "ALVR Remote Controller",
            };

            if device_id == *LEFT_HAND_ID {
                format!("{serial_number}_Left")
            } else {
                format!("{serial_number}_Right")
            }
        } else {
            "Unknown".into()
        }
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
    let data_manager_lock = SERVER_DATA_MANAGER.read();
    let settings = data_manager_lock.settings();

    if device_id == *HEAD_ID {
        fn set_prop(key: OpenvrPropertyKey, value: OpenvrPropValue) {
            info!("Setting head OpenVR prop: {key:?} => {value:?}");
            unsafe {
                crate::SetOpenvrProperty(*HEAD_ID, to_ffi_openvr_prop(key, value));
            }
        }
        fn set_string(key: OpenvrPropertyKey, value: &str) {
            set_prop(key, OpenvrPropValue::String(value.into()));
        }

        match &settings.headset.emulation_mode {
            HeadsetEmulationMode::RiftS => {
                set_string(TrackingSystemName, "oculus");
                set_string(ModelNumber, "Oculus Rift S");
                set_string(ManufacturerName, "Oculus");
                set_string(RenderModelName, "generic_hmd");
                set_string(RegisteredDeviceType, "oculus/1WMGH000XX0000");
                set_string(DriverVersion, "1.42.0");
            }
            HeadsetEmulationMode::Vive => {
                set_string(TrackingSystemName, "Vive Tracker");
                set_string(ModelNumber, "ALVR driver server");
                set_string(ManufacturerName, "HTC");
                set_string(RenderModelName, "generic_hmd");
                set_string(RegisteredDeviceType, "vive");
                set_string(DriverVersion, "");
            }
            HeadsetEmulationMode::Quest2 => {
                set_string(TrackingSystemName, "oculus");
                set_string(ModelNumber, "Miramar");
                set_string(ManufacturerName, "Oculus");
                set_string(RenderModelName, "generic_hmd");
                set_string(RegisteredDeviceType, "oculus/1WMHH000X00000");
                set_string(DriverVersion, "1.55.0");
            }
            HeadsetEmulationMode::Custom { props, .. } => {
                for prop in props {
                    set_prop(prop.key, prop.value.clone());
                }
            }
        }

        set_prop(UserIpdMeters, OpenvrPropValue::Float(0.063));
        set_prop(UserHeadToEyeDepthMeters, OpenvrPropValue::Float(0.0));
        set_prop(SecondsFromVsyncToPhotons, OpenvrPropValue::Float(0.0));

        // return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
        set_prop(CurrentUniverseId, OpenvrPropValue::Uint64(2));

        if cfg!(windows) {
            // avoid "not fullscreen" warnings from vrmonitor
            set_prop(IsOnDesktop, OpenvrPropValue::Bool(false));

            // We let SteamVR handle VSyncs. We just wait in PostPresent().
            set_prop(
                DriverDirectModeSendsVsyncEvents,
                OpenvrPropValue::Bool(false),
            );
        }
        set_prop(DeviceProvidesBatteryStatus, OpenvrPropValue::Bool(true));
        set_prop(ContainsProximitySensor, OpenvrPropValue::Bool(true));

        // todo: set different strings for each emulation mode
        set_string(
            NamedIconPathDeviceOff,
            "{oculus}/icons/quest_headset_off.png",
        );
        set_string(
            NamedIconPathDeviceSearching,
            "{oculus}/icons/quest_headset_searching.gif",
        );
        set_string(
            NamedIconPathDeviceSearchingAlert,
            "{oculus}/icons/quest_headset_alert_searching.gif",
        );
        set_string(
            NamedIconPathDeviceReady,
            "{oculus}/icons/quest_headset_ready.png",
        );
        set_string(
            NamedIconPathDeviceReadyAlert,
            "{oculus}/icons/quest_headset_ready_alert.png",
        );
        set_string(
            NamedIconPathDeviceStandby,
            "{oculus}/icons/quest_headset_standby.png",
        );

        for prop in &settings.headset.extra_openvr_props {
            set_prop(prop.key, prop.value.clone());
        }
    } else if device_id == *LEFT_HAND_ID || device_id == *RIGHT_HAND_ID {
        if let Switch::Enabled(config) = &settings.headset.controllers {
            let set_prop = |key, value| {
                info!(
                    "Setting {} controller OpenVR prop: {key:?} => {value:?}",
                    if device_id == *LEFT_HAND_ID {
                        "left"
                    } else {
                        "right"
                    }
                );
                unsafe {
                    crate::SetOpenvrProperty(device_id, to_ffi_openvr_prop(key, value));
                }
            };
            let set_bool = |key, value| {
                set_prop(key, OpenvrPropValue::Bool(value));
            };
            let set_int32 = |key, value| {
                set_prop(key, OpenvrPropValue::Int32(value));
            };
            let set_uint64 = |key, value| {
                set_prop(key, OpenvrPropValue::Uint64(value));
            };
            let set_string = |key, value: &str| {
                set_prop(key, OpenvrPropValue::String(value.into()));
            };

            match config.emulation_mode {
                ControllersEmulationMode::RiftSTouch => {
                    set_string(TrackingSystemName, "oculus");
                    set_string(ManufacturerName, "Oculus");
                    if device_id == *LEFT_HAND_ID {
                        set_string(ModelNumber, "Oculus Rift S (Left Controller)");
                        set_string(RenderModelName, "oculus_rifts_controller_left");
                        set_string(
                            RegisteredDeviceType,
                            "oculus/1WMGH000XX0000_Controller_Left",
                        );
                    } else if device_id == *RIGHT_HAND_ID {
                        set_string(ModelNumber, "Oculus Rift S (Right Controller)");
                        set_string(RenderModelName, "oculus_rifts_controller_right");
                        set_string(
                            RegisteredDeviceType,
                            "oculus/1WMGH000XX0000_Controller_Right",
                        );
                    }
                    set_string(ControllerType, "oculus_touch");
                    set_string(InputProfilePath, "{oculus}/input/touch_profile.json");

                    if device_id == *LEFT_HAND_ID {
                        set_string(
                            NamedIconPathDeviceOff,
                            "{oculus}/icons/rifts_left_controller_off.png",
                        );
                        set_string(
                            NamedIconPathDeviceSearching,
                            "{oculus}/icons/rifts_left_controller_searching.gif",
                        );
                        set_string(
                            NamedIconPathDeviceSearchingAlert,
                            "{oculus}/icons/rifts_left_controller_searching_alert.gif",
                        );
                        set_string(
                            NamedIconPathDeviceReady,
                            "{oculus}/icons/rifts_left_controller_ready.png",
                        );
                        set_string(
                            NamedIconPathDeviceReadyAlert,
                            "{oculus}/icons/rifts_left_controller_ready_alert.png",
                        );
                        set_string(
                            NamedIconPathDeviceAlertLow,
                            "{oculus}/icons/rifts_left_controller_ready_low.png",
                        );
                    } else if device_id == *RIGHT_HAND_ID {
                        set_string(
                            NamedIconPathDeviceOff,
                            "{oculus}/icons/rifts_right_controller_off.png",
                        );
                        set_string(
                            NamedIconPathDeviceSearching,
                            "{oculus}/icons/rifts_right_controller_searching.gif",
                        );
                        set_string(
                            NamedIconPathDeviceSearchingAlert,
                            "{oculus}/icons/rifts_right_controller_searching_alert.gif",
                        );
                        set_string(
                            NamedIconPathDeviceReady,
                            "{oculus}/icons/rifts_right_controller_ready.png",
                        );
                        set_string(
                            NamedIconPathDeviceReadyAlert,
                            "{oculus}/icons/rifts_right_controller_ready_alert.png",
                        );
                        set_string(
                            NamedIconPathDeviceAlertLow,
                            "{oculus}/icons/rifts_right_controller_ready_low.png",
                        );
                    }
                }
                ControllersEmulationMode::ValveIndex => {
                    set_string(TrackingSystemName, "indexcontroller");
                    set_string(ManufacturerName, "Valve");
                    if device_id == *LEFT_HAND_ID {
                        set_string(ModelNumber, "Knuckles (Left Controller)");
                        set_string(
                            RenderModelName,
                            "{indexcontroller}valve_controller_knu_1_0_left",
                        );
                        set_string(
                            RegisteredDeviceType,
                            "valve/index_controllerLHR-E217CD00_Left",
                        );
                    } else if device_id == *RIGHT_HAND_ID {
                        set_string(ModelNumber, "Knuckles (Right Controller)");
                        set_string(
                            RenderModelName,
                            "{indexcontroller}valve_controller_knu_1_0_right",
                        );
                        set_string(
                            RegisteredDeviceType,
                            "valve/index_controllerLHR-E217CD00_Right",
                        );
                    }
                    set_string(ControllerType, "knuckles");
                    set_string(
                        InputProfilePath,
                        "{indexcontroller}/input/index_controller_profile.json",
                    );
                }
                ControllersEmulationMode::ViveWand => {
                    set_string(TrackingSystemName, "htc");
                    set_string(ManufacturerName, "HTC");
                    set_string(RenderModelName, "vr_controller_vive_1_5");
                    if device_id == *LEFT_HAND_ID {
                        set_string(ModelNumber, "ALVR Remote Controller (Left Controller)");
                        set_string(RegisteredDeviceType, "vive_controller_Left");
                    } else if device_id == *RIGHT_HAND_ID {
                        set_string(ModelNumber, "ALVR Remote Controller (Right Controller)");
                        set_string(RegisteredDeviceType, "oculus/vive_controller_Right");
                    }
                    set_string(ControllerType, "vive_controller");
                    set_string(InputProfilePath, "{oculus}/input/touch_profile.json");
                }
                ControllersEmulationMode::Quest2Touch => {
                    set_string(TrackingSystemName, "oculus");
                    set_string(ManufacturerName, "Oculus");
                    if device_id == *LEFT_HAND_ID {
                        set_string(ModelNumber, "Miramar (Left Controller)");
                        set_string(RenderModelName, "oculus_quest2_controller_left");
                        set_string(
                            RegisteredDeviceType,
                            "oculus/1WMHH000X00000_Controller_Left",
                        );
                    } else if device_id == *RIGHT_HAND_ID {
                        set_string(ModelNumber, "Miramar (Right Controller)");
                        set_string(RenderModelName, "oculus_quest2_controller_right");
                        set_string(
                            RegisteredDeviceType,
                            "oculus/1WMHH000X00000_Controller_Right",
                        );
                    }
                    set_string(ControllerType, "oculus_touch");
                    set_string(InputProfilePath, "{oculus}/input/touch_profile.json");

                    if device_id == *LEFT_HAND_ID {
                        set_string(
                            NamedIconPathDeviceOff,
                            "{oculus}/icons/rifts_left_controller_off.png",
                        );
                        set_string(
                            NamedIconPathDeviceSearching,
                            "{oculus}/icons/rifts_left_controller_searching.gif",
                        );
                        set_string(
                            NamedIconPathDeviceSearchingAlert,
                            "{oculus}/icons/rifts_left_controller_searching_alert.gif",
                        );
                        set_string(
                            NamedIconPathDeviceReady,
                            "{oculus}/icons/rifts_left_controller_ready.png",
                        );
                        set_string(
                            NamedIconPathDeviceReadyAlert,
                            "{oculus}/icons/rifts_left_controller_ready_alert.png",
                        );
                        set_string(
                            NamedIconPathDeviceAlertLow,
                            "{oculus}/icons/rifts_left_controller_ready_low.png",
                        );
                    } else if device_id == *RIGHT_HAND_ID {
                        set_string(
                            NamedIconPathDeviceOff,
                            "{oculus}/icons/rifts_right_controller_off.png",
                        );
                        set_string(
                            NamedIconPathDeviceSearching,
                            "{oculus}/icons/rifts_right_controller_searching.gif",
                        );
                        set_string(
                            NamedIconPathDeviceSearchingAlert,
                            "{oculus}/icons/rifts_right_controller_searching_alert.gif",
                        );
                        set_string(
                            NamedIconPathDeviceReady,
                            "{oculus}/icons/rifts_right_controller_ready.png",
                        );
                        set_string(
                            NamedIconPathDeviceReadyAlert,
                            "{oculus}/icons/rifts_right_controller_ready_alert.png",
                        );
                        set_string(
                            NamedIconPathDeviceAlertLow,
                            "{oculus}/icons/rifts_right_controller_ready_low.png",
                        );
                    }
                }
                ControllersEmulationMode::ViveTracker => {
                    set_string(TrackingSystemName, "lighthouse");
                    set_string(RenderModelName, "{htc}vr_tracker_vive_1_0");
                    if device_id == *LEFT_HAND_ID {
                        set_string(ModelNumber, "Vive Tracker Pro MV (Left Controller)");
                        set_string(RegisteredDeviceType, "ALVR/tracker/left_foot");
                        set_string(ControllerType, "vive_tracker_left_foot");
                    } else if device_id == *RIGHT_HAND_ID {
                        set_string(ModelNumber, "Vive Tracker Pro MV (Right Controller)");
                        set_string(RegisteredDeviceType, "ALVR/tracker/right_foot");
                        set_string(ControllerType, "vive_tracker_right_foot");
                    }
                    set_string(InputProfilePath, "{htc}/input/vive_tracker_profile.json");

                    // All of these property values were dumped from real a vive tracker via
                    // https://github.com/SDraw/openvr_dumper and were copied from
                    // https://github.com/SDraw/driver_kinectV2
                    set_string(ResourceRoot, "htc");
                    set_bool(WillDriftInYaw, false);
                    set_string(TrackingFirmwareVersion, "1541800000 RUNNER-WATCHMAN$runner-watchman@runner-watchman 2018-01-01 FPGA 512(2.56/0/0) BL 0 VRC 1541800000 Radio 1518800000");
                    set_string(
                        HardwareRevisionString,
                        "product 128 rev 2.5.6 lot 2000/0/0 0",
                    );
                    set_string(ConnectedWirelessDongle, "D0000BE000");
                    set_bool(DeviceIsWireless, true);
                    set_bool(DeviceIsCharging, false);
                    set_int32(ControllerHandSelectionPriority, -1);
                    // vr::HmdMatrix34_t l_transform = {
                    //     {{-1.f, 0.f, 0.f, 0.f}, {0.f, 0.f, -1.f, 0.f}, {0.f, -1.f, 0.f, 0.f}}};
                    // vr_properties->SetProperty(this->prop_container,
                    //                            vr::Prop_StatusDisplayTransform_Matrix34,
                    //                            &l_transform,
                    //                            sizeof(vr::HmdMatrix34_t),
                    //                            vr::k_unHmdMatrix34PropertyTag);
                    set_bool(FirmwareUpdateAvailable, false);
                    set_bool(FirmwareManualUpdate, false);
                    set_string(
                        FirmwareManualUpdateURL,
                        "https://developer.valvesoftware.com/wiki/SteamVR/HowTo_Update_Firmware",
                    );
                    set_uint64(HardwareRevisionUint64, 2214720000);
                    set_uint64(FirmwareVersion, 1541800000);
                    set_uint64(FPGAVersion, 512);
                    set_uint64(VRCVersion, 1514800000);
                    set_uint64(RadioVersion, 1518800000);
                    set_uint64(DongleVersion, 8933539758);
                    set_bool(DeviceCanPowerOff, true);
                    // vr_properties->SetStringProperty(this->prop_container,
                    //                                  vr::Prop_Firmware_ProgrammingTarget_String,
                    //                                  GetSerialNumber().c_str());
                    set_bool(FirmwareForceUpdateRequired, false);
                    set_bool(FirmwareRemindUpdate, false);
                    set_bool(HasDisplayComponent, false);
                    set_bool(HasCameraComponent, false);
                    set_bool(HasDriverDirectModeComponent, false);
                    set_bool(HasVirtualDisplayComponent, false);

                    // icons
                    set_string(NamedIconPathDeviceOff, "{htc}/icons/tracker_status_off.png");
                    set_string(
                        NamedIconPathDeviceSearching,
                        "{htc}/icons/tracker_status_searching.gif",
                    );
                    set_string(
                        NamedIconPathDeviceSearchingAlert,
                        "{htc}/icons/tracker_status_searching_alert.gif",
                    );
                    set_string(
                        NamedIconPathDeviceReady,
                        "{htc}/icons/tracker_status_ready.png",
                    );
                    set_string(
                        NamedIconPathDeviceReadyAlert,
                        "{htc}/icons/tracker_status_ready_alert.png",
                    );
                    set_string(
                        NamedIconPathDeviceNotReady,
                        "{htc}/icons/tracker_status_error.png",
                    );
                    set_string(
                        NamedIconPathDeviceStandby,
                        "{htc}/icons/tracker_status_standby.png",
                    );
                    set_string(
                        NamedIconPathDeviceAlertLow,
                        "{htc}/icons/tracker_status_ready_low.png",
                    );
                }
            }

            set_string(SerialNumber, &serial_number(device_id));
            set_string(AttachedDeviceId, &serial_number(device_id));

            set_uint64(SupportedButtons, 0xFFFFFFFFFFFFFFFF);

            // OpenXR does not support controller battery
            set_bool(DeviceProvidesBatteryStatus, false);

            // k_eControllerAxis_Joystick = 2
            set_prop(Axis0Type, OpenvrPropValue::Int32(2));

            if matches!(config.emulation_mode, ControllersEmulationMode::ViveTracker) {
                // TrackedControllerRole_Invalid
                set_int32(ControllerRoleHint, 0);
            } else if device_id == *LEFT_HAND_ID {
                // TrackedControllerRole_LeftHand
                set_int32(ControllerRoleHint, 1);
            } else if device_id == *RIGHT_HAND_ID {
                // TrackedControllerRole_RightHand
                set_int32(ControllerRoleHint, 2);
            }

            for prop in &config.extra_openvr_props {
                set_prop(prop.key, prop.value.clone());
            }
        }
    }
}
