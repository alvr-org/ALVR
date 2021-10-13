#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::missing_safety_doc
)]

use alvr_common::{Fov, OpenvrPropValue};
use alvr_ipc::{
    ButtonValue, DriverRequest, InputType, IpcClient, IpcSseReceiver, Layer, MotionData,
    ResponseForDriver, SsePacket, TrackedDeviceType, VideoConfigUpdate,
};
use core::slice;
use nalgebra::Vector3;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{ffi::c_void, os::raw::c_char, sync::Arc, thread, time::Duration};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use root as drv;
use root::vr;

include!(concat!(env!("OUT_DIR"), "/properties_mappings.rs"));

struct IpcConnections {
    client: Option<IpcClient<DriverRequest, ResponseForDriver>>,
    sse_receiver: Option<IpcSseReceiver<SsePacket>>,
}

lazy_static::lazy_static! {
    static ref IPC_CONNECTIONS: Arc<Mutex<IpcConnections>> = {
        let (client, sse_receiver) = if let Ok((client, sse_receiver)) = alvr_ipc::ipc_connect("driver") {
            (Some(client), Some(sse_receiver))
        } else {
            (None, None)
        };

        Arc::new(Mutex::new(IpcConnections {
            client,
            sse_receiver,
        }))
    };

    static ref IPC_RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));

    static ref BUTTON_COMPONENTS:
        Arc<Mutex<HashMap<u64, HashMap<String, vr::VRInputComponentHandle_t>>>> =
            Arc::new(Mutex::new(HashMap::new()));
}

fn log(message: &str) {
    let c_string = CString::new(message).unwrap();
    unsafe { drv::_log(c_string.as_ptr()) };
}

fn ipc_driver_config_to_driver(config: VideoConfigUpdate) -> drv::DriverConfigUpdate {
    drv::DriverConfigUpdate {
        preferred_view_width: config.preferred_view_size.0,
        preferred_view_height: config.preferred_view_size.1,
        fov: [
            vr::HmdRect2_t {
                vTopLeft: vr::HmdVector2_t {
                    v: [config.fov[0].left, config.fov[0].top],
                },
                vBottomRight: vr::HmdVector2_t {
                    v: [config.fov[0].right, config.fov[0].bottom],
                },
            },
            vr::HmdRect2_t {
                vTopLeft: vr::HmdVector2_t {
                    v: [config.fov[1].left, config.fov[1].top],
                },
                vBottomRight: vr::HmdVector2_t {
                    v: [config.fov[1].right, config.fov[1].bottom],
                },
            },
        ],
        ipd_m: config.ipd_m,
        fps: config.fps,
    }
}

fn set_property(device_index: u64, name: &str, value: OpenvrPropValue) {
    let key = match tracked_device_property_name_to_key(name) {
        Ok(key) => key,
        Err(e) => {
            log(&e);
            return;
        }
    };

    unsafe {
        match value {
            OpenvrPropValue::Bool(value) => drv::set_bool_property(device_index, key, value),
            OpenvrPropValue::Float(value) => drv::set_float_property(device_index, key, value),
            OpenvrPropValue::Int32(value) => drv::set_int32_property(device_index, key, value),
            OpenvrPropValue::Uint64(value) => drv::set_uint64_property(device_index, key, value),
            OpenvrPropValue::Vector3(value) => {
                drv::set_vec3_property(device_index, key, &vr::HmdVector3_t { v: value })
            }
            OpenvrPropValue::Double(value) => drv::set_double_property(device_index, key, value),
            OpenvrPropValue::String(value) => {
                let c_string = CString::new(value).unwrap();
                drv::set_string_property(device_index, key, c_string.as_ptr())
            }
        }
    };
}

fn set_tracking_data(
    motion_data: Vec<Option<MotionData>>,
    hand_skeleton_motions: [Option<[MotionData; 25]>; 2],
    target_time_offset: Duration,
) {
    let time_offset_s = target_time_offset.as_secs_f64();

    let data = motion_data
        .into_iter()
        .map(|maybe_data| {
            if let Some(data) = maybe_data {
                let p = data.position;
                let o = data.orientation;
                let lv = data.linear_velocity.unwrap_or_else(Vector3::zeros);
                let av = data.angular_velocity.unwrap_or_else(Vector3::zeros);

                vr::DriverPose_t {
                    poseTimeOffset: time_offset_s,
                    vecPosition: [p[0] as _, p[1] as _, p[2] as _],
                    vecVelocity: [lv[0] as _, lv[1] as _, lv[2] as _],
                    qRotation: vr::HmdQuaternion_t {
                        w: o[3] as _,
                        x: o[0] as _,
                        y: o[1] as _,
                        z: o[2] as _,
                    },
                    vecAngularVelocity: [av[0] as _, av[1] as _, av[2] as _],
                    result: vr::TrackingResult_Running_OK,
                    poseIsValid: true,
                    deviceIsConnected: true,
                    ..Default::default()
                }
            } else {
                vr::DriverPose_t {
                    result: vr::TrackingResult_Uninitialized,
                    deviceIsConnected: false,
                    ..Default::default()
                }
            }
        })
        .collect::<Vec<_>>();

    unsafe { drv::set_tracking_data(data.as_ptr(), data.len() as _) };
}

