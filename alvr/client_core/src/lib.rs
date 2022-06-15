#![allow(non_upper_case_globals, non_snake_case, clippy::missing_safety_doc)]

mod connection;
mod connection_utils;
mod logging_backend;
mod platform;
mod statistics;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_audio::{AudioDevice, AudioDeviceType};
use alvr_common::{
    glam::{Quat, Vec2, Vec3},
    once_cell::sync::{Lazy, OnceCell},
    parking_lot::Mutex,
    prelude::*,
    ALVR_VERSION, HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_session::{AudioDeviceId, Fov};
use alvr_sockets::{
    BatteryPacket, ClientControlPacket, ClientStatistics, HeadsetInfoPacket, Input,
    LegacyController, LegacyInput, MotionData, ViewsConfig,
};
use jni::{
    objects::{GlobalRef, JObject, ReleaseMode},
    sys::{jboolean, jobject},
    JNIEnv, JavaVM,
};
use statistics::StatisticsManager;
use std::{
    collections::{HashMap, VecDeque},
    ffi::{c_void, CStr},
    intrinsics::copy_nonoverlapping,
    os::raw::{c_char, c_uchar},
    ptr, slice,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::{runtime::Runtime, sync::mpsc, sync::Notify};

// This is the actual storage for the context pointer set in ndk-context. usually stored in
// ndk-glue instead
static GLOBAL_CONTEXT: OnceCell<GlobalRef> = OnceCell::new();
static GLOBAL_ASSET_MANAGER: OnceCell<GlobalRef> = OnceCell::new();

static STATISTICS_MANAGER: Lazy<Mutex<Option<StatisticsManager>>> = Lazy::new(|| Mutex::new(None));

static RUNTIME: Lazy<Mutex<Option<Runtime>>> = Lazy::new(|| Mutex::new(None));
static INPUT_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<Input>>>> =
    Lazy::new(|| Mutex::new(None));
static STATISTICS_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ClientStatistics>>>> =
    Lazy::new(|| Mutex::new(None));
static CONTROL_CHANNEL_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ClientControlPacket>>>> =
    Lazy::new(|| Mutex::new(None));
static ON_PAUSE_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);

static DECODER_REF: Lazy<Mutex<Option<GlobalRef>>> = Lazy::new(|| Mutex::new(None));
static IDR_PARSED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static STREAM_TEAXTURE_HANDLE: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));

static EVENT_BUFFER: Lazy<Mutex<VecDeque<AlvrEvent>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

#[repr(u8)]
pub enum AlvrEvent {
    Haptics {
        device_id: u64,
        duration_s: f32,
        frequency: f32,
        amplitude: f32,
    },
}

