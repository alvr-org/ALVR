mod graphics;
mod props;
mod tracking;

#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    non_snake_case,
    clippy::unseparated_literal_suffix
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;

use crate::{
    input_mapping::REGISTERED_BUTTON_SET, logging_backend, ServerCoreContext, ServerCoreEvent,
    SERVER_DATA_MANAGER,
};
use alvr_common::{
    error, once_cell::sync::Lazy, parking_lot::RwLock, warn, BUTTON_INFO, HAND_LEFT_ID,
    HAND_RIGHT_ID,
};
use alvr_filesystem as afs;
use alvr_packets::{ButtonValue, Haptics};
use alvr_session::CodecType;
use std::{
    ffi::{c_char, c_void, CString},
    ptr, thread,
    time::{Duration, Instant},
};

static FILESYSTEM_LAYOUT: Lazy<afs::Layout> = Lazy::new(|| {
    afs::filesystem_layout_from_openvr_driver_root_dir(
        &alvr_server_io::get_driver_dir_from_registered().unwrap(),
    )
});

static SERVER_CORE_CONTEXT: Lazy<RwLock<Option<ServerCoreContext>>> = Lazy::new(|| {
    logging_backend::init_logging();

    unsafe {
        g_sessionPath = CString::new(FILESYSTEM_LAYOUT.session().to_string_lossy().to_string())
            .unwrap()
            .into_raw();
        g_driverRootDir = CString::new(
            FILESYSTEM_LAYOUT
                .openvr_driver_root_dir
                .to_string_lossy()
                .to_string(),
        )
        .unwrap()
        .into_raw();
    };

    graphics::initialize_shaders();

    unsafe {
        LogError = Some(crate::c_api::alvr_log_error);
        LogWarn = Some(crate::c_api::alvr_log_warn);
        LogInfo = Some(crate::c_api::alvr_log_info);
        LogDebug = Some(crate::c_api::alvr_log_debug);
        LogPeriodically = Some(crate::c_api::alvr_log_periodically);
        PathStringToHash = Some(crate::c_api::alvr_path_to_id);

        CppInit();
    }

    RwLock::new(Some(ServerCoreContext::new()))
});