fn set_button_data(data: Vec<Vec<(String, ButtonValue)>>) {
    let components = BUTTON_COMPONENTS.lock();
    for (device_index, values) in data.into_iter().enumerate() {
        let device_components = &components[&(device_index as _)];
        for (path, value) in values {
            let component = device_components[&path];
            unsafe {
                match value {
                    ButtonValue::Boolean(value) => drv::update_boolean_component(component, value),
                    ButtonValue::Scalar(value) => drv::update_scalar_component(component, value),
                }
            }
        }
    }
}

extern "C" fn spawn_sse_receiver_loop() -> bool {
    if let Some(mut receiver) = IPC_CONNECTIONS.lock().sse_receiver.take() {
        thread::spawn(move || {
            while IPC_RUNNING.load(Ordering::Relaxed) {
                if let Ok(maybe_message) = receiver.receive_non_blocking() {
                    match maybe_message {
                        Some(message) => match message {
                            SsePacket::UpdateVideoConfig(config) => unsafe {
                                drv::update_config(ipc_driver_config_to_driver(config))
                            },
                            SsePacket::UpdateBattery {
                                device_index,
                                value,
                            } => todo!(),
                            SsePacket::PropertyChanged {
                                device_index,
                                name,
                                value,
                            } => set_property(device_index, &name, value),
                            SsePacket::TrackingData {
                                motion_data,
                                hand_skeleton_motions,
                                target_time_offset,
                            } => set_tracking_data(
                                motion_data,
                                *hand_skeleton_motions,
                                target_time_offset,
                            ),
                            SsePacket::ButtonsData(data) => set_button_data(data),
                            SsePacket::Restart => unsafe { drv::restart() },
                        },
                        None => {
                            thread::sleep(Duration::from_millis(2));
                        }
                    }
                } else {
                    break;
                }
            }

            unsafe { drv::vendor_event(vr::VREvent_DriverRequestedQuit) };
        });

        true
    } else {
        false
    }
}

extern "C" fn stop_sse_receiver() {
    IPC_RUNNING.store(false, Ordering::Relaxed);
}

extern "C" fn get_initialization_config() -> drv::InitializationConfig {
    if let Some(client) = &mut IPC_CONNECTIONS.lock().client {
        let response = client.request(&DriverRequest::GetInitializationConfig);
        if let Ok(ResponseForDriver::InitializationConfig {
            tracked_devices,
            display_config,
        }) = response
        {
            let mut tracked_device_serial_numbers = [[0; 20]; 10];
            let mut tracked_device_classes = [vr::TrackedDeviceClass_Invalid; 10];
            let mut controller_role = [vr::TrackedControllerRole_Invalid; 10];
            for idx in 0..tracked_devices.len() {
                let config = &tracked_devices[idx];

                let serial_number_cstring = CString::new(config.serial_number.clone()).unwrap();
                unsafe {
                    ptr::copy_nonoverlapping(
                        serial_number_cstring.as_ptr(),
                        tracked_device_serial_numbers[idx].as_mut_ptr(),
                        serial_number_cstring.as_bytes_with_nul().len(),
                    )
                };

                tracked_device_classes[idx] = match config.device_type {
                    TrackedDeviceType::Hmd => vr::TrackedDeviceClass_HMD,
                    TrackedDeviceType::LeftHand | TrackedDeviceType::RightHand => {
                        vr::TrackedDeviceClass_Controller
                    }
                    TrackedDeviceType::GenericTracker => vr::TrackedDeviceClass_GenericTracker,
                };

                controller_role[idx] = match config.device_type {
                    TrackedDeviceType::Hmd | TrackedDeviceType::GenericTracker => {
                        vr::TrackedControllerRole_Invalid
                    }
                    TrackedDeviceType::LeftHand => vr::TrackedControllerRole_LeftHand,
                    TrackedDeviceType::RightHand => vr::TrackedControllerRole_RightHand,
                }
            }

            let (presentation, config) = if let Some(display_config) = display_config {
                (
                    display_config.presentation,
                    ipc_driver_config_to_driver(display_config.config),
                )
            } else {
                (false, drv::DriverConfigUpdate::default())
            };

            return drv::InitializationConfig {
                tracked_device_serial_numbers,
                tracked_device_classes,
                controller_role,
                tracked_devices_count: tracked_devices.len() as _,
                presentation,
                config,
            };
        }
    }

    drv::InitializationConfig::default()
}

extern "C" fn set_extra_properties(device_index: u64) {
    if let Some(client) = &mut IPC_CONNECTIONS.lock().client {
        let response = client.request(&DriverRequest::GetExtraProperties(device_index));

        if let Ok(ResponseForDriver::ExtraProperties(props)) = response {
            for (name, value) in props {
                set_property(device_index, &name, value);
            }
        }
    }
}

