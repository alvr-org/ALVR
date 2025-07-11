#![expect(dead_code)]

use crate::{
    ClientCapabilities, ClientCoreContext, ClientCoreEvent, storage,
    video_decoder::{self, VideoDecoderConfig, VideoDecoderSource},
};
use alvr_common::{
    AlvrCodecType, AlvrFov, AlvrPose, AlvrQuat, AlvrViewParams, DeviceMotion, Pose, ViewParams,
    anyhow::Result,
    debug, error,
    glam::{UVec2, Vec2, Vec3},
    info,
    parking_lot::Mutex,
    warn,
};
use alvr_graphics::{
    GraphicsContext, LobbyRenderer, LobbyViewParams, SDR_FORMAT_GL, StreamRenderer,
    StreamViewParams,
};
use alvr_packets::{ButtonEntry, ButtonValue, FaceData, TrackingData};
use alvr_session::{
    CodecType, FoveatedEncodingConfig, MediacodecPropType, MediacodecProperty, UpscalingConfig,
};
use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char, c_void},
    ptr,
    rc::Rc,
    slice,
    time::{Duration, Instant},
};

static CLIENT_CORE_CONTEXT: Mutex<Option<ClientCoreContext>> = Mutex::new(None);
static HUD_MESSAGE: Mutex<String> = Mutex::new(String::new());
static SETTINGS: Mutex<String> = Mutex::new(String::new());
static SERVER_VERSION: Mutex<String> = Mutex::new(String::new());
static DECODER_CONFIG_BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);

// Core interface:

#[repr(C)]
pub struct AlvrClientCapabilities {
    default_view_width: u32,
    default_view_height: u32,
    refresh_rates: *const f32,
    refresh_rates_count: u64,
    foveated_encoding: bool,
    encoder_high_profile: bool,
    encoder_10_bits: bool,
    encoder_av1: bool,
    prefer_10bit: bool,
    prefer_full_range: bool,
    preferred_encoding_gamma: f32,
    prefer_hdr: bool,
}

#[repr(u8)]
pub enum AlvrEvent {
    HudMessageUpdated,
    StreamingStarted {
        view_width: u32,
        view_height: u32,
        refresh_rate_hint: f32,
        encoding_gamma: f32,
        enable_foveated_encoding: bool,
        enable_hdr: bool,
    },
    StreamingStopped,
    Haptics {
        device_id: u64,
        duration_s: f32,
        frequency: f32,
        amplitude: f32,
    },
    /// Note: All subsequent DecoderConfig events should be ignored until reconnection
    DecoderConfig {
        codec: AlvrCodecType,
    },
    // Unimplemented
    RealTimeConfig {},
}