extern "C" fn driver_ready_idle(set_default_chap: bool) {
    thread::spawn(move || {
        unsafe { InitOpenvrClient() };

        if set_default_chap {
            // call this when inside a new thread. Calling this on the parent thread will crash
            // SteamVR
            unsafe {
                SetChaperoneArea(2.0, 2.0);
            }
        }

        if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
            context.start_connection();
        }

        let mut last_resync = Instant::now();
        loop {
            let event = if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
                match context.poll_event() {
                    Some(event) => event,
                    None => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                }
            } else {
                break;
            };

            match event {
                ServerCoreEvent::SetOpenvrProperty { device_id, prop } => unsafe {
                    SetOpenvrProperty(device_id, props::to_ffi_openvr_prop(prop))
                },
                ServerCoreEvent::ClientConnected => {
                    unsafe {
                        InitializeStreaming();
                        RequestDriverResync();
                    };
                }
                ServerCoreEvent::ClientDisconnected => unsafe { DeinitializeStreaming() },
                ServerCoreEvent::Battery(info) => unsafe {
                    SetBattery(info.device_id, info.gauge_value, info.is_plugged)
                },
                ServerCoreEvent::PlayspaceSync(bounds) => unsafe {
                    SetChaperoneArea(bounds.x, bounds.y)
                },
                ServerCoreEvent::ViewsConfig(config) => unsafe {
                    SetViewsConfig(FfiViewsConfig {
                        fov: [
                            FfiFov {
                                left: config.fov[0].left,
                                right: config.fov[0].right,
                                up: config.fov[0].up,
                                down: config.fov[0].down,
                            },
                            FfiFov {
                                left: config.fov[1].left,
                                right: config.fov[1].right,
                                up: config.fov[1].up,
                                down: config.fov[1].down,
                            },
                        ],
                        // todo: send full matrix to steamvr
                        ipd_m: config.local_view_transforms[1].position.x
                            - config.local_view_transforms[0].position.x,
                    });
                },
                ServerCoreEvent::Tracking {
                    tracking,
                    controllers_pose_time_offset,
                } => {
                    let controllers_config;
                    let track_body;
                    {
                        let data_lock = SERVER_DATA_MANAGER.read();
                        let headset_config = &data_lock.settings().headset;

                        controllers_config = headset_config.controllers.clone().into_option();
                        track_body = headset_config.body_tracking.enabled();
                    };

                    let track_controllers = controllers_config
                        .as_ref()
                        .map(|c| c.tracked)
                        .unwrap_or(false);

                    let left_openvr_hand_skeleton;
                    let right_openvr_hand_skeleton;
                    {
                        let data_manager_lock = SERVER_DATA_MANAGER.read();
                        let headset_config = &data_manager_lock.settings().headset;

                        left_openvr_hand_skeleton = tracking.hand_skeletons[0].map(|s| {
                            tracking::to_openvr_hand_skeleton(headset_config, *HAND_LEFT_ID, s)
                        });
                        right_openvr_hand_skeleton = tracking.hand_skeletons[1].map(|s| {
                            tracking::to_openvr_hand_skeleton(headset_config, *HAND_RIGHT_ID, s)
                        });
                    }

                    let enable_skeleton = controllers_config
                        .as_ref()
                        .map(|c| c.enable_skeleton)
                        .unwrap_or(false);
                    let ffi_left_hand_skeleton = enable_skeleton
                        .then_some(left_openvr_hand_skeleton)
                        .flatten()
                        .map(tracking::to_ffi_skeleton);
                    let ffi_right_hand_skeleton = enable_skeleton
                        .then_some(right_openvr_hand_skeleton)
                        .flatten()
                        .map(tracking::to_ffi_skeleton);

                    let ffi_motions = tracking
                        .device_motions
                        .iter()
                        .map(|(id, motion)| tracking::to_ffi_motion(*id, *motion))
                        .collect::<Vec<_>>();

                    let ffi_body_trackers =
                        tracking::to_ffi_body_trackers(&tracking.device_motions, track_body);

                    unsafe {
                        SetTracking(
                            tracking.target_timestamp.as_nanos() as _,
                            controllers_pose_time_offset.as_secs_f32(),
                            ffi_motions.as_ptr(),
                            ffi_motions.len() as _,
                            if let Some(skeleton) = &ffi_left_hand_skeleton {
                                skeleton
                            } else {
                                ptr::null()
                            },
                            if let Some(skeleton) = &ffi_right_hand_skeleton {
                                skeleton
                            } else {
                                ptr::null()
                            },
                            track_controllers.into(),
                            if let Some(body_trackers) = &ffi_body_trackers {
                                body_trackers.as_ptr()
                            } else {
                                ptr::null()
                            },
                            if let Some(body_trackers) = &ffi_body_trackers {
                                body_trackers.len() as _
                            } else {
                                0
                            },
                        )
                    };
                }
                ServerCoreEvent::Buttons(entries) => {
                    for entry in entries {
                        let value = match entry.value {
                            ButtonValue::Binary(value) => FfiButtonValue {
                                type_: FfiButtonType_BUTTON_TYPE_BINARY,
                                __bindgen_anon_1: FfiButtonValue__bindgen_ty_1 {
                                    binary: value.into(),
                                },
                            },

                            ButtonValue::Scalar(value) => FfiButtonValue {
                                type_: FfiButtonType_BUTTON_TYPE_SCALAR,
                                __bindgen_anon_1: FfiButtonValue__bindgen_ty_1 { scalar: value },
                            },
                        };
                        unsafe { SetButton(entry.path_id, value) };
                    }
                }
                ServerCoreEvent::RequestIDR => unsafe { RequestIDR() },
                ServerCoreEvent::CaptureFrame => unsafe { CaptureFrame() },
                ServerCoreEvent::GameRenderLatencyFeedback(game_latency) => {
                    if cfg!(target_os = "linux") && game_latency.as_secs_f32() > 0.25 {
                        let now = Instant::now();
                        if now.saturating_duration_since(last_resync).as_secs_f32() > 0.1 {
                            last_resync = now;
                            warn!("Desync detected. Attempting recovery.");
                            unsafe {
                                RequestDriverResync();
                            }
                        }
                    }
                }
                ServerCoreEvent::ShutdownPending => {
                    SERVER_CORE_CONTEXT.write().take();

                    unsafe { ShutdownSteamvr() };
                }
                ServerCoreEvent::RestartPending => {
                    if let Some(context) = SERVER_CORE_CONTEXT.write().take() {
                        context.restart();
                    }

                    unsafe { ShutdownSteamvr() };
                }
            }
        }

        unsafe { ShutdownOpenvrClient() };
    });
}

