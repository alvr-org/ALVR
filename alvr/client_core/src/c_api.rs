use crate::ClientEvent;
use alvr_common::{
    glam::{Quat, UVec2, Vec2, Vec3},
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    prelude::*,
};
use alvr_events::ButtonValue;
use alvr_session::CodecType;
use alvr_sockets::{DeviceMotion, Fov, Tracking};
use std::{
    collections::VecDeque,
    ffi::{c_char, c_void, CStr},
    ptr, slice,
    time::{Duration, Instant},
};

struct ReconstructedNal {
    timestamp_ns: u64,
    data: Vec<u8>,
}
static NAL_QUEUE: Lazy<Mutex<VecDeque<ReconstructedNal>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

#[repr(u8)]
pub enum AlvrCodec {
    H264 = 0,
    H265 = 1,
}

#[repr(u8)]
pub enum AlvrEvent {
    StreamingStarted {
        view_width: u32,
        view_height: u32,
        fps: f32,
        oculus_foveation_level: i32,
        dynamic_oculus_foveation: bool,
        extra_latency: bool,
        controller_prediction_multiplier: f32,
    },
    StreamingStopped,
    Haptics {
        device_id: u64,
        duration_s: f32,
        frequency: f32,
        amplitude: f32,
    },
    CreateDecoder {
        codec: AlvrCodec,
    },
    FrameReady,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EyeFov {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
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
pub struct AlvrDeviceMotion {
    device_id: u64,
    orientation: AlvrQuat,
    position: [f32; 3],
    linear_velocity: [f32; 3],
    angular_velocity: [f32; 3],
}

#[cfg(target_os = "android")]
#[repr(C)]
pub struct AlvrEyeInput {
    orientation: AlvrQuat,
    position: [f32; 3],
    fov: EyeFov,
}

#[repr(C)]
pub struct OculusHand {
    enabled: bool,
    bone_rotations: [AlvrQuat; 19],
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

/// On non-Android platforms, java_vm and constext should be null.
/// NB: context must be thread safe.
#[allow(unused_variables)]
#[no_mangle]
pub unsafe extern "C" fn alvr_initialize(
    java_vm: *mut c_void,
    context: *mut c_void,
    recommended_view_width: u32,
    recommended_view_height: u32,
    refresh_rates: *const f32,
    refresh_rates_count: i32,
    external_decoder: bool,
) {
    #[cfg(target_os = "android")]
    ndk_context::initialize_android_context(java_vm, context);

    let recommended_view_resolution = UVec2::new(recommended_view_width, recommended_view_height);

    let supported_refresh_rates =
        slice::from_raw_parts(refresh_rates, refresh_rates_count as _).to_vec();

    crate::initialize(
        recommended_view_resolution,
        supported_refresh_rates,
        external_decoder,
    );
}

#[no_mangle]
pub extern "C" fn alvr_destroy() {
    crate::destroy();
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
            ClientEvent::StreamingStarted {
                view_resolution,
                fps,
                oculus_foveation_level,
                dynamic_oculus_foveation,
                extra_latency,
                controller_prediction_multiplier,
            } => AlvrEvent::StreamingStarted {
                view_width: view_resolution.x,
                view_height: view_resolution.y,
                fps,
                oculus_foveation_level: oculus_foveation_level as i32,
                dynamic_oculus_foveation,
                extra_latency,
                controller_prediction_multiplier,
            },
            ClientEvent::StreamingStopped => AlvrEvent::StreamingStopped,
            ClientEvent::Haptics {
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
            ClientEvent::CreateDecoder { codec, config_nal } => {
                NAL_QUEUE.lock().push_back(ReconstructedNal {
                    timestamp_ns: 0,
                    data: config_nal,
                });

                AlvrEvent::CreateDecoder {
                    codec: if matches!(codec, CodecType::H264) {
                        AlvrCodec::H264
                    } else {
                        AlvrCodec::H265
                    },
                }
            }
            ClientEvent::FrameReady { timestamp, nal } => {
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

#[no_mangle]
pub unsafe extern "C" fn alvr_send_views_config(fov: *const EyeFov, ipd_m: f32) {
    let fov = slice::from_raw_parts(fov, 2);
    let fov = [
        Fov {
            left: fov[0].left,
            right: fov[0].right,
            top: fov[0].top,
            bottom: fov[0].bottom,
        },
        Fov {
            left: fov[1].left,
            right: fov[1].right,
            top: fov[1].top,
            bottom: fov[1].bottom,
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
    crate::send_playspace(Vec2::new(width, height));
}

#[no_mangle]
pub extern "C" fn alvr_send_button(path_id: u64, value: AlvrButtonValue) {
    let value = match value {
        AlvrButtonValue::Binary(value) => ButtonValue::Binary(value),
        AlvrButtonValue::Scalar(value) => ButtonValue::Scalar(value),
    };

    crate::send_button(path_id, value);
}

#[no_mangle]
pub extern "C" fn alvr_send_tracking(
    target_timestamp_ns: u64,
    device_motions: *const AlvrDeviceMotion,
    device_motions_count: u64,
    left_oculus_hand: OculusHand,
    right_oculus_hand: OculusHand,
) {
    fn from_tracking_quat(quat: AlvrQuat) -> Quat {
        Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w)
    }

    fn from_oculus_hand(hand: OculusHand) -> Option<[Quat; 19]> {
        hand.enabled.then(|| {
            let vec = hand
                .bone_rotations
                .iter()
                .cloned()
                .map(from_tracking_quat)
                .collect::<Vec<_>>();

            let mut array = [Quat::IDENTITY; 19];
            array.copy_from_slice(&vec);

            array
        })
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
                    orientation: from_tracking_quat(motion.orientation),
                    position: Vec3::from_slice(&motion.position),
                    linear_velocity: Vec3::from_slice(&motion.linear_velocity),
                    angular_velocity: Vec3::from_slice(&motion.angular_velocity),
                },
            )
        })
        .collect::<Vec<_>>();

    let tracking = Tracking {
        target_timestamp: Duration::from_nanos(target_timestamp_ns),
        device_motions,
        left_hand_skeleton: from_oculus_hand(left_oculus_hand),
        right_hand_skeleton: from_oculus_hand(right_oculus_hand),
    };

    crate::send_tracking(tracking);
}

#[no_mangle]
pub extern "C" fn alvr_get_prediction_offset_ns() -> u64 {
    crate::get_prediction_offset().as_nanos() as _
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

/// Can be called before or after `alvr_initialize()`
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn alvr_initialize_opengl() {
    crate::initialize_opengl();
}

/// Must be called after `alvr_destroy()`. Can be skipped if the GL context is destroyed before
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn alvr_destroy_opengl() {
    crate::destroy_opengl();
}

#[cfg(target_os = "android")]
unsafe fn convert_swapchain_array(
    swapchain_textures: *mut *const i32,
    swapchain_length: i32,
) -> [Vec<i32>; 2] {
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

/// Must be called before `alvr_resume()`
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_resume_opengl(
    preferred_view_width: u32,
    preferred_view_height: u32,
    swapchain_textures: *mut *const i32,
    swapchain_length: i32,
) {
    crate::resume_opengl(
        UVec2::new(preferred_view_width, preferred_view_height),
        convert_swapchain_array(swapchain_textures, swapchain_length),
    );
}

/// Must be called after `alvr_pause()`
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn alvr_pause_opengl() {
    crate::pause_opengl();
}

#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_start_stream_opengl(
    swapchain_textures: *mut *const i32,
    swapchain_length: i32,
) {
    crate::start_stream_opengl(convert_swapchain_array(
        swapchain_textures,
        swapchain_length,
    ));
}

#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_render_lobby_opengl(
    eye_inputs: *const AlvrEyeInput,
    swapchain_indices: *const i32,
) {
    let eye_inputs = [
        {
            let o = (*eye_inputs).orientation;
            let f = (*eye_inputs).fov;
            crate::EyeInput {
                orientation: [o.x, o.y, o.z, o.w],
                position: (*eye_inputs).position,
                fovLeft: f.left,
                fovRight: f.right,
                fovTop: f.top,
                fovBottom: f.bottom,
            }
        },
        {
            let o = (*eye_inputs.offset(1)).orientation;
            let f = (*eye_inputs.offset(1)).fov;
            crate::EyeInput {
                orientation: [o.x, o.y, o.z, o.w],
                position: (*eye_inputs.offset(1)).position,
                fovLeft: f.left,
                fovRight: f.right,
                fovTop: f.top,
                fovBottom: f.bottom,
            }
        },
    ];

    crate::render_lobby_opengl(
        eye_inputs,
        [*swapchain_indices, *swapchain_indices.offset(1)],
    );
}

#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_render_stream_opengl(
    hardware_buffer: *mut c_void,
    swapchain_indices: *const i32,
) {
    crate::render_stream_opengl(
        hardware_buffer,
        [*swapchain_indices, *swapchain_indices.offset(1)],
    );
}

/// Call only with internal decoder (Android only)
/// Returns frame timestamp in nanoseconds or -1 if no frame available. Returns an AHardwareBuffer
/// from out_buffer.
#[cfg(target_os = "android")]
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