#[repr(C)]
pub struct AlvrVideoFrameData {
    callback_context: *mut c_void,
    timestamp_ns: u64,
    buffer_ptr: *const u8,
    buffer_size: u64,
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
#[repr(u8)]
pub enum AlvrLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_initialize_logging() {
    crate::init_logging();
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_path_string_to_id(path: *const c_char) -> u64 {
    alvr_common::hash_string(unsafe { CStr::from_ptr(path) }.to_str().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_log(level: AlvrLogLevel, message: *const c_char) {
    let message = unsafe { CStr::from_ptr(message) }.to_str().unwrap();
    match level {
        AlvrLogLevel::Error => error!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Warn => warn!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Info => info!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Debug => debug!("[ALVR NATIVE] {message}"),
    }
}

#[unsafe(no_mangle)]
#[cfg_attr(not(debug_assertions), expect(unused_variables))]
pub extern "C" fn alvr_dbg_client_impl(message: *const c_char) {
    alvr_common::dbg_client_impl!("{}", unsafe { CStr::from_ptr(message) }.to_str().unwrap())
}

#[unsafe(no_mangle)]
#[cfg_attr(not(debug_assertions), expect(unused_variables))]
pub extern "C" fn alvr_dbg_decoder(message: *const c_char) {
    alvr_common::dbg_decoder!("{}", unsafe { CStr::from_ptr(message) }.to_str().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_log_time(tag: *const c_char) {
    let tag = unsafe { CStr::from_ptr(tag) }.to_str().unwrap();
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

#[unsafe(no_mangle)]
pub extern "C" fn alvr_mdns_service(service_buffer: *mut c_char) -> u64 {
    string_to_c_str(service_buffer, alvr_sockets::MDNS_SERVICE_TYPE)
}

/// To make sure the value is correct, call after alvr_initialize()
#[unsafe(no_mangle)]
pub extern "C" fn alvr_hostname(hostname_buffer: *mut c_char) -> u64 {
    string_to_c_str(hostname_buffer, &storage::Config::load().hostname)
}

/// To make sure the value is correct, call after alvr_initialize()
#[unsafe(no_mangle)]
pub extern "C" fn alvr_protocol_id(protocol_buffer: *mut c_char) -> u64 {
    string_to_c_str(protocol_buffer, &storage::Config::load().protocol_id)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn alvr_try_get_permission(permission: *const c_char) {
    alvr_system_info::try_get_permission(unsafe { CStr::from_ptr(permission) }.to_str().unwrap());
}

/// NB: for android, `context` must be thread safe.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn alvr_initialize_android_context(java_vm: *mut c_void, context: *mut c_void) {
    unsafe { ndk_context::initialize_android_context(java_vm, context) };
}

/// On android, alvr_initialize_android_context() must be called first, then alvr_initialize().
#[unsafe(no_mangle)]
pub extern "C" fn alvr_initialize(capabilities: AlvrClientCapabilities) {
    let default_view_resolution = UVec2::new(
        capabilities.default_view_width,
        capabilities.default_view_height,
    );

    let refresh_rates = unsafe {
        slice::from_raw_parts(
            capabilities.refresh_rates,
            capabilities.refresh_rates_count as usize,
        )
    }
    .to_vec();

    let capabilities = ClientCapabilities {
        default_view_resolution,
        refresh_rates,
        foveated_encoding: capabilities.foveated_encoding,
        encoder_high_profile: capabilities.encoder_high_profile,
        encoder_10_bits: capabilities.encoder_10_bits,
        encoder_av1: capabilities.encoder_av1,
        prefer_10bit: capabilities.prefer_10bit,
        prefer_full_range: capabilities.prefer_full_range,
        preferred_encoding_gamma: capabilities.preferred_encoding_gamma,
        prefer_hdr: capabilities.prefer_hdr,
    };
    *CLIENT_CORE_CONTEXT.lock() = Some(ClientCoreContext::new(capabilities));
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_destroy() {
    *CLIENT_CORE_CONTEXT.lock() = None;

    #[cfg(target_os = "android")]
    unsafe {
        ndk_context::release_android_context()
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_resume() {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.resume();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_pause() {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.pause();
    }
}

/// Returns true if there was a new event
#[unsafe(no_mangle)]
pub extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent) -> bool {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock()
        && let Some(event) = context.poll_event()
    {
        let event = match event {
            ClientCoreEvent::UpdateHudMessage(message) => {
                *HUD_MESSAGE.lock() = message;

                AlvrEvent::HudMessageUpdated
            }
            ClientCoreEvent::StreamingStarted(stream_config) => {
                *SETTINGS.lock() = serde_json::to_string(&stream_config.settings).unwrap();
                *SERVER_VERSION.lock() = stream_config.server_version.to_string();

                AlvrEvent::StreamingStarted {
                    view_width: stream_config.negotiated_config.view_resolution.x,
                    view_height: stream_config.negotiated_config.view_resolution.y,
                    refresh_rate_hint: stream_config.negotiated_config.refresh_rate_hint,
                    encoding_gamma: stream_config.negotiated_config.encoding_gamma,
                    enable_foveated_encoding: stream_config
                        .negotiated_config
                        .enable_foveated_encoding,
                    enable_hdr: stream_config.negotiated_config.enable_hdr,
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
                *DECODER_CONFIG_BUFFER.lock() = config_nal;

                AlvrEvent::DecoderConfig {
                    codec: match codec {
                        CodecType::H264 => AlvrCodecType::H264,
                        CodecType::Hevc => AlvrCodecType::Hevc,
                        CodecType::AV1 => AlvrCodecType::AV1,
                    },
                }
            }
            ClientCoreEvent::RealTimeConfig(_) => AlvrEvent::RealTimeConfig {},
        };

        unsafe { *out_event = event };

        true
    } else {
        false
    }
}

// Returns the length of the message. message_buffer can be null.
#[unsafe(no_mangle)]
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

/// Settings will be updated after receiving StreamingStarted event
#[unsafe(no_mangle)]
pub extern "C" fn alvr_get_settings_json(out_buffer: *mut c_char) -> u64 {
    string_to_c_str(out_buffer, &SETTINGS.lock())
}

/// Will be updated after receiving StreamingStarted event
#[unsafe(no_mangle)]
pub extern "C" fn alvr_get_server_version(out_buffer: *mut c_char) -> u64 {
    string_to_c_str(out_buffer, &SERVER_VERSION.lock())
}

/// Returns the number of bytes of the decoder_buffer
#[unsafe(no_mangle)]
pub extern "C" fn alvr_get_decoder_config(out_buffer: *mut c_char) -> u64 {
    let buffer = DECODER_CONFIG_BUFFER.lock();

    let size = buffer.len();

    if !out_buffer.is_null() {
        unsafe { ptr::copy_nonoverlapping(buffer.as_ptr(), out_buffer.cast(), size) }
    }

    size as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_send_battery(device_id: u64, gauge_value: f32, is_plugged: bool) {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.send_battery(device_id, gauge_value, is_plugged);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_send_playspace(width: f32, height: f32) {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.send_playspace(Some(Vec2::new(width, height)));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_send_active_interaction_profile(device_id: u64, profile_id: u64) {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.send_active_interaction_profile(device_id, profile_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_send_custom_interaction_profile(
    device_id: u64,
    input_ids_ptr: *const u64,
    input_ids_count: u64,
) {
    let input_ids = unsafe { slice::from_raw_parts(input_ids_ptr, input_ids_count as usize) };
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.send_custom_interaction_profile(device_id, input_ids.iter().cloned().collect());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_send_button(path_id: u64, value: AlvrButtonValue) {
    let value = match value {
        AlvrButtonValue::Binary(value) => ButtonValue::Binary(value),
        AlvrButtonValue::Scalar(value) => ButtonValue::Scalar(value),
    };

    // crate::send_buttons(vec![ButtonEntry { path_id, value }]);
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.send_buttons(vec![ButtonEntry { path_id, value }]);
    }
}

/// The view poses need to be in local space, as if the head is at the origin.
/// view_params: array of 2
#[unsafe(no_mangle)]
pub extern "C" fn alvr_send_view_params(view_params: *const AlvrViewParams) {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.send_view_params(unsafe {
            [
                alvr_common::from_capi_view_params(&(*view_params)),
                alvr_common::from_capi_view_params(&(*view_params.offset(1))),
            ]
        });
    }
}

/// hand_skeleton:
/// * outer ptr: array of 2 (can be null);
/// * inner ptr: array of 26 (can be null if hand is absent)
///
/// combined_eye_gaze: can be null if eye gaze is absent
#[unsafe(no_mangle)]
pub extern "C" fn alvr_send_tracking(
    poll_timestamp_ns: u64,
    device_motions: *const AlvrDeviceMotion,
    device_motions_count: u64,
    hand_skeletons: *const *const AlvrPose,
    combined_eye_gaze: *const AlvrQuat,
) {
    let mut raw_motions = vec![AlvrDeviceMotion::default(); device_motions_count as _];
    unsafe {
        ptr::copy_nonoverlapping(
            device_motions,
            raw_motions.as_mut_ptr(),
            device_motions_count as usize,
        );
    }

    let device_motions = raw_motions
        .into_iter()
        .map(|motion| {
            (
                motion.device_id,
                DeviceMotion {
                    pose: alvr_common::from_capi_pose(&motion.pose),
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
                (!hand_skeleton.is_null()).then(|| {
                    let hand_skeleton = unsafe { slice::from_raw_parts(hand_skeleton, 26) };

                    let mut array = [Pose::IDENTITY; 26];

                    for (pose, capi_pose) in array.iter_mut().zip(hand_skeleton.iter()) {
                        *pose = Pose {
                            orientation: alvr_common::from_capi_quat(&capi_pose.orientation),
                            position: Vec3::from_slice(&capi_pose.position),
                        };
                    }

                    array
                })
            })
            .collect::<Vec<_>>();

        [hand_skeletons[0], hand_skeletons[1]]
    } else {
        [None, None]
    };

    let eyes_combined = if !combined_eye_gaze.is_null() {
        Some(alvr_common::from_capi_quat(unsafe { &*combined_eye_gaze }))
    } else {
        None
    };

    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.send_tracking(TrackingData {
            poll_timestamp: Duration::from_nanos(poll_timestamp_ns),
            device_motions,
            hand_skeletons,
            face: FaceData {
                eyes_combined,
                ..Default::default()
            },
            body: None,
        });
    }
}

/// Safety: `context` must be thread safe and valid until the StreamingStopped event.
#[unsafe(no_mangle)]
pub extern "C" fn alvr_set_decoder_input_callback(
    callback_context: *mut c_void,
    callback: extern "C" fn(AlvrVideoFrameData) -> bool,
) {
    struct CallbackContext(*mut c_void);
    unsafe impl Send for CallbackContext {}

    let callback_context = CallbackContext(callback_context);

    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.set_decoder_input_callback(Box::new(move |timestamp, buffer| {
            // Make sure to capture the struct itself instead of just the pointer to make the
            // borrow checker happy
            let callback_context = &callback_context;

            callback(AlvrVideoFrameData {
                callback_context: callback_context.0,
                timestamp_ns: timestamp.as_nanos() as u64,
                buffer_ptr: buffer.as_ptr(),
                buffer_size: buffer.len() as u64,
            })
        }));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_report_frame_decoded(target_timestamp_ns: u64) {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.report_frame_decoded(Duration::from_nanos(target_timestamp_ns));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_report_fatal_decoder_error(message: *const c_char) {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.report_fatal_decoder_error(unsafe { CStr::from_ptr(message).to_str().unwrap() });
    }
}

/// out_view_params must be a vector of 2 elements
/// out_view_params is populated only if the core context is valid
#[unsafe(no_mangle)]
pub extern "C" fn alvr_report_compositor_start(
    target_timestamp_ns: u64,
    out_view_params: *mut AlvrViewParams,
) {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        let view_params =
            context.report_compositor_start(Duration::from_nanos(target_timestamp_ns));

        unsafe {
            *out_view_params = alvr_common::to_capi_view_params(&view_params[0]);
            *out_view_params.offset(1) = alvr_common::to_capi_view_params(&view_params[1]);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_report_submit(target_timestamp_ns: u64, vsync_queue_ns: u64) {
    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.report_submit(
            Duration::from_nanos(target_timestamp_ns),
            Duration::from_nanos(vsync_queue_ns),
        );
    }
}

// OpenGL-related interface

thread_local! {
    static GRAPHICS_CONTEXT: RefCell<Option<Rc<GraphicsContext>>> = const { RefCell::new(None) };
    static LOBBY_RENDERER: RefCell<Option<LobbyRenderer>> = const { RefCell::new(None) };
    static STREAM_RENDERER: RefCell<Option<StreamRenderer>> = const { RefCell::new(None) };
}

#[repr(C)]
pub struct AlvrLobbyViewParams {
    swapchain_index: u32,
    view_params: AlvrViewParams,
}

#[repr(C)]
pub struct AlvrStreamViewParams {
    swapchain_index: u32,
    reprojection_rotation: AlvrQuat,
    fov: AlvrFov,
}

#[repr(C)]
pub struct AlvrStreamConfig {
    view_resolution_width: u32,
    view_resolution_height: u32,
    swapchain_textures: *mut *const u32,
    swapchain_length: u32,
    enable_foveation: bool,
    foveation_center_size_x: f32,
    foveation_center_size_y: f32,
    foveation_center_shift_x: f32,
    foveation_center_shift_y: f32,
    foveation_edge_ratio_x: f32,
    foveation_edge_ratio_y: f32,
    enable_upscaling: bool,
    upscaling_edge_direction: bool,
    upscaling_edge_threshold: f32,
    upscaling_edge_sharpness: f32,
    upscale_factor: f32,
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_initialize_opengl() {
    GRAPHICS_CONTEXT.set(Some(Rc::new(GraphicsContext::new_gl())));
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_destroy_opengl() {
    GRAPHICS_CONTEXT.set(None);
}

fn convert_swapchain_array(
    swapchain_textures: *mut *const u32,
    swapchain_length: u32,
) -> [Vec<u32>; 2] {
    let swapchain_length = swapchain_length as usize;
    let mut left_swapchain = vec![0; swapchain_length];
    unsafe {
        ptr::copy_nonoverlapping(
            *swapchain_textures,
            left_swapchain.as_mut_ptr(),
            swapchain_length,
        )
    };
    let mut right_swapchain = vec![0; swapchain_length];
    unsafe {
        ptr::copy_nonoverlapping(
            *swapchain_textures.offset(1),
            right_swapchain.as_mut_ptr(),
            swapchain_length,
        )
    };

    [left_swapchain, right_swapchain]
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_resume_opengl(
    preferred_view_width: u32,
    preferred_view_height: u32,
    swapchain_textures: *mut *const u32,
    swapchain_length: u32,
) {
    LOBBY_RENDERER.set(Some(LobbyRenderer::new(
        GRAPHICS_CONTEXT.with_borrow(|c| c.as_ref().unwrap().clone()),
        UVec2::new(preferred_view_width, preferred_view_height),
        convert_swapchain_array(swapchain_textures, swapchain_length),
        "",
    )));
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_pause_opengl() {
    STREAM_RENDERER.set(None);
    LOBBY_RENDERER.set(None)
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_update_hud_message_opengl(message: *const c_char) {
    LOBBY_RENDERER.with_borrow(|renderer| {
        if let Some(renderer) = renderer {
            renderer.update_hud_message(unsafe { CStr::from_ptr(message) }.to_str().unwrap());
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_start_stream_opengl(config: AlvrStreamConfig) {
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
    let upscaling = config.enable_upscaling.then_some(UpscalingConfig {
        edge_direction: config.upscaling_edge_direction,
        edge_sharpness: config.upscaling_edge_sharpness,
        edge_threshold: config.upscaling_edge_threshold,
        upscale_factor: config.upscale_factor,
    });

    STREAM_RENDERER.set(Some(StreamRenderer::new(
        GRAPHICS_CONTEXT.with_borrow(|c| c.as_ref().unwrap().clone()),
        view_resolution,
        alvr_graphics::compute_target_view_resolution(view_resolution, &upscaling),
        swapchain_textures,
        SDR_FORMAT_GL,
        foveated_encoding,
        true,
        false, // TODO: limited range fix config
        1.0,   // TODO: encoding gamma config
        upscaling,
    )));
}

// todo: support hands
#[unsafe(no_mangle)]
pub extern "C" fn alvr_render_lobby_opengl(
    view_inputs: *const AlvrLobbyViewParams,
    render_background: bool,
) {
    let view_inputs = unsafe {
        [
            LobbyViewParams {
                swapchain_index: (*view_inputs).swapchain_index,
                view_params: alvr_common::from_capi_view_params(&(*view_inputs).view_params),
            },
            LobbyViewParams {
                swapchain_index: (*view_inputs.offset(1)).swapchain_index,
                view_params: alvr_common::from_capi_view_params(
                    &(*view_inputs.offset(1)).view_params,
                ),
            },
        ]
    };

    LOBBY_RENDERER.with_borrow(|renderer| {
        if let Some(renderer) = renderer {
            renderer.render(
                view_inputs,
                [(None, None), (None, None)],
                None,
                None,
                render_background,
                false,
            );
        }
    });
}

/// view_params: array of 2
#[unsafe(no_mangle)]
pub extern "C" fn alvr_render_stream_opengl(
    hardware_buffer: *mut c_void,
    view_params: *const AlvrStreamViewParams,
) {
    STREAM_RENDERER.with_borrow(|renderer| {
        if let Some(renderer) = renderer {
            let left_params = unsafe { &*view_params };
            let right_params = unsafe { &*view_params.offset(1) };
            renderer.render(
                hardware_buffer,
                [
                    StreamViewParams {
                        swapchain_index: left_params.swapchain_index,
                        input_view_params: ViewParams {
                            pose: Pose::IDENTITY,
                            fov: alvr_common::from_capi_fov(&left_params.fov),
                        },
                        output_view_params: ViewParams {
                            pose: Pose {
                                orientation: alvr_common::from_capi_quat(
                                    &left_params.reprojection_rotation,
                                ),
                                position: Vec3::ZERO,
                            },
                            fov: alvr_common::from_capi_fov(&left_params.fov),
                        },
                    },
                    StreamViewParams {
                        swapchain_index: right_params.swapchain_index,
                        input_view_params: ViewParams {
                            pose: Pose::IDENTITY,
                            fov: alvr_common::from_capi_fov(&right_params.fov),
                        },
                        output_view_params: ViewParams {
                            pose: Pose {
                                orientation: alvr_common::from_capi_quat(
                                    &right_params.reprojection_rotation,
                                ),
                                position: Vec3::ZERO,
                            },
                            fov: alvr_common::from_capi_fov(&right_params.fov),
                        },
                    },
                ],
                None,
            );
        }
    });
}

// Decoder-related interface

static DECODER_SOURCE: Mutex<Option<VideoDecoderSource>> = Mutex::new(None);

#[repr(u8)]
pub enum AlvrMediacodecPropType {
    Float,
    Int32,
    Int64,
    String,
}

#[repr(C)]
pub union AlvrMediacodecPropValue {
    float_: f32,
    int32: i32,
    int64: i64,
    string: *const c_char,
}

#[repr(C)]
pub struct AlvrMediacodecOption {
    key: *const c_char,
    ty: AlvrMediacodecPropType,
    value: AlvrMediacodecPropValue,
}

#[repr(C)]
pub struct AlvrDecoderConfig {
    codec: AlvrCodecType,
    force_software_decoder: bool,
    max_buffering_frames: f32,
    buffering_history_weight: f32,
    options: *const AlvrMediacodecOption,
    options_count: u64,
    config_buffer: *const u8,
    config_buffer_size: u64,
}

/// alvr_initialize() must be called before alvr_create_decoder
#[unsafe(no_mangle)]
pub extern "C" fn alvr_create_decoder(config: AlvrDecoderConfig) {
    let config = VideoDecoderConfig {
        codec: match config.codec {
            AlvrCodecType::H264 => CodecType::H264,
            AlvrCodecType::Hevc => CodecType::Hevc,
            AlvrCodecType::AV1 => CodecType::AV1,
        },
        force_software_decoder: config.force_software_decoder,
        max_buffering_frames: config.max_buffering_frames,
        buffering_history_weight: config.buffering_history_weight,
        options: if !config.options.is_null() {
            let options =
                unsafe { slice::from_raw_parts(config.options, config.options_count as usize) };
            options
                .iter()
                .map(|option| unsafe {
                    let key = CStr::from_ptr(option.key).to_str().unwrap();
                    let prop = match option.ty {
                        AlvrMediacodecPropType::Float => MediacodecProperty {
                            ty: MediacodecPropType::Float,
                            value: option.value.float_.to_string(),
                        },
                        AlvrMediacodecPropType::Int32 => MediacodecProperty {
                            ty: MediacodecPropType::Int32,
                            value: option.value.int32.to_string(),
                        },
                        AlvrMediacodecPropType::Int64 => MediacodecProperty {
                            ty: MediacodecPropType::Int64,
                            value: option.value.int64.to_string(),
                        },
                        AlvrMediacodecPropType::String => MediacodecProperty {
                            ty: MediacodecPropType::String,
                            value: CStr::from_ptr(option.value.string)
                                .to_str()
                                .unwrap()
                                .to_owned(),
                        },
                    };

                    (key.to_owned(), prop)
                })
                .collect()
        } else {
            vec![]
        },
        config_buffer: unsafe {
            slice::from_raw_parts(config.config_buffer, config.config_buffer_size as usize).to_vec()
        },
    };

    let (mut sink, source) =
        video_decoder::create_decoder(config, |maybe_timestamp: Result<Duration>| {
            if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
                match maybe_timestamp {
                    Ok(timestamp) => context.report_frame_decoded(timestamp),
                    Err(e) => context.report_fatal_decoder_error(&e.to_string()),
                }
            }
        });

    *DECODER_SOURCE.lock() = Some(source);

    if let Some(context) = &*CLIENT_CORE_CONTEXT.lock() {
        context.set_decoder_input_callback(Box::new(move |timestamp, buffer| {
            sink.push_nal(timestamp, buffer)
        }));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_destroy_decoder() {
    *DECODER_SOURCE.lock() = None;
}

// Returns true if the timestamp and buffer has been written to
#[unsafe(no_mangle)]
pub extern "C" fn alvr_get_frame(
    out_timestamp_ns: *mut u64,
    out_buffer_ptr: *mut *mut c_void,
) -> bool {
    if let Some(source) = &mut *DECODER_SOURCE.lock()
        && let Some((timestamp, buffer_ptr)) = source.get_frame()
    {
        unsafe {
            *out_timestamp_ns = timestamp.as_nanos() as u64;
            *out_buffer_ptr = buffer_ptr;
        }

        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_rotation_delta(source: AlvrQuat, destination: AlvrQuat) -> AlvrQuat {
    alvr_common::to_capi_quat(
        &(alvr_common::from_capi_quat(&source).inverse()
            * alvr_common::from_capi_quat(&destination)),
    )
}