pub extern "C" fn register_buttons(device_id: u64) {
    for id in &*REGISTERED_BUTTON_SET {
        if let Some(info) = BUTTON_INFO.get(id) {
            if info.device_id == device_id {
                unsafe { RegisterButton(*id) };
            }
        } else {
            error!("Cannot register unrecognized button ID {id}");
        }
    }
}

extern "C" fn send_haptics(device_id: u64, duration_s: f32, frequency: f32, amplitude: f32) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        let haptics = Haptics {
            device_id,
            duration: Duration::from_secs_f32(f32::max(duration_s, 0.0)),
            frequency,
            amplitude,
        };

        context.send_haptics(haptics);
    }
}

extern "C" fn set_video_config_nals(buffer_ptr: *const u8, len: i32, codec: i32) {
    let codec = if codec == 0 {
        CodecType::H264
    } else if codec == 1 {
        CodecType::Hevc
    } else {
        CodecType::AV1
    };

    let mut config_buffer = vec![0; len as usize];

    unsafe { ptr::copy_nonoverlapping(buffer_ptr, config_buffer.as_mut_ptr(), len as usize) };

    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.set_video_config_nals(config_buffer, codec);
    }
}

extern "C" fn send_video(timestamp_ns: u64, buffer_ptr: *mut u8, len: i32, is_idr: bool) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        let buffer = unsafe { std::slice::from_raw_parts(buffer_ptr, len as usize) };
        context.send_video_nal(Duration::from_nanos(timestamp_ns), buffer.to_vec(), is_idr);
    }
}

extern "C" fn get_dynamic_encoder_params() -> FfiDynamicEncoderParams {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        if let Some(params) = context.get_dynamic_encoder_params() {
            FfiDynamicEncoderParams {
                updated: 1,
                bitrate_bps: params.bitrate_bps,
                framerate: params.framerate,
            }
        } else {
            FfiDynamicEncoderParams::default()
        }
    } else {
        FfiDynamicEncoderParams::default()
    }
}

extern "C" fn report_composed(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_composed(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

extern "C" fn report_present(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_present(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

extern "C" fn wait_for_vsync() {
    // NB: don't sleep while locking SERVER_DATA_MANAGER or SERVER_CORE_CONTEXT
    let sleep_duration = if SERVER_DATA_MANAGER
        .read()
        .settings()
        .video
        .optimize_game_render_latency
    {
        SERVER_CORE_CONTEXT
            .read()
            .as_ref()
            .and_then(|ctx| ctx.duration_until_next_vsync())
    } else {
        None
    };

    if let Some(duration) = sleep_duration {
        thread::sleep(duration);
    }
}

pub extern "C" fn shutdown_driver() {
    SERVER_CORE_CONTEXT.write().take();
}

/// This is the SteamVR/OpenVR entry point
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    // Make sure the context is initialized, and initialize logging
    SERVER_CORE_CONTEXT.read().as_ref();

    GetSerialNumber = Some(props::get_serial_number);
    SetOpenvrProps = Some(props::set_device_openvr_props);
    RegisterButtons = Some(register_buttons);
    DriverReadyIdle = Some(driver_ready_idle);
    HapticsSend = Some(send_haptics);
    SetVideoConfigNals = Some(set_video_config_nals);
    VideoSend = Some(send_video);
    GetDynamicEncoderParams = Some(get_dynamic_encoder_params);
    ReportComposed = Some(report_composed);
    ReportPresent = Some(report_present);
    WaitForVSync = Some(wait_for_vsync);
    ShutdownRuntime = Some(shutdown_driver);

    CppOpenvrEntryPoint(interface_name, return_code)
}
