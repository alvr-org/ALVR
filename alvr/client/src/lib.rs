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
    Fov, MotionData, ALVR_VERSION,
};
use alvr_sockets::{
    HandPoseInput, HeadsetInfoPacket, Input, LegacyInput, PrivateIdentity, TimeSyncPacket,
    ViewConfig,
};
use jni::{
    objects::{JClass, JObject, JString},
    JNIEnv,
};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
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
    static ref LEGACY_SENDER: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>> = Mutex::new(None);
    static ref INPUT_SENDER: Mutex<Option<mpsc::UnboundedSender<Input>>> = Mutex::new(None);
    static ref TIME_SYNC_SENDER: Mutex<Option<mpsc::UnboundedSender<TimeSyncPacket>>> =
        Mutex::new(None);
    static ref VIDEO_ERROR_REPORT_SENDER: Mutex<Option<mpsc::UnboundedSender<()>>> =
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
    extern "C" fn input_send(data: TrackingInfo) {
        fn from_tracking_quat(quat: TrackingQuat) -> Quat {
            Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w)
        }

        fn from_tracking_vector3(vec: TrackingVector3) -> Vec3 {
            Vec3::new(vec.x, vec.y, vec.z)
        }

        if let Some(sender) = &*INPUT_SENDER.lock() {
            let head_orientation = from_tracking_quat(data.HeadPose_Pose_Orientation);
            let head_to_right_eye_position = head_orientation * Vec3::X * data.ipd / 2_f32;
            let head_position = from_tracking_vector3(data.HeadPose_Pose_Position);
            let right_eye_position = head_position + head_to_right_eye_position;
            let left_eye_position = head_position - head_to_right_eye_position;

            let input = Input {
                target_timestamp: Duration::from_secs_f64(data.predictedDisplayTime),
                view_configs: vec![
                    ViewConfig {
                        orientation: head_orientation,
                        position: left_eye_position,
                        fov: Fov {
                            left: data.eyeFov[0].left,
                            right: data.eyeFov[0].right,
                            top: data.eyeFov[0].top,
                            bottom: data.eyeFov[0].bottom,
                        },
                    },
                    ViewConfig {
                        orientation: head_orientation,
                        position: right_eye_position,
                        fov: Fov {
                            left: data.eyeFov[1].left,
                            right: data.eyeFov[1].right,
                            top: data.eyeFov[1].top,
                            bottom: data.eyeFov[1].bottom,
                        },
                    },
                ],
                left_pose_input: HandPoseInput {
                    grip_motion: MotionData {
                        orientation: from_tracking_quat(
                            if data.controller[0].flags & (1 << 5) > 0 {
                                data.controller[0].boneRootOrientation
                            } else {
                                data.controller[0].orientation
                            },
                        ),
                        position: from_tracking_vector3(
                            if data.controller[0].flags & (1 << 5) > 0 {
                                data.controller[0].boneRootPosition
                            } else {
                                data.controller[0].position
                            },
                        ),
                        linear_velocity: Some(from_tracking_vector3(
                            data.controller[0].linearVelocity,
                        )),
                        angular_velocity: Some(from_tracking_vector3(
                            data.controller[0].angularVelocity,
                        )),
                    },
                    hand_tracking_input: None,
                },
                right_pose_input: HandPoseInput {
                    grip_motion: MotionData {
                        orientation: from_tracking_quat(
                            if data.controller[1].flags & (1 << 5) > 0 {
                                data.controller[1].boneRootOrientation
                            } else {
                                data.controller[1].orientation
                            },
                        ),
                        position: from_tracking_vector3(
                            if data.controller[1].flags & (1 << 5) > 0 {
                                data.controller[1].boneRootPosition
                            } else {
                                data.controller[1].position
                            },
                        ),
                        linear_velocity: Some(from_tracking_vector3(
                            data.controller[1].linearVelocity,
                        )),
                        angular_velocity: Some(from_tracking_vector3(
                            data.controller[1].angularVelocity,
                        )),
                    },
                    hand_tracking_input: None,
                },
                trackers_pose_input: vec![],
                button_values: HashMap::new(), // unused for now
                legacy: LegacyInput {
                    flags: data.flags,
                    client_time: data.clientTime,
                    frame_index: data.FrameIndex,
                    battery: data.battery,
                    plugged: data.plugged,
                    mounted: data.mounted,
                    controller_flags: [data.controller[0].flags, data.controller[1].flags],
                    buttons: [data.controller[0].buttons, data.controller[1].buttons],
                    trackpad_position: [
                        Vec2::new(
                            data.controller[0].trackpadPosition.x,
                            data.controller[0].trackpadPosition.y,
                        ),
                        Vec2::new(
                            data.controller[1].trackpadPosition.x,
                            data.controller[1].trackpadPosition.y,
                        ),
                    ],
                    trigger_value: [
                        data.controller[0].triggerValue,
                        data.controller[1].triggerValue,
                    ],
                    grip_value: [data.controller[0].gripValue, data.controller[1].gripValue],
                    controller_battery: [
                        data.controller[0].batteryPercentRemaining,
                        data.controller[1].batteryPercentRemaining,
                    ],
                    bone_rotations: [
                        {
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
                        {
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
                    ],
                    bone_positions_base: [
                        {
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
                        {
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
                    ],
                    input_state_status: [
                        data.controller[0].inputStateStatus,
                        data.controller[1].inputStateStatus,
                    ],
                    finger_pinch_strengths: [
                        data.controller[0].fingerPinchStrengths,
                        data.controller[1].fingerPinchStrengths,
                    ],
                    hand_finger_confience: [
                        data.controller[0].handFingerConfidences,
                        data.controller[1].handFingerConfidences,
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

    inputSend = Some(input_send);
    timeSyncSend = Some(time_sync_send);
    videoErrorReportSend = Some(video_error_report_send);

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
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onHapticsFeedbackNative(
    _: JNIEnv,
    _: JObject,
    start_time: i64,
    amplitude: f32,
    duration: f32,
    frequency: f32,
    hand: u8,
) {
    onHapticsFeedbackNative(start_time, amplitude, duration, frequency, hand)
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
