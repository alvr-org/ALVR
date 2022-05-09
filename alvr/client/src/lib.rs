#![allow(non_upper_case_globals, non_snake_case, clippy::missing_safety_doc)]

mod connection;
mod connection_utils;
mod decoder;
mod logging_backend;
mod statistics;
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
    BatteryPacket, ClientStatistics, HeadsetInfoPacket, Input, LegacyController, LegacyInput,
    MotionData, ViewsConfig,
};
use decoder::{DECODER_REF, STREAM_TEAXTURE_HANDLE};
use jni::{
    objects::{GlobalRef, JObject, ReleaseMode},
    sys::jboolean,
    JNIEnv, JavaVM,
};
use statistics::StatisticsManager;
use std::{collections::HashMap, ffi::CStr, os::raw::c_char, ptr, slice, time::Duration};
use tokio::{runtime::Runtime, sync::mpsc, sync::Notify};

// This is the actual storage for the context pointer set in ndk-context. usually stored in
// ndk-glue instead
static GLOBAL_CONTEXT: OnceCell<GlobalRef> = OnceCell::new();

static STATISTICS_MANAGER: Lazy<Mutex<Option<StatisticsManager>>> = Lazy::new(|| Mutex::new(None));

static RUNTIME: Lazy<Mutex<Option<Runtime>>> = Lazy::new(|| Mutex::new(None));
static INPUT_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<Input>>>> =
    Lazy::new(|| Mutex::new(None));
static STATISTICS_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ClientStatistics>>>> =
    Lazy::new(|| Mutex::new(None));
static VIDEO_ERROR_REPORT_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<()>>>> =
    Lazy::new(|| Mutex::new(None));
static VIEWS_CONFIG_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ViewsConfig>>>> =
    Lazy::new(|| Mutex::new(None));
static BATTERY_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<BatteryPacket>>>> =
    Lazy::new(|| Mutex::new(None));
static ON_PAUSE_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_initializeNative(
    env: JNIEnv,
    context: JObject,
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
        let android_context = ndk_context::android_context();

        let vm = unsafe { jni::JavaVM::from_raw(android_context.vm().cast()).unwrap() };
        let env = vm.attach_current_thread().unwrap();

        let asset_manager = env
            .call_method(
                android_context.context().cast(),
                "getAssets",
                "()Landroid/content/res/AssetManager;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap();

        let result = unsafe {
            onCreate(
                env.get_native_interface() as _,
                *context as _,
                *asset_manager as _,
            )
        };

        *STREAM_TEAXTURE_HANDLE.lock() = result.streamSurfaceHandle;

        Ok(())
    }());
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
) {
    let rendered_frame_index = if let Some(decoder) = &*DECODER_REF.lock() {
        let vm = JavaVM::from_raw(ndk_context::android_context().vm().cast()).unwrap();
        let env = vm.get_env().unwrap();

        env.call_method(decoder.as_obj(), "clearAvailable", "()J", &[])
            .unwrap()
            .j()
            .unwrap()
    } else {
        -1
    };

    if rendered_frame_index != -1 {
        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            stats.report_frame_decoded(Duration::from_nanos(rendered_frame_index as _));
        }
        renderNative(rendered_frame_index);
    }
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
    _: JNIEnv,
    _: JObject,
    jscreen_surface: JObject,
    decoder: JObject,
) {
    alvr_common::show_err(|| -> StrResult {
        let vm = JavaVM::from_raw(ndk_context::android_context().vm().cast()).unwrap();
        let env = vm.get_env().unwrap();

        // let decoder_class = env
        //     .find_class("com/polygraphene/alvr/DecoderThread")
        //     .unwrap();
        // let handle = *STREAM_TEAXTURE_HANDLE.lock();
        // let decoder = env
        //     .new_object(decoder_class, "(I)V", &[handle.into()])
        //     .unwrap();
        *DECODER_REF.lock() = Some(env.new_global_ref(decoder).map_err(err!())?);

        let config = storage::load_config();

        let result = onResumeNative(*jscreen_surface as _, config.dark_mode);

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
            let connection_loop = connection::connection_lifecycle_loop(headset_info);

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
    codec: i32,
    real_time: jboolean,
) {
    onStreamStartNative();

    let vm = JavaVM::from_raw(ndk_context::android_context().vm().cast()).unwrap();
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

    extern "C" fn report_submit(target_timestamp_ns: u64, vsync_queue_ns: u64) {
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

    extern "C" fn get_prediction_offset_ns() -> u64 {
        if let Some(stats) = &*STATISTICS_MANAGER.lock() {
            stats.average_total_pipeline_latency().as_nanos() as _
        } else {
            0
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
        reportSubmit = Some(report_submit);
        getPredictionOffsetNs = Some(get_prediction_offset_ns);
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