#[repr(C)]
pub struct AlvrEyeInput {
    orientation: TrackingQuat,
    position: [f32; 3],
    fov: EyeFov,
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
#[derive(Clone, Copy)]
pub struct TrackingQuat {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrackingVector3 {
    x: f32,
    y: f32,
    z: f32,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct TrackingController {
    enabled: bool,
    isHand: bool,
    buttons: u64,

    trackpadPosition: [f32; 2],
    triggerValue: f32,
    gripValue: f32,

    orientation: TrackingQuat,
    position: TrackingVector3,
    angularVelocity: TrackingVector3,
    linearVelocity: TrackingVector3,

    boneRotations: [TrackingQuat; 19],
    bonePositionsBase: [TrackingVector3; 19],
    boneRootOrientation: TrackingQuat,
    boneRootPosition: TrackingVector3,
    handFingerConfidences: u32,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct TrackingInfo {
    targetTimestampNs: u64,
    HeadPose_Pose_Orientation: TrackingQuat,
    HeadPose_Pose_Position: TrackingVector3,

    mounted: u8,

    controller: [TrackingController; 2],
}

#[no_mangle]
pub unsafe extern "C" fn alvr_path_string_to_hash(path: *const c_char) -> u64 {
    alvr_common::hash_string(CStr::from_ptr(path).to_str().unwrap())
}

// NB: context must be thread safe.
#[no_mangle]
pub extern "C" fn alvr_initialize(java_vm: *mut c_void, context: *mut c_void) {
    unsafe { ndk_context::initialize_android_context(java_vm, context) };
    logging_backend::init_logging();

    extern "C" fn video_error_report_send() {
        if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
            sender.send(ClientControlPacket::VideoErrorReport).ok();
        }
    }

    extern "C" fn push_nal(buffer: *const c_char, length: i32, frame_index: u64) {
        let vm = platform::vm();
        let env = vm.get_env().unwrap();

        let decoder_lock = DECODER_REF.lock();

        let nal = if let Some(decoder) = &*decoder_lock {
            env.call_method(
                decoder,
                "obtainNAL",
                "(I)Lcom/polygraphene/alvr/NAL;",
                &[length.into()],
            )
            .unwrap()
            .l()
            .unwrap()
        } else {
            let nal_class = env.find_class("com/polygraphene/alvr/NAL").unwrap();
            env.new_object(
                nal_class,
                "(I)Lcom/polygraphene/alvr/NAL;",
                &[length.into()],
            )
            .unwrap()
        };

        if nal.is_null() {
            return;
        }

        env.set_field(nal, "length", "I", length.into()).unwrap();
        env.set_field(nal, "frameIndex", "J", (frame_index as i64).into())
            .unwrap();
        {
            let jarray = env.get_field(nal, "buf", "[B").unwrap().l().unwrap();
            let jbuffer = env
                .get_byte_array_elements(*jarray, ReleaseMode::CopyBack)
                .unwrap();
            unsafe { ptr::copy_nonoverlapping(buffer as _, jbuffer.as_ptr(), length as usize) };
            jbuffer.commit().unwrap();
        }

        if let Some(decoder) = &*decoder_lock {
            env.call_method(
                decoder,
                "pushNAL",
                "(Lcom/polygraphene/alvr/NAL;)V",
                &[nal.into()],
            )
            .unwrap();
        }
    }

    unsafe {
        pathStringToHash = Some(alvr_path_string_to_hash);
        videoErrorReportSend = Some(video_error_report_send);
        pushNal = Some(push_nal);
    }

    // Make sure to reset config in case of version compat mismatch.
    if platform::load_config().protocol_id != alvr_common::protocol_id() {
        // NB: Config::default() sets the current protocol ID
        platform::store_config(&platform::Config::default());
    }

    platform::try_get_microphone_permission();

    let vm = platform::vm();
    let env = vm.attach_current_thread().unwrap();

    let asset_manager = env
        .call_method(
            ndk_context::android_context().context().cast(),
            "getAssets",
            "()Landroid/content/res/AssetManager;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let asset_manager = env.new_global_ref(asset_manager).unwrap();

    let result = unsafe {
        initNative(
            ndk_context::android_context().vm(),
            ndk_context::android_context().context(),
            *asset_manager.as_obj() as _,
        )
    };
    *STREAM_TEAXTURE_HANDLE.lock() = result.streamSurfaceHandle;

    GLOBAL_ASSET_MANAGER.set(asset_manager);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_destroy() {
    destroyNative();
}

#[no_mangle]
pub unsafe extern "C" fn alvr_resume(
    decoder_object: *mut c_void,
    recommended_eye_width: u32,
    recommended_eye_height: u32,
    refres_rates: *const f32,
    refresh_rates_count: i32,
    swapchain_textures: *mut *const i32,
    swapchain_length: i32,
) {
    let vm = platform::vm();
    let env = vm.get_env().unwrap();

    *DECODER_REF.lock() = Some(env.new_global_ref(decoder_object.cast()).unwrap());

    let config = platform::load_config();

    prepareLoadingRoom(
        recommended_eye_width as _,
        recommended_eye_height as _,
        config.dark_mode,
        swapchain_textures,
        swapchain_length,
    );

    let available_refresh_rates =
        slice::from_raw_parts(refres_rates, refresh_rates_count as _).to_vec();
    let preferred_refresh_rate = available_refresh_rates.last().cloned().unwrap_or(60_f32);

    let microphone_sample_rate =
        AudioDevice::new(None, AudioDeviceId::Default, AudioDeviceType::Input)
            .unwrap()
            .input_sample_rate()
            .unwrap();

    let headset_info = HeadsetInfoPacket {
        recommended_eye_width: recommended_eye_width as _,
        recommended_eye_height: recommended_eye_height as _,
        available_refresh_rates,
        preferred_refresh_rate,
        microphone_sample_rate,
        reserved: format!("{}", *ALVR_VERSION),
    };

    let runtime = Runtime::new().unwrap();

    runtime.spawn(async move {
        let connection_loop = connection::connection_lifecycle_loop(headset_info);

        tokio::select! {
            _ = connection_loop => (),
            _ = ON_PAUSE_NOTIFIER.notified() => ()
        };
    });

    *RUNTIME.lock() = Some(runtime);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_pause() {
    ON_PAUSE_NOTIFIER.notify_waiters();

    // shutdown and wait for tasks to finish
    drop(RUNTIME.lock().take());

    destroyRenderers();

    if let Some(decoder) = DECODER_REF.lock().take() {
        let vm = platform::vm();
        let env = vm.attach_current_thread().unwrap();

        env.call_method(decoder.as_obj(), "stopAndWait", "()V", &[])
            .unwrap();
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent) -> bool {
    if let Some(event) = EVENT_BUFFER.lock().pop_front() {
        *out_event = event;

        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_start_stream(
    codec: i32,
    real_time: bool,
    swapchain_textures: *mut *const i32,
    swapchain_length: i32,
) {
    streamStartNative(swapchain_textures, swapchain_length);

    let vm = platform::vm();
    let env = vm.get_env().unwrap();

    if let Some(decoder) = &*DECODER_REF.lock() {
        env.call_method(
            decoder.as_obj(),
            "onConnect",
            "(IZ)V",
            &[codec.into(), real_time.into()],
        )
        .unwrap();
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_views_config(fov: *const EyeFov, ipd_m: f32) {
    let fov = unsafe { slice::from_raw_parts(fov, 2) };
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender
            .send(ClientControlPacket::ViewsConfig(ViewsConfig {
                fov: [
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
                ],
                ipd_m,
            }))
            .ok();
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_battery(device_id: u64, gauge_value: f32, is_plugged: bool) {
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender
            .send(ClientControlPacket::Battery(BatteryPacket {
                device_id,
                gauge_value,
                is_plugged,
            }))
            .ok();
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_playspace(width: f32, height: f32) {
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender
            .send(ClientControlPacket::PlayspaceSync(Vec2::new(width, height)))
            .ok();
    }
}

/// Returns frame timestamp in nanoseconds
#[no_mangle]
pub unsafe extern "C" fn alvr_wait_for_frame() -> i64 {
    if let Some(decoder) = &*DECODER_REF.lock() {
        let vm = platform::vm();
        let env = vm.get_env().unwrap();

        env.call_method(decoder.as_obj(), "clearAvailable", "()J", &[])
            .unwrap()
            .j()
            .unwrap()
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_render_lobby(
    eye_inputs: *const AlvrEyeInput,
    swapchain_indices: *const i32,
) {
    let eye_inputs = [
        {
            let o = (*eye_inputs).orientation;
            let f = (*eye_inputs).fov;
            EyeInput {
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
            EyeInput {
                orientation: [o.x, o.y, o.z, o.w],
                position: (*eye_inputs.offset(1)).position,
                fovLeft: f.left,
                fovRight: f.right,
                fovTop: f.top,
                fovBottom: f.bottom,
            }
        },
    ];

    renderLoadingNative(eye_inputs.as_ptr(), swapchain_indices);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_render_stream(timestamp_ns: i64, swapchain_indices: *const i32) {
    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
        stats.report_frame_decoded(Duration::from_nanos(timestamp_ns as _));
    }
    renderNative(swapchain_indices);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_is_streaming() -> bool {
    isConnectedNative()
}

#[no_mangle]
pub extern "C" fn alvr_send_input(data: TrackingInfo) {
    fn from_tracking_quat(quat: TrackingQuat) -> Quat {
        Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w)
    }

    fn from_tracking_vector3(vec: TrackingVector3) -> Vec3 {
        Vec3::new(vec.x, vec.y, vec.z)
    }

    if let Some(sender) = &*INPUT_SENDER.lock() {
        let input = Input {
            target_timestamp: Duration::from_nanos(data.targetTimestampNs),
            device_motions: vec![
                (
                    *HEAD_ID,
                    MotionData {
                        orientation: from_tracking_quat(data.HeadPose_Pose_Orientation),
                        position: from_tracking_vector3(data.HeadPose_Pose_Position),
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ),
                (
                    *LEFT_HAND_ID,
                    MotionData {
                        orientation: from_tracking_quat(if data.controller[0].isHand {
                            data.controller[0].boneRootOrientation
                        } else {
                            data.controller[0].orientation
                        }),
                        position: from_tracking_vector3(if data.controller[0].isHand {
                            data.controller[0].boneRootPosition
                        } else {
                            data.controller[0].position
                        }),
                        linear_velocity: from_tracking_vector3(data.controller[0].linearVelocity),
                        angular_velocity: from_tracking_vector3(data.controller[0].angularVelocity),
                    },
                ),
                (
                    *RIGHT_HAND_ID,
                    MotionData {
                        orientation: from_tracking_quat(if data.controller[1].isHand {
                            data.controller[1].boneRootOrientation
                        } else {
                            data.controller[1].orientation
                        }),
                        position: from_tracking_vector3(if data.controller[1].isHand {
                            data.controller[1].boneRootPosition
                        } else {
                            data.controller[1].position
                        }),
                        linear_velocity: from_tracking_vector3(data.controller[1].linearVelocity),
                        angular_velocity: from_tracking_vector3(data.controller[1].angularVelocity),
                    },
                ),
            ],
            left_hand_tracking: None,
            right_hand_tracking: None,
            button_values: HashMap::new(), // unused for now
            legacy: LegacyInput {
                mounted: data.mounted,
                controllers: [
                    LegacyController {
                        enabled: data.controller[0].enabled,
                        is_hand: data.controller[0].isHand,
                        buttons: data.controller[0].buttons,
                        trackpad_position: Vec2::new(
                            data.controller[0].trackpadPosition[0],
                            data.controller[0].trackpadPosition[1],
                        ),
                        trigger_value: data.controller[0].triggerValue,
                        grip_value: data.controller[0].gripValue,
                        bone_rotations: {
                            let vec = data.controller[0]
                                .boneRotations
                                .iter()
                                .cloned()
                                .map(from_tracking_quat)
                                .collect::<Vec<_>>();

                            let mut array = [Quat::IDENTITY; 19];
                            array.copy_from_slice(&vec);

                            array
                        },
                        bone_positions_base: {
                            let vec = data.controller[0]
                                .bonePositionsBase
                                .iter()
                                .cloned()
                                .map(from_tracking_vector3)
                                .collect::<Vec<_>>();

                            let mut array = [Vec3::ZERO; 19];
                            array.copy_from_slice(&vec);

                            array
                        },
                        hand_finger_confience: data.controller[0].handFingerConfidences,
                    },
                    LegacyController {
                        enabled: data.controller[1].enabled,
                        is_hand: data.controller[1].isHand,
                        buttons: data.controller[1].buttons,
                        trackpad_position: Vec2::new(
                            data.controller[1].trackpadPosition[0],
                            data.controller[1].trackpadPosition[1],
                        ),

                        trigger_value: data.controller[1].triggerValue,

                        grip_value: data.controller[1].gripValue,

                        bone_rotations: {
                            let vec = data.controller[1]
                                .boneRotations
                                .iter()
                                .cloned()
                                .map(from_tracking_quat)
                                .collect::<Vec<_>>();

                            let mut array = [Quat::IDENTITY; 19];
                            array.copy_from_slice(&vec);

                            array
                        },

                        bone_positions_base: {
                            let vec = data.controller[1]
                                .bonePositionsBase
                                .iter()
                                .cloned()
                                .map(from_tracking_vector3)
                                .collect::<Vec<_>>();

                            let mut array = [Vec3::ZERO; 19];
                            array.copy_from_slice(&vec);

                            array
                        },

                        hand_finger_confience: data.controller[1].handFingerConfidences,
                    },
                ],
            },
        };

        sender.send(input).ok();
    }
}

#[no_mangle]
pub extern "C" fn alvr_get_prediction_offset_ns() -> u64 {
    if let Some(stats) = &*STATISTICS_MANAGER.lock() {
        stats.average_total_pipeline_latency().as_nanos() as _
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn alvr_report_submit(target_timestamp_ns: u64, vsync_queue_ns: u64) {
    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
        let timestamp = Duration::from_nanos(target_timestamp_ns);
        stats.report_submit(timestamp, Duration::from_nanos(vsync_queue_ns));

        if let Some(sender) = &*STATISTICS_SENDER.lock() {
            if let Some(stats) = stats.summary(timestamp) {
                sender.send(stats).ok();
            } else {
                error!("Statistics summary not ready!");
            }
        }
    }
}

/// decoder helper
#[no_mangle]
pub extern "C" fn alvr_set_waiting_next_idr(waiting: bool) {
    IDR_PARSED.store(!waiting, Ordering::Relaxed);
}

/// decoder helper
#[no_mangle]
pub extern "C" fn alvr_request_idr() {
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender.send(ClientControlPacket::RequestIdr).ok();
    }
}

/// decoder helper
#[no_mangle]
pub extern "C" fn alvr_restart_rendering_cycle() {
    let vm = platform::vm();
    let env = vm.attach_current_thread().unwrap();

    env.call_method(platform::context(), "restartRenderCycle", "()V", &[])
        .unwrap();
}

/// decoder helper
#[no_mangle]
pub extern "C" fn alvr_get_stream_texture_handle() -> i32 {
    *STREAM_TEAXTURE_HANDLE.lock()
}
