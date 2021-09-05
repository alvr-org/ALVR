#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::missing_safety_doc
)]

use alvr_common::{Fov, OpenvrPropValue};
use alvr_ipc::{
    DriverConfigUpdate, DriverRequest, IpcClient, IpcSseReceiver, Layer, ResponseForDriver,
    SsePacket, TrackedDeviceType,
};
use core::slice;
use parking_lot::Mutex;
use std::ffi::CString;
use std::ptr;
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
}

fn log(message: &str) {
    let c_string = CString::new(message).unwrap();
    unsafe { drv::log(c_string.as_ptr()) };
}

extern "C" fn spawn_sse_receiver_loop() -> bool {
    if let Some(mut receiver) = IPC_CONNECTIONS.lock().sse_receiver.take() {
        thread::spawn(move || {
            while let Ok(maybe_message) = receiver.receive_non_blocking() {
                match maybe_message {
                    Some(message) => match message {
                        SsePacket::UpdateConfig(_) => todo!(),
                        SsePacket::PropertyChanged { name, value } => todo!(),
                        SsePacket::TrackingData {
                            trackers_data,
                            hand_skeleton_motions,
                            target_time_offset,
                        } => todo!(),
                        SsePacket::ButtonsData(data) => todo!(),
                        SsePacket::Restart => todo!(),
                    },
                    None => {
                        thread::sleep(Duration::from_millis(2));
                    }
                }
            }
        });

        true
    } else {
        false
    }
}

fn ipc_driver_config_to_driver(config: DriverConfigUpdate) -> drv::DriverConfigUpdate {
    drv::DriverConfigUpdate {
        preferred_view_width: config.preferred_view_size.0,
        preferred_view_height: config.preferred_view_size.1,
        fov: [
            drv::Fov {
                left: config.fov[0].left,
                right: config.fov[0].right,
                top: config.fov[0].top,
                bottom: config.fov[0].bottom,
            },
            drv::Fov {
                left: config.fov[1].left,
                right: config.fov[1].right,
                top: config.fov[1].top,
                bottom: config.fov[1].bottom,
            },
        ],
        ipd_m: config.ipd_m,
        fps: config.fps,
    }
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
                    left: drv_layer.fov[idx].left,
                    right: drv_layer.fov[idx].right,
                    top: drv_layer.fov[idx].top,
                    bottom: drv_layer.fov[idx].bottom,
                };

                // float uMin, vMin;
                // float uMax, vMax;
                let rect_offset = (drv_layer.bounds[idx].uMin, drv_layer.bounds[idx].vMin);
                let rect_size = (
                    drv_layer.bounds[idx].uMax - rect_offset.0,
                    drv_layer.bounds[idx].vMax - rect_offset.1,
                );

                layer_views.push(Layer {
                    orientation: todo!(),
                    fov,
                    swaphcain_id: drv_layer.swapchain_ids[idx],
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
    drv::get_initialization_config = Some(get_initialization_config);
    drv::set_extra_properties = Some(set_extra_properties);
    drv::create_swapchain = Some(create_swapchain);
    drv::destroy_swapchain = Some(destroy_swapchain);
    drv::next_swapchain_index = Some(next_swapchain_index);
    drv::present = Some(present);

    drv::entry_point(interface_name, return_code)
}
