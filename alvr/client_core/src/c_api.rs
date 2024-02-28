use crate::{
    opengl::{self, RenderViewInput},
    storage, ClientCapabilities, ClientCoreEvent,
};
use alvr_common::{
    debug, error,
    glam::{Quat, UVec2, Vec2, Vec3},
    info,
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    warn, DeviceMotion, Fov, Pose,
};
use alvr_packets::{ButtonEntry, ButtonValue, FaceData, Tracking};
use alvr_session::{CodecType, FoveatedEncodingConfig};
use std::{
    collections::VecDeque,
    ffi::{c_char, c_void, CStr, CString},
    ptr, slice,
    time::{Duration, Instant},
};

// Core interface:

struct ReconstructedNal {
    timestamp_ns: u64,
    data: Vec<u8>,
}

static HUD_MESSAGE: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("".into()));
static SETTINGS: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("".into()));
static NAL_QUEUE: Lazy<Mutex<VecDeque<ReconstructedNal>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

#[repr(C)]
pub struct AlvrClientCapabilities {
    default_view_width: u32,
    default_view_height: u32,
    external_decoder: bool,
    refresh_rates: *const f32,
    refresh_rates_count: i32,
    foveated_encoding: bool,
    encoder_high_profile: bool,
    encoder_10_bits: bool,
    encoder_av1: bool,
}

#[repr(u8)]
pub enum AlvrCodec {
    H264 = 0,
    Hevc = 1,
    AV1 = 2,
}

