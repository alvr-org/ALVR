#![allow(non_upper_case_globals, non_snake_case, clippy::missing_safety_doc)]

mod connection;
mod connection_utils;
mod logging_backend;

#[cfg(target_os = "android")]
mod audio;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{
    glam::{Quat, Vec2, Vec3},
    lazy_static,
    prelude::*,
    ALVR_VERSION, HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_session::Fov;
use alvr_sockets::{
    BatteryPacket, HeadsetInfoPacket, Input, LegacyController, LegacyInput, MotionData,
    PrivateIdentity, TimeSyncPacket, ViewsConfig,
};
use jni::{
    objects::{JClass, JObject, JString},
    JNIEnv,
};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    ffi::CStr,
    os::raw::c_char,
    ptr, slice,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{runtime::Runtime, sync::mpsc, sync::Notify};

lazy_static! {
    static ref RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
    static ref IDR_PARSED: AtomicBool = AtomicBool::new(false);
    static ref INPUT_SENDER: Mutex<Option<mpsc::UnboundedSender<Input>>> = Mutex::new(None);
    static ref TIME_SYNC_SENDER: Mutex<Option<mpsc::UnboundedSender<TimeSyncPacket>>> =
        Mutex::new(None);
    static ref VIDEO_ERROR_REPORT_SENDER: Mutex<Option<mpsc::UnboundedSender<()>>> =
        Mutex::new(None);
    static ref VIEWS_CONFIG_SENDER: Mutex<Option<mpsc::UnboundedSender<ViewsConfig>>> =
        Mutex::new(None);
    static ref BATTERY_SENDER: Mutex<Option<mpsc::UnboundedSender<BatteryPacket>>> =
        Mutex::new(None);
    static ref IDR_REQUEST_NOTIFIER: Notify = Notify::new();
    static ref ON_PAUSE_NOTIFIER: Notify = Notify::new();
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_initNativeLogging(
    _: JNIEnv,
    _: JClass,
) {
    logging_backend::init_logging();
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_createIdentity(
    env: JNIEnv,
    _: JClass,
    jidentity: JObject,
) {
    alvr_common::show_err(|| -> StrResult {
        let identity = alvr_sockets::create_identity(None)?;

        let jhostname = trace_err!(env.new_string(identity.hostname))?.into();
        trace_err!(env.set_field(jidentity, "hostname", "Ljava/lang/String;", jhostname))?;

        let jcert_pem = trace_err!(env.new_string(identity.certificate_pem))?.into();
        trace_err!(env.set_field(jidentity, "certificatePEM", "Ljava/lang/String;", jcert_pem))?;

        let jkey_pem = trace_err!(env.new_string(identity.key_pem))?.into();
        trace_err!(env.set_field(jidentity, "privateKey", "Ljava/lang/String;", jkey_pem))
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
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onCreateNative(
    env: JNIEnv,
    activity: JObject,
    asset_manager: JObject,
    jout_result: JObject,
) {
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
                            linear_velocity: None,
                            angular_velocity: None,
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
                            linear_velocity: Some(from_tracking_vector3(
                                data.controller[0].linearVelocity,
                            )),
                            angular_velocity: Some(from_tracking_vector3(
                                data.controller[0].angularVelocity,
                            )),
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
                            linear_velocity: Some(from_tracking_vector3(
                                data.controller[1].linearVelocity,
                            )),
                            angular_velocity: Some(from_tracking_vector3(
                                data.controller[1].angularVelocity,
                            )),
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

    pathStringToHash = Some(path_string_to_hash);
    inputSend = Some(input_send);
    timeSyncSend = Some(time_sync_send);
    videoErrorReportSend = Some(video_error_report_send);
    viewsConfigSend = Some(views_config_send);
    batterySend = Some(battery_send);

    alvr_common::show_err(|| -> StrResult {
        let result = onCreate(
            env.get_native_interface() as _,
            *activity as _,
            *asset_manager as _,
        );

        trace_err!(env.set_field(
            jout_result,
            "streamSurfaceHandle",
            "I",
            result.streamSurfaceHandle.into()
        ))?;
        trace_err!(env.set_field(
            jout_result,
            "loadingSurfaceHandle",
            "I",
            result.loadingSurfaceHandle.into()
        ))?;

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
    jhostname: JString,
    jcertificate_pem: JString,
    jprivate_key: JString,
    jscreen_surface: JObject,
    dark_mode: u8,
) {
    alvr_common::show_err(|| -> StrResult {
        let java_vm = trace_err!(env.get_java_vm())?;
        let activity_ref = trace_err!(env.new_global_ref(jactivity))?;
        let nal_class_ref = trace_err!(env.new_global_ref(nal_class))?;

        let result = onResumeNative(*jscreen_surface as _, dark_mode == 1);

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

        let private_identity = PrivateIdentity {
            hostname: trace_err!(env.get_string(jhostname))?.into(),
            certificate_pem: trace_err!(env.get_string(jcertificate_pem))?.into(),
            key_pem: trace_err!(env.get_string(jprivate_key))?.into(),
        };

        let runtime = trace_err!(Runtime::new())?;

        runtime.spawn(async move {
            let connection_loop = connection::connection_lifecycle_loop(
                headset_info,
                device_name,
                private_identity,
                Arc::new(java_vm),
                Arc::new(activity_ref),
                Arc::new(nal_class_ref),
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
    _: JNIEnv,
    _: JObject,
) {
    ON_PAUSE_NOTIFIER.notify_waiters();

    // shutdown and wait for tasks to finish
    drop(RUNTIME.lock().take());

    onPauseNative();
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
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_requestIDR(
    _: JNIEnv,
    _: JObject,
) {
    IDR_REQUEST_NOTIFIER.notify_waiters();
}
