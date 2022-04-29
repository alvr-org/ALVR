#![allow(non_upper_case_globals, non_snake_case, clippy::missing_safety_doc)]

mod connection;
mod connection_utils;
mod logging_backend;
mod storage;

#[cfg(target_os = "android")]
mod permission;

#[cfg(target_os = "android")]
mod audio;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{
    glam::{Quat, Vec2, Vec3},
    once_cell::sync::{Lazy, OnceCell},
    parking_lot::Mutex,
    prelude::*,
    ALVR_VERSION, HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_session::Fov;
use alvr_sockets::{
    BatteryPacket, HeadsetInfoPacket, Input, LegacyController, LegacyInput, MotionData,
    TimeSyncPacket, ViewsConfig,
};
use jni::{
    objects::{GlobalRef, JClass, JObject, JString, ReleaseMode},
    sys::jobject,
    JNIEnv, JavaVM,
};
use std::{
    collections::HashMap,
    ffi::{c_void, CStr},
    os::raw::c_char,
    ptr, slice,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{runtime::Runtime, sync::mpsc, sync::Notify};

// This is the actual storage for the context pointer set in ndk-context. usually stored in
// ndk-glue instead
static GLOBAL_CONTEXT: OnceCell<GlobalRef> = OnceCell::new();
static DECODER_REF: Lazy<Mutex<Option<GlobalRef>>> = Lazy::new(|| Mutex::new(None));

static RUNTIME: Lazy<Mutex<Option<Runtime>>> = Lazy::new(|| Mutex::new(None));
static IDR_PARSED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static INPUT_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<Input>>>> =
    Lazy::new(|| Mutex::new(None));
static TIME_SYNC_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<TimeSyncPacket>>>> =
    Lazy::new(|| Mutex::new(None));
static VIDEO_ERROR_REPORT_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<()>>>> =
    Lazy::new(|| Mutex::new(None));
static VIEWS_CONFIG_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ViewsConfig>>>> =
    Lazy::new(|| Mutex::new(None));
static BATTERY_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<BatteryPacket>>>> =
    Lazy::new(|| Mutex::new(None));
static IDR_REQUEST_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);
static ON_PAUSE_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_initializeNative(
    env: JNIEnv,
    context: JObject,
    asset_manager: JObject,
    jout_result: JObject,
) {
    GLOBAL_CONTEXT
        .set(env.new_global_ref(context).unwrap())
        .map_err(|_| ())
        .unwrap();

    alvr_initialize(
        env.get_java_vm().unwrap(),
        GLOBAL_CONTEXT.get().unwrap().as_obj(),
    );

    // todo: manage loading and stream textures on lib side
    alvr_common::show_err(|| -> StrResult {
        let result = unsafe {
            onCreate(
                env.get_native_interface() as _,
                *context as _,
                *asset_manager as _,
            )
        };

        env.set_field(
            jout_result,
            "streamSurfaceHandle",
            "I",
            result.streamSurfaceHandle.into(),
        )
        .map_err(err!())?;
        env.set_field(
            jout_result,
            "loadingSurfaceHandle",
            "I",
            result.loadingSurfaceHandle.into(),
        )
        .map_err(err!())?;

        Ok(())
    }());
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_DecoderThread_DecoderInput(
    _: JNIEnv,
    _: JObject,
    frame_index: i64,
) {
    decoderInput(frame_index);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_DecoderThread_DecoderOutput(
    _: JNIEnv,
    _: JObject,
    frame_index: i64,
) {
    decoderOutput(frame_index);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_DecoderThread_setWaitingNextIDR(
    _: JNIEnv,
    _: JObject,
    waiting: bool,
) {
    IDR_PARSED.store(!waiting, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_destroyNative(
    env: JNIEnv,
    _: JObject,
) {
    destroyNative(env.get_native_interface() as _)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_renderNative(
    _: JNIEnv,
    _: JObject,
    rendered_frame_index: i64,
) {
    renderNative(rendered_frame_index)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_renderLoadingNative(
    _: JNIEnv,
    _: JObject,
) {
    renderLoadingNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onResumeNative(
    env: JNIEnv,
    jactivity: JObject,
    nal_class: JClass,
    jscreen_surface: JObject,
    decoder: JObject,
) {
    alvr_common::show_err(|| -> StrResult {
        let java_vm = env.get_java_vm().map_err(err!())?;
        let activity_ref = env.new_global_ref(jactivity).map_err(err!())?;

        *DECODER_REF.lock() = Some(env.new_global_ref(decoder).map_err(err!())?);

        let config = storage::load_config();

        let result = onResumeNative(*jscreen_surface as _, config.dark_mode);

        let device_name = if result.deviceType == DeviceType_OCULUS_GO {
            "Oculus Go"
        } else if result.deviceType == DeviceType_OCULUS_QUEST {
            "Oculus Quest"
        } else if result.deviceType == DeviceType_OCULUS_QUEST_2 {
            "Oculus Quest 2"
        } else {
            "Unknown device"
        };

        let available_refresh_rates =
            slice::from_raw_parts(result.refreshRates, result.refreshRatesCount as _).to_vec();
        let preferred_refresh_rate = available_refresh_rates.last().cloned().unwrap_or(60_f32);

        let headset_info = HeadsetInfoPacket {
            recommended_eye_width: result.recommendedEyeWidth as _,
            recommended_eye_height: result.recommendedEyeHeight as _,
            available_refresh_rates,
            preferred_refresh_rate,
            reserved: format!("{}", *ALVR_VERSION),
        };

        let runtime = Runtime::new().map_err(err!())?;

        runtime.spawn(async move {
            let connection_loop = connection::connection_lifecycle_loop(
                headset_info,
                device_name,
                &config.hostname,
                Arc::new(java_vm),
                Arc::new(activity_ref),
            );

            tokio::select! {
                _ = connection_loop => (),
                _ = ON_PAUSE_NOTIFIER.notified() => ()
            };
        });

        *RUNTIME.lock() = Some(runtime);

        Ok(())
    }());
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onStreamStartNative(
    _: JNIEnv,
    _: JObject,
) {
    onStreamStartNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onPauseNative(
    env: JNIEnv,
    _: JObject,
) {
    ON_PAUSE_NOTIFIER.notify_waiters();

    // shutdown and wait for tasks to finish
    drop(RUNTIME.lock().take());

    onPauseNative();

    if let Some(decoder) = DECODER_REF.lock().take() {
        env.call_method(decoder.as_obj(), "stopAndWait", "()V", &[])
            .unwrap();
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onBatteryChangedNative(
    _: JNIEnv,
    _: JObject,
    battery: i32,
    plugged: i32,
) {
    onBatteryChangedNative(battery, plugged)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_isConnectedNative(
    _: JNIEnv,
    _: JObject,
) -> u8 {
    isConnectedNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_DecoderThread_requestIDR(
    _: JNIEnv,
    _: JObject,
) {
    IDR_REQUEST_NOTIFIER.notify_waiters();
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_DecoderThread_restartRenderCycle(
    env: JNIEnv,
    _: JObject,
) {
    let context = ndk_context::android_context().context();

    env.call_method(context.cast(), "restartRenderCycle", "()V", &[])
        .unwrap();
}

// Rust Interface:

// Note: Java VM and Android Context must be initialized with ndk-glue
pub fn initialize() {
    logging_backend::init_logging();

    unsafe extern "C" fn path_string_to_hash(path: *const c_char) -> u64 {
        alvr_common::hash_string(CStr::from_ptr(path).to_str().unwrap())
    }

    extern "C" fn input_send(data: TrackingInfo) {
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
                            linear_velocity: from_tracking_vector3(
                                data.controller[0].linearVelocity,
                            ),
                            angular_velocity: from_tracking_vector3(
                                data.controller[0].angularVelocity,
                            ),
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
                            linear_velocity: from_tracking_vector3(
                                data.controller[1].linearVelocity,
                            ),
                            angular_velocity: from_tracking_vector3(
                                data.controller[1].angularVelocity,
                            ),
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
                                data.controller[0].trackpadPosition.x,
                                data.controller[0].trackpadPosition.y,
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
                                data.controller[1].trackpadPosition.x,
                                data.controller[1].trackpadPosition.y,
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

    extern "C" fn time_sync_send(data: TimeSync) {
        if let Some(sender) = &*TIME_SYNC_SENDER.lock() {
            let time_sync = TimeSyncPacket {
                mode: data.mode,
                server_time: data.serverTime,
                client_time: data.clientTime,
                packets_lost_total: data.packetsLostTotal,
                packets_lost_in_second: data.packetsLostInSecond,
                average_send_latency: data.averageSendLatency,
                average_transport_latency: data.averageTransportLatency,
                average_decode_latency: data.averageDecodeLatency,
                idle_time: data.idleTime,
                fec_failure: data.fecFailure,
                fec_failure_in_second: data.fecFailureInSecond,
                fec_failure_total: data.fecFailureTotal,
                fps: data.fps,
                server_total_latency: data.serverTotalLatency,
                tracking_recv_frame_index: data.trackingRecvFrameIndex,
            };

            sender.send(time_sync).ok();
        }
    }

    extern "C" fn video_error_report_send() {
        if let Some(sender) = &*VIDEO_ERROR_REPORT_SENDER.lock() {
            sender.send(()).ok();
        }
    }

    extern "C" fn views_config_send(fov: *mut EyeFov, ipd_m: f32) {
        let fov = unsafe { slice::from_raw_parts(fov, 2) };
        if let Some(sender) = &*VIEWS_CONFIG_SENDER.lock() {
            sender
                .send(ViewsConfig {
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
                })
                .ok();
        }
    }

    extern "C" fn battery_send(device_id: u64, gauge_value: f32, is_plugged: bool) {
        if let Some(sender) = &*BATTERY_SENDER.lock() {
            sender
                .send(BatteryPacket {
                    device_id,
                    gauge_value,
                    is_plugged,
                })
                .ok();
        }
    }

    extern "C" fn push_nal(buffer: *const c_char, length: i32, frame_index: u64) {
        let vm = unsafe { JavaVM::from_raw(ndk_context::android_context().vm().cast()).unwrap() };
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
        pathStringToHash = Some(path_string_to_hash);
        inputSend = Some(input_send);
        timeSyncSend = Some(time_sync_send);
        videoErrorReportSend = Some(video_error_report_send);
        viewsConfigSend = Some(views_config_send);
        batterySend = Some(battery_send);
        pushNal = Some(push_nal);
    }

    // Make sure to reset config in case of version compat mismatch.
    if storage::load_config().protocol_id != alvr_common::protocol_id() {
        // NB: Config::default() sets the current protocol ID
        storage::store_config(&storage::Config::default());
    }

    permission::try_get_microphone_permission();
}

// C interface:

// NB: context must be thread safe.
#[no_mangle]
pub extern "C" fn alvr_initialize(vm: JavaVM, context: JObject) {
    unsafe {
        ndk_context::initialize_android_context(vm.get_java_vm_pointer().cast(), context.cast())
    };

    initialize();
}