extern "C" fn set_button_layout(device_index: u64) {
    if let Some(client) = &mut IPC_CONNECTIONS.lock().client {
        let response = client.request(&DriverRequest::GetButtonLayout(device_index));

        if let Ok(ResponseForDriver::ButtonLayout(layout)) = response {
            let components = layout
                .into_iter()
                .map(|(path, input_type)| {
                    let path_cstring = CString::new(path.clone()).unwrap();
                    let component = unsafe {
                        match input_type {
                            InputType::Boolean => {
                                drv::create_boolean_component(device_index, path_cstring.as_ptr())
                            }
                            InputType::NormalizedOneSided => drv::create_scalar_component(
                                device_index,
                                path_cstring.as_ptr(),
                                vr::VRScalarUnits_NormalizedOneSided,
                            ),
                            InputType::NormalizedTwoSided => drv::create_scalar_component(
                                device_index,
                                path_cstring.as_ptr(),
                                vr::VRScalarUnits_NormalizedTwoSided,
                            ),
                        }
                    };

                    (path, component)
                })
                .collect();

            BUTTON_COMPONENTS.lock().insert(device_index, components);
        }
    }
}

extern "C" fn send_haptics(device_index: u64, event: vr::VREvent_HapticVibration_t) {
    if let Some(client) = &mut IPC_CONNECTIONS.lock().client {
        client
            .request(&DriverRequest::Haptics {
                device_index,
                duration: Duration::from_secs_f32(event.fDurationSeconds),
                frequency: event.fFrequency,
                amplitude: event.fAmplitude,
            })
            .ok();
    }
}

extern "C" fn create_swapchain(
    pid: u32,
    desc: vr::IVRDriverDirectModeComponent_SwapTextureSetDesc_t,
) -> drv::SwapchainData {
    if let Some(client) = &mut IPC_CONNECTIONS.lock().client {
        let response = client.request(&DriverRequest::CreateSwapchain {
            images_count: 3,
            width: desc.nWidth,
            height: desc.nHeight,
            format: desc.nFormat,
            sample_count: desc.nSampleCount,
        });

        if let Ok(ResponseForDriver::Swapchain { id, textures }) = response {
            let mut texture_handles = [0; 3];
            texture_handles.copy_from_slice(&textures);

            return drv::SwapchainData {
                id,
                pid,
                texture_handles,
            };
        }
    }

    drv::SwapchainData::default()
}

extern "C" fn destroy_swapchain(id: u64) {
    if let Some(client) = &mut IPC_CONNECTIONS.lock().client {
        client.request(&DriverRequest::DestroySwapchain { id }).ok();
    }
}

extern "C" fn next_swapchain_index(id: u64) -> u32 {
    if let Some(client) = &mut IPC_CONNECTIONS.lock().client {
        let response = client.request(&DriverRequest::GetNextSwapchainIndex { id });
        if let Ok(ResponseForDriver::SwapchainIndex(idx)) = response {
            return idx as _;
        }
    }

    0
}

extern "C" fn present(layers: *const drv::Layer, count: u32) {
    if let Some(client) = &mut IPC_CONNECTIONS.lock().client {
        let drv_layers = unsafe { slice::from_raw_parts::<drv::Layer>(layers, count as _) };

        let mut layers = vec![];
        for drv_layer in drv_layers {
            let mut layer_views = vec![];
            for idx in 0..2 {
                let fov = Fov {
                    left: drv_layer.fov[idx].vTopLeft.v[0],
                    right: drv_layer.fov[idx].vBottomRight.v[0],
                    top: drv_layer.fov[idx].vTopLeft.v[1],
                    bottom: drv_layer.fov[idx].vBottomRight.v[1],
                };

                let rect_offset = (drv_layer.bounds[idx].uMin, drv_layer.bounds[idx].vMin);
                let rect_size = (
                    drv_layer.bounds[idx].uMax - rect_offset.0,
                    drv_layer.bounds[idx].vMax - rect_offset.1,
                );

                layer_views.push(Layer {
                    orientation: todo!(),
                    fov,
                    swapchain_id: drv_layer.swapchain_ids[idx],
                    rect_offset,
                    rect_size,
                });
            }

            layers.push(layer_views);
        }

        client.request(&DriverRequest::PresentLayers(layers)).ok();
    }
}

// Entry point. The entry point must live on the Rust side, since C symbols are not exported
#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    // Initialize funtion pointers
    drv::spawn_sse_receiver_loop = Some(spawn_sse_receiver_loop);
    drv::stop_sse_receiver = Some(stop_sse_receiver);
    drv::get_initialization_config = Some(get_initialization_config);
    drv::set_extra_properties = Some(set_extra_properties);
    drv::set_button_layout = Some(set_button_layout);
    drv::create_swapchain = Some(create_swapchain);
    drv::destroy_swapchain = Some(destroy_swapchain);
    drv::next_swapchain_index = Some(next_swapchain_index);
    drv::present = Some(present);

    drv::entry_point(interface_name, return_code)
}
