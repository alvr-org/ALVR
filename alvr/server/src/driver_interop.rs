use std::time::Duration;

use crate::{DRIVER_SENDER, SESSION_MANAGER};
use alvr_common::{prelude::*, OpenvrPropValue};
use alvr_ipc::{
    DriverRequest, ResponseForDriver, TrackedDeviceConfig, TrackedDeviceType, VideoConfigUpdate,
};
use settings_schema::Switch;
use tokio::time;

async fn driver_pipeline() -> StrResult {
    let (mut ipc_server, ipc_sse_sender) = loop {
        if let Ok(pair) =
            alvr_ipc::ipc_listen("/tmp/alvr_driver_request.sock", "/tmp/alvr_driver_sse.sock")
        {
            break pair;
        }
        time::sleep(Duration::from_millis(100)).await;
    };

    *DRIVER_SENDER.lock() = Some(ipc_sse_sender);

    // todo: this will not be needed anymore if defining devices in a vector in the settings
    let mut tracked_device_types = vec![];

    loop {
        ipc_server.serve_non_blocking(|request| match request {
            DriverRequest::GetInitializationConfig => {
                let settings = SESSION_MANAGER.lock().get().to_settings();

                let mut tracked_devices = vec![];
                tracked_device_types = vec![];

                if !settings.headset.tracking_ref_only {
                    tracked_devices.push(TrackedDeviceConfig {
                        serial_number: settings.headset.serial_number,
                        device_type: TrackedDeviceType::Hmd,
                    });
                    tracked_device_types.push(TrackedDeviceType::Hmd);
                }
                if let Switch::Enabled(controllers) = settings.headset.controllers {
                    tracked_devices.push(TrackedDeviceConfig {
                        serial_number: controllers.serial_number.clone(),
                        device_type: TrackedDeviceType::LeftHand,
                    });
                    tracked_device_types.push(TrackedDeviceType::LeftHand);

                    tracked_devices.push(TrackedDeviceConfig {
                        serial_number: controllers.serial_number,
                        device_type: TrackedDeviceType::RightHand,
                    });
                    tracked_device_types.push(TrackedDeviceType::RightHand);
                }

                ResponseForDriver::InitializationConfig {
                    tracked_devices,
                    presentation: cfg!(windows) && !settings.headset.tracking_ref_only,
                }
            }
            DriverRequest::GetExtraProperties(device_index) => {
                let settings = SESSION_MANAGER.lock().get().to_settings();
                match tracked_device_types[device_index as usize] {
                    TrackedDeviceType::Hmd => {
                        let props = vec![
                            (
                                "Prop_TrackingSystemName_String".into(),
                                OpenvrPropValue::String(settings.headset.tracking_system_name),
                            ),
                            (
                                "Prop_ModelNumber_String".into(),
                                OpenvrPropValue::String(settings.headset.model_number),
                            ),
                            (
                                "Prop_ManufacturerName_String".into(),
                                OpenvrPropValue::String(settings.headset.manufacturer_name),
                            ),
                            (
                                "Prop_RenderModelName_String".into(),
                                OpenvrPropValue::String(settings.headset.render_model_name),
                            ),
                            (
                                "Prop_RegisteredDeviceType_String".into(),
                                OpenvrPropValue::String(settings.headset.registered_device_type),
                            ),
                            (
                                "Prop_DriverVersion_String".into(),
                                OpenvrPropValue::String(settings.headset.driver_version),
                            ),
                            (
                                "Prop_UserIpdMeters_Float".into(),
                                OpenvrPropValue::Float(0.63),
                            ),
                            (
                                "Prop_UserHeadToEyeDepthMeters_Float".into(),
                                OpenvrPropValue::Float(0.),
                            ),
                            (
                                "Prop_DisplayFrequency_Float".into(),
                                OpenvrPropValue::Float(60.0),
                            ),
                            (
                                "Prop_SecondsFromVsyncToPhotons_Float".into(),
                                OpenvrPropValue::Float(0.0),
                            ),
                            (
                                "Prop_CurrentUniverseId_Uint64".into(),
                                OpenvrPropValue::Uint64(settings.headset.universe_id),
                            ),
                            #[cfg(windows)]
                            (
                                // avoid "not fullscreen" warnings from vrmonitor
                                "Prop_IsOnDesktop_Bool".into(),
                                OpenvrPropValue::Bool(false),
                            ),
                            #[cfg(windows)]
                            (
                                // Manually send VSync events on direct mode.
                                // https://github.com/ValveSoftware/virtual_display/issues/1
                                "Prop_DriverDirectModeSendsVsyncEvents_Bool".into(),
                                OpenvrPropValue::Bool(true),
                            ),
                            (
                                "Prop_DeviceProvidesBatteryStatus_Bool".into(),
                                OpenvrPropValue::Bool(true),
                            ),
                            (
                                "Prop_NamedIconPathDeviceOff_String".into(),
                                OpenvrPropValue::String(
                                    "{oculus}/icons/quest_headset_off.png".into(),
                                ),
                            ),
                            (
                                "Prop_NamedIconPathDeviceSearching_String".into(),
                                OpenvrPropValue::String(
                                    "{oculus}/icons/quest_headset_searching.gif".into(),
                                ),
                            ),
                            (
                                "Prop_NamedIconPathDeviceSearchingAlert_String".into(),
                                OpenvrPropValue::String(
                                    "{oculus}/icons/quest_headset_alert_searching.gif".into(),
                                ),
                            ),
                            (
                                "Prop_NamedIconPathDeviceReady_String".into(),
                                OpenvrPropValue::String(
                                    "{oculus}/icons/quest_headset_ready.png".into(),
                                ),
                            ),
                            (
                                "Prop_NamedIconPathDeviceReadyAlert_String".into(),
                                OpenvrPropValue::String(
                                    "{oculus}/icons/quest_headset_ready_alert.png".into(),
                                ),
                            ),
                            (
                                "Prop_NamedIconPathDeviceStandby_String".into(),
                                OpenvrPropValue::String(
                                    "{oculus}/icons/quest_headset_standby.png".into(),
                                ),
                            ),
                        ];

                        ResponseForDriver::ExtraProperties(props)
                    }
                    TrackedDeviceType::LeftHand => todo!(),
                    TrackedDeviceType::RightHand => todo!(),
                    TrackedDeviceType::GenericTracker => todo!(),
                }
            }
            DriverRequest::GetButtonLayout(_) => todo!(),
            DriverRequest::CreateSwapchain {
                images_count,
                width,
                height,
                format,
                sample_count,
            } => todo!(),
            DriverRequest::DestroySwapchain { id } => todo!(),
            DriverRequest::GetNextSwapchainIndex { id } => todo!(),
            DriverRequest::PresentLayers(_) => todo!(),
            DriverRequest::Haptics {
                device_index,
                duration,
                frequency,
                amplitude,
            } => todo!(),
        })?;

        time::sleep(Duration::from_millis(1)).await;
    }
}

pub async fn driver_lifecycle_loop() {
    loop {
        alvr_common::show_err(driver_pipeline().await);
    }
}