#[repr(u8)]
pub enum AlvrEvent {
    HudMessageUpdated,
    StreamingStarted {
        view_width: u32,
        view_height: u32,
        refresh_rate_hint: f32,
        enable_foveated_encoding: bool,
    },
    StreamingStopped,
    Haptics {
        device_id: u64,
        duration_s: f32,
        frequency: f32,
        amplitude: f32,
    },
    // Note: All subsequent DecoderConfig events should be ignored until reconnection
    DecoderConfig {
        codec: AlvrCodec,
    },
    FrameReady,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrFov {
    left: f32,
    right: f32,
    up: f32,
    down: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AlvrQuat {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct AlvrPose {
    orientation: AlvrQuat,
    position: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct AlvrDeviceMotion {
    device_id: u64,
    pose: AlvrPose,
    linear_velocity: [f32; 3],
    angular_velocity: [f32; 3],
}

#[allow(dead_code)]
#[repr(C)]
pub enum AlvrButtonValue {
    Binary(bool),
    Scalar(f32),
}

#[allow(dead_code)]
#[repr(C)]
pub enum AlvrLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[no_mangle]
pub unsafe extern "C" fn alvr_path_string_to_id(path: *const c_char) -> u64 {
    alvr_common::hash_string(CStr::from_ptr(path).to_str().unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn alvr_log(level: AlvrLogLevel, message: *const c_char) {
    let message = CStr::from_ptr(message).to_str().unwrap();
    match level {
        AlvrLogLevel::Error => error!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Warn => warn!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Info => info!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Debug => debug!("[ALVR NATIVE] {message}"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_log_time(tag: *const c_char) {
    let tag = CStr::from_ptr(tag).to_str().unwrap();
    error!("[ALVR NATIVE] {tag}: {:?}", Instant::now());
}

fn string_to_c_str(buffer: *mut c_char, value: &str) -> u64 {
    let cstring = CString::new(value).unwrap();
    if !buffer.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(cstring.as_ptr(), buffer, cstring.as_bytes_with_nul().len());
        }
    }

    cstring.as_bytes_with_nul().len() as u64
}

#[no_mangle]
pub extern "C" fn alvr_mdns_service(service_buffer: *mut c_char) -> u64 {
    string_to_c_str(service_buffer, alvr_sockets::MDNS_SERVICE_TYPE)
}

// To make sure the value is correct, call after alvr_initialize()
#[no_mangle]
pub extern "C" fn alvr_hostname(hostname_buffer: *mut c_char) -> u64 {
    string_to_c_str(hostname_buffer, &storage::Config::load().hostname)
}

// To make sure the value is correct, call after alvr_initialize()
#[no_mangle]
pub extern "C" fn alvr_protocol_id(protocol_buffer: *mut c_char) -> u64 {
    string_to_c_str(protocol_buffer, &storage::Config::load().protocol_id)
}

/// NB: for android, `context` must be thread safe.
#[no_mangle]
pub unsafe extern "C" fn alvr_initialize(
    #[cfg(target_os = "android")] java_vm: *mut c_void,
    #[cfg(target_os = "android")] context: *mut c_void,
    capabilities: AlvrClientCapabilities,
) {
    #[cfg(target_os = "android")]
    ndk_context::initialize_android_context(java_vm, context);

    let default_view_resolution = UVec2::new(
        capabilities.default_view_width,
        capabilities.default_view_height,
    );

    let refresh_rates = slice::from_raw_parts(
        capabilities.refresh_rates,
        capabilities.refresh_rates_count as _,
    )
    .to_vec();

    crate::initialize(ClientCapabilities {
        default_view_resolution,
        external_decoder: capabilities.external_decoder,
        refresh_rates,
        foveated_encoding: capabilities.foveated_encoding,
        encoder_high_profile: capabilities.encoder_high_profile,
        encoder_10_bits: capabilities.encoder_10_bits,
        encoder_av1: capabilities.encoder_av1,
    });
}

#[no_mangle]
pub unsafe extern "C" fn alvr_destroy() {
    crate::destroy();

    #[cfg(target_os = "android")]
    ndk_context::release_android_context();
}

#[no_mangle]
pub extern "C" fn alvr_resume() {
    crate::resume();
}

#[no_mangle]
pub extern "C" fn alvr_pause() {
    crate::pause();
}

/// Returns true if there was a new event
#[no_mangle]
pub extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent) -> bool {
    if let Some(event) = crate::poll_event() {
        let event = match event {
            ClientCoreEvent::UpdateHudMessage(message) => {
                *HUD_MESSAGE.lock() = message;

                AlvrEvent::HudMessageUpdated
            }
            ClientCoreEvent::StreamingStarted {
                settings,
                negotiated_config,
            } => {
                *SETTINGS.lock() = serde_json::to_string(&settings).unwrap();

                AlvrEvent::StreamingStarted {
                    view_width: negotiated_config.view_resolution.x,
                    view_height: negotiated_config.view_resolution.y,
                    refresh_rate_hint: negotiated_config.refresh_rate_hint,
                    enable_foveated_encoding: negotiated_config.enable_foveated_encoding,
                }
            }
            ClientCoreEvent::StreamingStopped => AlvrEvent::StreamingStopped,
            ClientCoreEvent::Haptics {
                device_id,
                duration,
                frequency,
                amplitude,
            } => AlvrEvent::Haptics {
                device_id,
                duration_s: duration.as_secs_f32(),
                frequency,
                amplitude,
            },
            ClientCoreEvent::DecoderConfig { codec, config_nal } => {
                NAL_QUEUE.lock().push_back(ReconstructedNal {
                    timestamp_ns: 0,
                    data: config_nal,
                });

                AlvrEvent::DecoderConfig {
                    codec: match codec {
                        CodecType::H264 => AlvrCodec::H264,
                        CodecType::Hevc => AlvrCodec::Hevc,
                        CodecType::AV1 => AlvrCodec::AV1,
                    },
                }
            }
            ClientCoreEvent::FrameReady { timestamp, nal } => {
                NAL_QUEUE.lock().push_back(ReconstructedNal {
                    timestamp_ns: timestamp.as_nanos() as _,
                    data: nal,
                });

                AlvrEvent::FrameReady
            }
        };

        unsafe { *out_event = event };

        true
    } else {
        false
    }
}

// Settings will be updated after receiving StreamingStarted event
#[no_mangle]
pub extern "C" fn alvr_get_settings_json(buffer: *mut c_char) -> u64 {
    string_to_c_str(buffer, &SETTINGS.lock())
}

/// Call only with external decoder
/// Returns the number of bytes of the next nal, or 0 if there are no nals ready.
/// If out_nal or out_timestamp_ns is null, no nal is dequeued. Use to get the nal allocation size.
/// Returns out_timestamp_ns == 0 if config NAL.
#[no_mangle]
pub extern "C" fn alvr_poll_nal(out_nal: *mut c_char, out_timestamp_ns: *mut u64) -> u64 {
    let mut queue_lock = NAL_QUEUE.lock();
    if let Some(ReconstructedNal { timestamp_ns, data }) = queue_lock.pop_front() {
        let nal_size = data.len();
        if !out_nal.is_null() && !out_timestamp_ns.is_null() {
            unsafe {
                ptr::copy_nonoverlapping(data.as_ptr(), out_nal as _, nal_size);
                *out_timestamp_ns = timestamp_ns;
            }
        } else {
            queue_lock.push_front(ReconstructedNal { timestamp_ns, data })
        }

        nal_size as u64
    } else {
        0
    }
}

// Returns the length of the message. message_buffer can be null.
#[no_mangle]
pub extern "C" fn alvr_hud_message(message_buffer: *mut c_char) -> u64 {
    let cstring = CString::new(HUD_MESSAGE.lock().clone()).unwrap();
    if !message_buffer.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(
                cstring.as_ptr(),
                message_buffer,
                cstring.as_bytes_with_nul().len(),
            );
        }
    }

    cstring.as_bytes_with_nul().len() as u64
}

#[no_mangle]
pub unsafe extern "C" fn alvr_send_views_config(fov: *const AlvrFov, ipd_m: f32) {
    let fov = slice::from_raw_parts(fov, 2);
    let fov = [
        Fov {
            left: fov[0].left,
            right: fov[0].right,
            up: fov[0].up,
            down: fov[0].down,
        },
        Fov {
            left: fov[1].left,
            right: fov[1].right,
            up: fov[1].up,
            down: fov[1].down,
        },
    ];

    crate::send_views_config(fov, ipd_m);
}

#[no_mangle]
pub extern "C" fn alvr_send_battery(device_id: u64, gauge_value: f32, is_plugged: bool) {
    crate::send_battery(device_id, gauge_value, is_plugged);
}

#[no_mangle]
pub extern "C" fn alvr_send_playspace(width: f32, height: f32) {
    if width != 0.0 && height != 0.0 {
        crate::send_playspace(Some(Vec2::new(width, height)));
    } else {
        crate::send_playspace(None);
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_button(path_id: u64, value: AlvrButtonValue) {
    let value = match value {
        AlvrButtonValue::Binary(value) => ButtonValue::Binary(value),
        AlvrButtonValue::Scalar(value) => ButtonValue::Scalar(value),
    };

    crate::send_buttons(vec![ButtonEntry { path_id, value }]);
}

/// hand_skeleton:
/// * outer ptr: array of 2 (can be null);
/// * inner ptr: array of 26 (can be null if hand is absent)
/// eye_gazes:
/// * outer ptr: array of 2 (can be null);
/// * inner ptr: pose (can be null if eye gaze is absent)
#[no_mangle]
pub extern "C" fn alvr_send_tracking(
    target_timestamp_ns: u64,
    device_motions: *const AlvrDeviceMotion,
    device_motions_count: u64,
    hand_skeletons: *const *const AlvrPose,
    eye_gazes: *const *const AlvrPose,
) {
    fn from_capi_quat(quat: AlvrQuat) -> Quat {
        Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w)
    }

    let mut raw_motions = vec![AlvrDeviceMotion::default(); device_motions_count as _];
    unsafe {
        ptr::copy_nonoverlapping(
            device_motions,
            raw_motions.as_mut_ptr(),
            device_motions_count as _,
        );
    }

    let device_motions = raw_motions
        .into_iter()
        .map(|motion| {
            (
                motion.device_id,
                DeviceMotion {
                    pose: Pose {
                        orientation: from_capi_quat(motion.pose.orientation),
                        position: Vec3::from_slice(&motion.pose.position),
                    },
                    linear_velocity: Vec3::from_slice(&motion.linear_velocity),
                    angular_velocity: Vec3::from_slice(&motion.angular_velocity),
                },
            )
        })
        .collect::<Vec<_>>();

    let hand_skeletons = if !hand_skeletons.is_null() {
        let hand_skeletons = unsafe { slice::from_raw_parts(hand_skeletons, 2) };
        let hand_skeletons = hand_skeletons
            .iter()
            .map(|&hand_skeleton| {
                if !hand_skeleton.is_null() {
                    let hand_skeleton = unsafe { slice::from_raw_parts(hand_skeleton, 26) };

                    let mut array = [Pose::default(); 26];

                    for (pose, capi_pose) in array.iter_mut().zip(hand_skeleton.iter()) {
                        *pose = Pose {
                            orientation: from_capi_quat(capi_pose.orientation),
                            position: Vec3::from_slice(&capi_pose.position),
                        };
                    }

                    Some(array)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        [hand_skeletons[0], hand_skeletons[1]]
    } else {
        [None, None]
    };

    let eye_gazes = if !eye_gazes.is_null() {
        let eye_gazes = unsafe { slice::from_raw_parts(eye_gazes, 2) };
        let eye_gazes = eye_gazes
            .iter()
            .map(|&eye_gaze| {
                if !eye_gaze.is_null() {
                    let eye_gaze = unsafe { &*eye_gaze };

                    Some(Pose {
                        orientation: from_capi_quat(eye_gaze.orientation),
                        position: Vec3::from_slice(&eye_gaze.position),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        [eye_gazes[0], eye_gazes[1]]
    } else {
        [None, None]
    };

    let tracking = Tracking {
        target_timestamp: Duration::from_nanos(target_timestamp_ns),
        device_motions,
        hand_skeletons,
        face_data: FaceData {
            eye_gazes,
            ..Default::default()
        },
    };

    crate::send_tracking(tracking);
}

#[no_mangle]
pub extern "C" fn alvr_get_head_prediction_offset_ns() -> u64 {
    crate::get_head_prediction_offset().as_nanos() as _
}

#[no_mangle]
pub extern "C" fn alvr_get_tracker_prediction_offset_ns() -> u64 {
    crate::get_tracker_prediction_offset().as_nanos() as _
}

#[no_mangle]
pub extern "C" fn alvr_report_submit(target_timestamp_ns: u64, vsync_queue_ns: u64) {
    crate::report_submit(
        Duration::from_nanos(target_timestamp_ns),
        Duration::from_nanos(vsync_queue_ns),
    );
}

/// Call only with external decoder
#[no_mangle]
pub extern "C" fn alvr_request_idr() {
    crate::request_idr();
}

/// Call only with external decoder
#[no_mangle]
pub extern "C" fn alvr_report_frame_decoded(target_timestamp_ns: u64) {
    crate::report_frame_decoded(Duration::from_nanos(target_timestamp_ns as _));
}

/// Call only with external decoder
#[no_mangle]
pub extern "C" fn alvr_report_compositor_start(target_timestamp_ns: u64) {
    crate::report_compositor_start(Duration::from_nanos(target_timestamp_ns as _));
}

/// Call only with internal decoder (Android only)
/// Returns frame timestamp in nanoseconds or -1 if no frame available. Returns an AHardwareBuffer
/// from out_buffer.
#[allow(unused_variables)]
#[no_mangle]
pub unsafe extern "C" fn alvr_get_frame(out_buffer: *mut *mut std::ffi::c_void) -> i64 {
    if let Some((timestamp, buffer)) = crate::decoder::get_frame() {
        *out_buffer = buffer;

        timestamp.as_nanos() as _
    } else {
        -1
    }
}

// OpenGL-related interface

#[repr(C)]
pub struct AlvrViewInput {
    orientation: AlvrQuat,
    position: [f32; 3],
    fov: AlvrFov,
    swapchain_index: u32,
}

#[repr(C)]
pub struct AlvrStreamConfig {
    pub view_resolution_width: u32,
    pub view_resolution_height: u32,
    pub swapchain_textures: *mut *const u32,
    pub swapchain_length: u32,
    pub enable_foveation: bool,
    pub foveation_center_size_x: f32,
    pub foveation_center_size_y: f32,
    pub foveation_center_shift_x: f32,
    pub foveation_center_shift_y: f32,
    pub foveation_edge_ratio_x: f32,
    pub foveation_edge_ratio_y: f32,
}

#[no_mangle]
pub extern "C" fn alvr_initialize_opengl() {
    opengl::initialize();
}

#[no_mangle]
pub extern "C" fn alvr_destroy_opengl() {
    opengl::destroy();
}

unsafe fn convert_swapchain_array(
    swapchain_textures: *mut *const u32,
    swapchain_length: u32,
) -> [Vec<u32>; 2] {
    let swapchain_length = swapchain_length as usize;
    let mut left_swapchain = vec![0; swapchain_length];
    ptr::copy_nonoverlapping(
        *swapchain_textures,
        left_swapchain.as_mut_ptr(),
        swapchain_length,
    );
    let mut right_swapchain = vec![0; swapchain_length];
    ptr::copy_nonoverlapping(
        *swapchain_textures.offset(1),
        right_swapchain.as_mut_ptr(),
        swapchain_length,
    );

    [left_swapchain, right_swapchain]
}

#[no_mangle]
pub unsafe extern "C" fn alvr_resume_opengl(
    preferred_view_width: u32,
    preferred_view_height: u32,
    swapchain_textures: *mut *const u32,
    swapchain_length: u32,
) {
    opengl::resume(
        UVec2::new(preferred_view_width, preferred_view_height),
        convert_swapchain_array(swapchain_textures, swapchain_length),
    );
}

#[no_mangle]
pub extern "C" fn alvr_pause_opengl() {
    opengl::pause();
}

#[no_mangle]
pub unsafe extern "C" fn alvr_update_hud_message_opengl(message: *const c_char) {
    opengl::update_hud_message(CStr::from_ptr(message).to_str().unwrap());
}

#[no_mangle]
pub unsafe extern "C" fn alvr_start_stream_opengl(config: AlvrStreamConfig) {
    let view_resolution = UVec2::new(config.view_resolution_width, config.view_resolution_height);
    let swapchain_textures =
        convert_swapchain_array(config.swapchain_textures, config.swapchain_length);
    let foveated_encoding = config.enable_foveation.then_some(FoveatedEncodingConfig {
        force_enable: true,
        center_size_x: config.foveation_center_size_x,
        center_size_y: config.foveation_center_size_y,
        center_shift_x: config.foveation_center_shift_x,
        center_shift_y: config.foveation_center_shift_y,
        edge_ratio_x: config.foveation_edge_ratio_x,
        edge_ratio_y: config.foveation_edge_ratio_y,
    });

    opengl::start_stream(view_resolution, swapchain_textures, foveated_encoding, true);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_render_lobby_opengl(view_inputs: *const AlvrViewInput) {
    let view_inputs = [
        {
            let o = (*view_inputs).orientation;
            let f = (*view_inputs).fov;
            RenderViewInput {
                pose: Pose {
                    orientation: Quat::from_xyzw(o.x, o.y, o.z, o.w),
                    position: Vec3::from_array((*view_inputs).position),
                },
                fov: Fov {
                    left: f.left,
                    right: f.right,
                    up: f.up,
                    down: f.down,
                },
                swapchain_index: (*view_inputs).swapchain_index,
            }
        },
        {
            let o = (*view_inputs.offset(1)).orientation;
            let f = (*view_inputs.offset(1)).fov;
            RenderViewInput {
                pose: Pose {
                    orientation: Quat::from_xyzw(o.x, o.y, o.z, o.w),
                    position: Vec3::from_array((*view_inputs.offset(1)).position),
                },
                fov: Fov {
                    left: f.left,
                    right: f.right,
                    up: f.up,
                    down: f.down,
                },
                swapchain_index: (*view_inputs.offset(1)).swapchain_index,
            }
        },
    ];

    opengl::render_lobby(view_inputs);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_render_stream_opengl(
    hardware_buffer: *mut c_void,
    swapchain_indices: *const u32,
) {
    opengl::render_stream(
        hardware_buffer,
        [*swapchain_indices, *swapchain_indices.offset(1)],
    );
}
