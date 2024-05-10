// Note: many properties are missing or are stubs.
// todo: fill out more properties for headset and controllers
// todo: add more emulation modes

use crate::{FfiOpenvrProperty, FfiOpenvrPropertyValue, SERVER_DATA_MANAGER};
use alvr_common::{info, settings_schema::Switch, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID};
use alvr_session::{
    ControllersEmulationMode, HeadsetEmulationMode, OpenvrPropValue, OpenvrProperty,
};
use std::{
    ffi::{c_char, CString},
    ptr,
};

pub fn to_ffi_openvr_prop(prop: OpenvrProperty) -> FfiOpenvrProperty {
    let (key, value) = prop.into_key_value();

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

    FfiOpenvrProperty { key, type_, value }
}

fn serial_number(device_id: u64) -> String {
    let data_manager_lock = SERVER_DATA_MANAGER.read();
    let settings = data_manager_lock.settings();

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
    use OpenvrProperty::*;

    let data_manager_lock = SERVER_DATA_MANAGER.read();
    let settings = data_manager_lock.settings();

    if device_id == *HEAD_ID {
        fn set_prop(prop: OpenvrProperty) {
            info!("Setting head OpenVR prop: {prop:?}");
            unsafe {
                crate::SetOpenvrProperty(*HEAD_ID, to_ffi_openvr_prop(prop));
            }
        }

        match &settings.headset.emulation_mode {
            HeadsetEmulationMode::RiftS => {
                set_prop(TrackingSystemName("oculus".into()));
                set_prop(ModelNumber("Oculus Rift S".into()));
                set_prop(ManufacturerName("Oculus".into()));
                set_prop(RenderModelName("generic_hmd".into()));
                set_prop(RegisteredDeviceType("oculus/1WMGH000XX0000".into()));
                set_prop(DriverVersion("1.42.0".into()));
                set_prop(NamedIconPathDeviceOff(
                    "{oculus}/icons/rifts_headset_off.png".into(),
                ));
                set_prop(NamedIconPathDeviceSearching(
                    "{oculus}/icons/rifts_headset_searching.gif".into(),
                ));
                set_prop(NamedIconPathDeviceSearchingAlert(
                    "{oculus}/icons/rifts_headset_alert_searching.gif".into(),
                ));
                set_prop(NamedIconPathDeviceReady(
                    "{oculus}/icons/rifts_headset_ready.png".into(),
                ));
                set_prop(NamedIconPathDeviceReadyAlert(
                    "{oculus}/icons/rifts_headset_ready_alert.png".into(),
                ));
                set_prop(NamedIconPathDeviceStandby(
                    "{oculus}/icons/rifts_headset_standby.png".into(),
                ));
            }
            HeadsetEmulationMode::Quest2 => {
                set_prop(TrackingSystemName("oculus".into()));
                set_prop(ModelNumber("Miramar".into()));
                set_prop(ManufacturerName("Oculus".into()));
                set_prop(RenderModelName("generic_hmd".into()));
                set_prop(RegisteredDeviceType("oculus/1WMHH000X00000".into()));
                set_prop(DriverVersion("1.55.0".into()));
                set_prop(NamedIconPathDeviceOff(
                    "{oculus}/icons/quest_headset_off.png".into(),
                ));
                set_prop(NamedIconPathDeviceSearching(
                    "{oculus}/icons/quest_headset_searching.gif".into(),
                ));
                set_prop(NamedIconPathDeviceSearchingAlert(
                    "{oculus}/icons/quest_headset_alert_searching.gif".into(),
                ));
                set_prop(NamedIconPathDeviceReady(
                    "{oculus}/icons/quest_headset_ready.png".into(),
                ));
                set_prop(NamedIconPathDeviceReadyAlert(
                    "{oculus}/icons/quest_headset_ready_alert.png".into(),
                ));
                set_prop(NamedIconPathDeviceStandby(
                    "{oculus}/icons/quest_headset_standby.png".into(),
                ));
            }
            HeadsetEmulationMode::Vive => {
                set_prop(TrackingSystemName("Vive Tracker".into()));
                set_prop(ModelNumber("ALVR driver server".into()));
                set_prop(ManufacturerName("HTC".into()));
                set_prop(RenderModelName("generic_hmd".into()));
                set_prop(RegisteredDeviceType("vive".into()));
                set_prop(DriverVersion("".into()));
                set_prop(NamedIconPathDeviceOff(
                    "{htc}/icons/vive_headset_status_off.png".into(),
                ));
                set_prop(NamedIconPathDeviceSearching(
                    "{htc}/icons/vive_headset_status_searching.gif".into(),
                ));
                set_prop(NamedIconPathDeviceSearchingAlert(
                    "{htc}/icons/vive_headset_status_searching_alert.gif".into(),
                ));
                set_prop(NamedIconPathDeviceReady(
                    "{htc}/icons/vive_headset_status_ready.png".into(),
                ));
                set_prop(NamedIconPathDeviceReadyAlert(
                    "{htc}/icons/vive_headset_status_ready_alert.png".into(),
                ));
                set_prop(NamedIconPathDeviceStandby(
                    "{htc}/icons/vive_headset_status_standby.png".into(),
                ));
            }
            HeadsetEmulationMode::Custom { .. } => (),
        }

        set_prop(UserIpdMeters(0.063));
        set_prop(UserHeadToEyeDepthMeters(0.0));
        set_prop(SecondsFromVsyncToPhotons(0.0));

        // return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
        set_prop(CurrentUniverseId(2));

        if cfg!(windows) {
            // avoid "not fullscreen" warnings from vrmonitor
            set_prop(IsOnDesktop(false));

            // We let SteamVR handle VSyncs. We just wait in PostPresent().
            set_prop(DriverDirectModeSendsVsyncEvents(false));
        }
        set_prop(DeviceProvidesBatteryStatus(true));
        set_prop(ContainsProximitySensor(true));

        for prop in &settings.headset.extra_openvr_props {
            set_prop(prop.clone());
        }
    } else if device_id == *HAND_LEFT_ID || device_id == *HAND_RIGHT_ID {
        if let Switch::Enabled(config) = &settings.headset.controllers {
            let set_prop = |prop| {
                info!(
                    "Setting {} controller OpenVR prop: {prop:?}",
                    if device_id == *HAND_LEFT_ID {
                        "left"
                    } else {
                        "right"
                    }
                );
                unsafe {
                    crate::SetOpenvrProperty(device_id, to_ffi_openvr_prop(prop));
                }
            };

            match config.emulation_mode {
                ControllersEmulationMode::Quest2Touch => {
                    set_prop(TrackingSystemName("oculus".into()));
                    set_prop(ManufacturerName("Oculus".into()));
                    if device_id == *HAND_LEFT_ID {
                        set_prop(ModelNumber("Miramar (Left Controller)".into()));
                        set_prop(RenderModelName("oculus_quest2_controller_left".into()));
                        set_prop(RegisteredDeviceType(
                            "oculus/1WMHH000X00000_Controller_Left".into(),
                        ));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(ModelNumber("Miramar (Right Controller)".into()));
                        set_prop(RenderModelName("oculus_quest2_controller_right".into()));
                        set_prop(RegisteredDeviceType(
                            "oculus/1WMHH000X00000_Controller_Right".into(),
                        ));
                    }
                    set_prop(ControllerType("oculus_touch".into()));
                    set_prop(InputProfilePath("{oculus}/input/touch_profile.json".into()));

                    if device_id == *HAND_LEFT_ID {
                        set_prop(NamedIconPathDeviceOff(
                            "{oculus}/icons/rifts_left_controller_off.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearching(
                            "{oculus}/icons/rifts_left_controller_searching.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearchingAlert(
                            "{oculus}/icons/rifts_left_controller_searching_alert.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceReady(
                            "{oculus}/icons/rifts_left_controller_ready.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceReadyAlert(
                            "{oculus}/icons/rifts_left_controller_ready_alert.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceAlertLow(
                            "{oculus}/icons/rifts_left_controller_ready_low.png".into(),
                        ));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(NamedIconPathDeviceOff(
                            "{oculus}/icons/rifts_right_controller_off.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearching(
                            "{oculus}/icons/rifts_right_controller_searching.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearchingAlert(
                            "{oculus}/icons/rifts_right_controller_searching_alert.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceReady(
                            "{oculus}/icons/rifts_right_controller_ready.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceReadyAlert(
                            "{oculus}/icons/rifts_right_controller_ready_alert.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceAlertLow(
                            "{oculus}/icons/rifts_right_controller_ready_low.png".into(),
                        ));
                    }
                }
                ControllersEmulationMode::Quest3Plus => {
                    set_prop(TrackingSystemName("oculus".into()));
                    set_prop(ManufacturerName("Oculus".into()));
                    if device_id == *HAND_LEFT_ID {
                        set_prop(ModelNumber("Meta Quest 3 (Left Controller)".into()));
                        set_prop(RenderModelName("oculus_quest_plus_controller_left".into()));
                        set_prop(RegisteredDeviceType(
                            "oculus/1WMHH000X00000_Controller_Left".into(),
                        ));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(ModelNumber("Meta Quest 3 (Right Controller)".into()));
                        set_prop(RenderModelName("oculus_quest_plus_controller_right".into()));
                        set_prop(RegisteredDeviceType(
                            "oculus/1WMHH000X00000_Controller_Right".into(),
                        ));
                    }
                    set_prop(ControllerType("oculus_touch".into()));
                    set_prop(InputProfilePath("{oculus}/input/touch_profile.json".into()));

                    if device_id == *HAND_LEFT_ID {
                        set_prop(NamedIconPathDeviceOff(
                            "{oculus}/icons/rifts_left_controller_off.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearching(
                            "{oculus}/icons/rifts_left_controller_searching.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearchingAlert(
                            "{oculus}/icons/rifts_left_controller_searching_alert.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceReady(
                            "{oculus}/icons/rifts_left_controller_ready.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceReadyAlert(
                            "{oculus}/icons/rifts_left_controller_ready_alert.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceAlertLow(
                            "{oculus}/icons/rifts_left_controller_ready_low.png".into(),
                        ));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(NamedIconPathDeviceOff(
                            "{oculus}/icons/rifts_right_controller_off.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearching(
                            "{oculus}/icons/rifts_right_controller_searching.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearchingAlert(
                            "{oculus}/icons/rifts_right_controller_searching_alert.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceReady(
                            "{oculus}/icons/rifts_right_controller_ready.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceReadyAlert(
                            "{oculus}/icons/rifts_right_controller_ready_alert.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceAlertLow(
                            "{oculus}/icons/rifts_right_controller_ready_low.png".into(),
                        ));
                    }
                }
                ControllersEmulationMode::RiftSTouch => {
                    set_prop(TrackingSystemName("oculus".into()));
                    set_prop(ManufacturerName("Oculus".into()));
                    if device_id == *HAND_LEFT_ID {
                        set_prop(ModelNumber("Oculus Rift S (Left Controller)".into()));
                        set_prop(RenderModelName("oculus_rifts_controller_left".into()));
                        set_prop(RegisteredDeviceType(
                            "oculus/1WMGH000XX0000_Controller_Left".into(),
                        ));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(ModelNumber("Oculus Rift S (Right Controller)".into()));
                        set_prop(RenderModelName("oculus_rifts_controller_right".into()));
                        set_prop(RegisteredDeviceType(
                            "oculus/1WMGH000XX0000_Controller_Right".into(),
                        ));
                    }
                    set_prop(ControllerType("oculus_touch".into()));
                    set_prop(InputProfilePath("{oculus}/input/touch_profile.json".into()));

                    if device_id == *HAND_LEFT_ID {
                        set_prop(NamedIconPathDeviceOff(
                            "{oculus}/icons/rifts_left_controller_off.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearching(
                            "{oculus}/icons/rifts_left_controller_searching.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearchingAlert(
                            "{oculus}/icons/rifts_left_controller_searching_alert.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceReady(
                            "{oculus}/icons/rifts_left_controller_ready.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceReadyAlert(
                            "{oculus}/icons/rifts_left_controller_ready_alert.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceAlertLow(
                            "{oculus}/icons/rifts_left_controller_ready_low.png".into(),
                        ));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(NamedIconPathDeviceOff(
                            "{oculus}/icons/rifts_right_controller_off.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearching(
                            "{oculus}/icons/rifts_right_controller_searching.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceSearchingAlert(
                            "{oculus}/icons/rifts_right_controller_searching_alert.gif".into(),
                        ));
                        set_prop(NamedIconPathDeviceReady(
                            "{oculus}/icons/rifts_right_controller_ready.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceReadyAlert(
                            "{oculus}/icons/rifts_right_controller_ready_alert.png".into(),
                        ));
                        set_prop(NamedIconPathDeviceAlertLow(
                            "{oculus}/icons/rifts_right_controller_ready_low.png".into(),
                        ));
                    }
                }
                ControllersEmulationMode::ValveIndex => {
                    set_prop(TrackingSystemName("indexcontroller".into()));
                    set_prop(ManufacturerName("Valve".into()));
                    if device_id == *HAND_LEFT_ID {
                        set_prop(ModelNumber("Knuckles (Left Controller)".into()));
                        set_prop(RenderModelName(
                            "{indexcontroller}valve_controller_knu_1_0_left".into(),
                        ));
                        set_prop(RegisteredDeviceType(
                            "valve/index_controllerLHR-E217CD00_Left".into(),
                        ));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(ModelNumber("Knuckles (Right Controller)".into()));
                        set_prop(RenderModelName(
                            "{indexcontroller}valve_controller_knu_1_0_right".into(),
                        ));
                        set_prop(RegisteredDeviceType(
                            "valve/index_controllerLHR-E217CD00_Right".into(),
                        ));
                    }
                    set_prop(ControllerType("knuckles".into()));
                    set_prop(InputProfilePath(
                        "{indexcontroller}/input/index_controller_profile.json".into(),
                    ));
                }
                ControllersEmulationMode::ViveWand => {
                    set_prop(TrackingSystemName("htc".into()));
                    set_prop(ManufacturerName("HTC".into()));
                    set_prop(RenderModelName("vr_controller_vive_1_5".into()));
                    if device_id == *HAND_LEFT_ID {
                        set_prop(ModelNumber(
                            "ALVR Remote Controller (Left Controller)".into(),
                        ));
                        set_prop(RegisteredDeviceType("vive_controller_Left".into()));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(ModelNumber(
                            "ALVR Remote Controller (Right Controller)".into(),
                        ));
                        set_prop(RegisteredDeviceType("oculus/vive_controller_Right".into()));
                    }
                    set_prop(ControllerType("vive_controller".into()));
                    set_prop(InputProfilePath("{oculus}/input/touch_profile.json".into()));
                }
                ControllersEmulationMode::ViveTracker => {
                    set_prop(TrackingSystemName("lighthouse".into()));
                    set_prop(RenderModelName("{htc}vr_tracker_vive_1_0".into()));
                    if device_id == *HAND_LEFT_ID {
                        set_prop(ModelNumber("Vive Tracker Pro MV (Left Controller)".into()));
                        set_prop(RegisteredDeviceType("ALVR/tracker/left_foot".into()));
                        set_prop(ControllerType("vive_tracker_left_foot".into()));
                    } else if device_id == *HAND_RIGHT_ID {
                        set_prop(ModelNumber("Vive Tracker Pro MV (Right Controller)".into()));
                        set_prop(RegisteredDeviceType("ALVR/tracker/right_foot".into()));
                        set_prop(ControllerType("vive_tracker_right_foot".into()));
                    }
                    set_prop(InputProfilePath(
                        "{htc}/input/vive_tracker_profile.json".into(),
                    ));

                    // All of these property values were dumped from real a vive tracker via
                    // https://github.com/SDraw/openvr_dumper and were copied from
                    // https://github.com/SDraw/driver_kinectV2
                    set_prop(ResourceRoot("htc".into()));
                    set_prop(WillDriftInYaw(false));
                    set_prop(TrackingFirmwareVersion(
                        "1541800000 RUNNER-WATCHMAN$runner-watchman@runner-watchman 2018-01-01 FPGA 512(2.56/0/0) BL 0 VRC 1541800000 Radio 1518800000".into(),
                    ));
                    set_prop(HardwareRevisionString(
                        "product 128 rev 2.5.6 lot 2000/0/0 0".into(),
                    ));
                    set_prop(ConnectedWirelessDongle("D0000BE000".into()));
                    set_prop(DeviceIsWireless(true));
                    set_prop(DeviceIsCharging(false));
                    set_prop(ControllerHandSelectionPriority(-1));
                    // vr::HmdMatrix34_t l_transform = {
                    //     {{-1.f, 0.f, 0.f, 0.f}, {0.f, 0.f, -1.f, 0.f}, {0.f, -1.f, 0.f, 0.f}}};
                    // vr_properties->SetProperty(this->prop_container,
                    //                            vr::Prop_StatusDisplayTransform_Matrix34,
                    //                            &l_transform,
                    //                            sizeof(vr::HmdMatrix34_t),
                    //                            vr::k_unHmdMatrix34PropertyTag);
                    set_prop(FirmwareUpdateAvailable(false));
                    set_prop(FirmwareManualUpdate(false));
                    set_prop(FirmwareManualUpdateURL(
                        "https://developer.valvesoftware.com/wiki/SteamVR/HowTo_Update_Firmware"
                            .into(),
                    ));
                    set_prop(HardwareRevisionUint64(2214720000));
                    set_prop(FirmwareVersion(1541800000));
                    set_prop(FPGAVersion(512));
                    set_prop(VRCVersion(1514800000));
                    set_prop(RadioVersion(1518800000));
                    set_prop(DongleVersion(8933539758));
                    set_prop(DeviceCanPowerOff(true));
                    // vr_properties->SetStringProperty(this->prop_container,
                    //                                  vr::Prop_Firmware_ProgrammingTarget_String,
                    //                                  GetSerialNumber().c_str());
                    set_prop(FirmwareForceUpdateRequired(false));
                    set_prop(FirmwareRemindUpdate(false));
                    set_prop(HasDisplayComponent(false));
                    set_prop(HasCameraComponent(false));
                    set_prop(HasDriverDirectModeComponent(false));
                    set_prop(HasVirtualDisplayComponent(false));

                    // icons
                    set_prop(NamedIconPathDeviceOff(
                        "{htc}/icons/tracker_status_off.png".into(),
                    ));
                    set_prop(NamedIconPathDeviceSearching(
                        "{htc}/icons/tracker_status_searching.gif".into(),
                    ));
                    set_prop(NamedIconPathDeviceSearchingAlert(
                        "{htc}/icons/tracker_status_searching_alert.gif".into(),
                    ));
                    set_prop(NamedIconPathDeviceReady(
                        "{htc}/icons/tracker_status_ready.png".into(),
                    ));
                    set_prop(NamedIconPathDeviceReadyAlert(
                        "{htc}/icons/tracker_status_ready_alert.png".into(),
                    ));
                    set_prop(NamedIconPathDeviceNotReady(
                        "{htc}/icons/tracker_status_error.png".into(),
                    ));
                    set_prop(NamedIconPathDeviceStandby(
                        "{htc}/icons/tracker_status_standby.png".into(),
                    ));
                    set_prop(NamedIconPathDeviceAlertLow(
                        "{htc}/icons/tracker_status_ready_low.png".into(),
                    ));
                }
                ControllersEmulationMode::Custom { .. } => todo!(),
            }

            set_prop(SerialNumber(serial_number(device_id)));
            set_prop(AttachedDeviceId(serial_number(device_id)));

            set_prop(SupportedButtons(0xFFFFFFFFFFFFFFFF));

            // OpenXR does not support controller battery
            set_prop(DeviceProvidesBatteryStatus(false));

            // k_eControllerAxis_Joystick = 2
            set_prop(Axis0Type(2));

            if matches!(config.emulation_mode, ControllersEmulationMode::ViveTracker) {
                // TrackedControllerRole_Invalid
                set_prop(ControllerRoleHint(0));
            } else if device_id == *HAND_LEFT_ID {
                // TrackedControllerRole_LeftHand
                set_prop(ControllerRoleHint(1));
            } else if device_id == *HAND_RIGHT_ID {
                // TrackedControllerRole_RightHand
                set_prop(ControllerRoleHint(2));
            }

            for prop in &config.extra_openvr_props {
                set_prop(prop.clone());
            }
        }
    }
}
