#![allow(non_upper_case_globals, non_snake_case)]

mod connection;
mod logging_backend;
mod statistics_manager;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::slice;

use alvr_common::{data::*, logging::*, *};
use jni::{objects::*, *};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use statistics_manager::StatisticsManager;
use tokio::{runtime::Runtime, sync::broadcast};

lazy_static! {
    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
    static ref MAYBE_ON_STREAM_STOP_NOTIFIER: Mutex<Option<broadcast::Sender<()>>> =
        Mutex::new(None);
    static ref MAYBE_ON_PAUSE_NOTIFIER: Mutex<Option<broadcast::Sender<()>>> = Mutex::new(None);
    static ref STATISTICS: Mutex<StatisticsManager> = Mutex::new(StatisticsManager::new());
    static ref ON_STREAM_START_PARAMS_TEMP: Mutex<Option<OnStreamStartParams>> = Mutex::new(None);
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
    show_err(|| -> StrResult {
        let identity = create_identity(None)?;

        let jhostname = trace_err!(env.new_string(identity.hostname))?.into();
        trace_err!(env.set_field(jidentity, "hostname", "Ljava/lang/String;", jhostname))?;

        let jcert_pem = trace_err!(env.new_string(identity.certificate_pem))?.into();
        trace_err!(env.set_field(jidentity, "certificatePEM", "Ljava/lang/String;", jcert_pem))?;

        let jkey_pem = trace_err!(env.new_string(identity.key_pem))?.into();
        trace_err!(env.set_field(jidentity, "privateKey", "Ljava/lang/String;", jkey_pem))
    }())
    .ok();
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onCreateNative(
    env: JNIEnv,
    jactivity: JClass,
    jasset_manager: JObject,
    jout_params: JObject,
) {
    show_err(|| -> StrResult {
        let result: OnCreateResult = unsafe {
            onCreate(
                env.get_native_interface() as _,
                **jactivity as _,
                *jasset_manager as _,
            )
        };

        trace_err!(env.set_field(
            jout_params,
            "streamSurfaceHandle",
            "I",
            result.surfaceTextureHandle.into()
        ))?;
        trace_err!(env.set_field(
            jout_params,
            "webviewSurfaceHandle",
            "I",
            result.webViewSurfaceHandle.into()
        ))
    }())
    .ok();
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onResumeNative(
    env: JNIEnv,
    jactivity: JClass,
    jhostname: JString,
    jcertificate_pem: JString,
    jprivate_key: JString,
    jscreen_surface: JObject,
) -> f32 {
    show_err(|| -> StrResult<f32> {
        let java_vm = trace_err!(env.get_java_vm())?;
        let activity_ref = trace_err!(env.new_global_ref(jactivity))?;

        let result = unsafe { onResume(env.get_native_interface() as _, *jscreen_surface as _) };

        let device_name = if result.deviceType == DeviceType_OCULUS_QUEST {
            "Oculus Quest"
        } else if result.deviceType == DeviceType_OCULUS_QUEST_2 {
            "Oculus Quest 2"
        } else {
            "Unknown device"
        };

        let available_refresh_rates =
            unsafe { slice::from_raw_parts(result.refreshRates, result.refreshRatesCount as _) }
                .to_vec();

        let headset_info = HeadsetInfoPacket {
            device_name: device_name.into(),
            recommended_eye_resolution: (
                result.recommendedEyeWidth as _,
                result.recommendedEyeHeight as _,
            ),
            recommended_left_eye_fov: Fov {
                left: result.leftEyeFov.left,
                right: result.leftEyeFov.right,
                top: result.leftEyeFov.top,
                bottom: result.leftEyeFov.bottom,
            },
            available_refresh_rates,
            reserved: serde_json::json!({}),
        };

        let private_identity = PrivateIdentity {
            hostname: trace_err!(env.get_string(jhostname))?.into(),
            certificate_pem: trace_err!(env.get_string(jcertificate_pem))?.into(),
            key_pem: trace_err!(env.get_string(jprivate_key))?.into(),
        };

        let runtime = trace_err!(Runtime::new())?;
        let (on_pause_notifier, mut on_pause_receiver) = broadcast::channel(1);
        let (on_stream_stop_notifier, _) = broadcast::channel(1);

        runtime.spawn({
            let on_stream_stop_notifier = on_stream_stop_notifier.clone();
            async move {
                let connection_loop = connection::connection_loop(
                    headset_info,
                    private_identity,
                    on_stream_stop_notifier,
                    java_vm,
                    activity_ref,
                );

                tokio::select! {
                    _ = connection_loop => (),
                    _ = on_pause_receiver.recv() => ()
                };
            }
        });

        *MAYBE_ON_STREAM_STOP_NOTIFIER.lock() = Some(on_stream_stop_notifier);
        *MAYBE_ON_PAUSE_NOTIFIER.lock() = Some(on_pause_notifier);
        *MAYBE_RUNTIME.lock() = Some(runtime);

        Ok(result.defaultRefreshRate)
    }())
    .unwrap()
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onStreamStartNative(
    _: JNIEnv,
    _: JClass,
) {
    if let Some(params) = ON_STREAM_START_PARAMS_TEMP.lock().take() {
        unsafe { onStreamStart(params) };
    }
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_renderNative(
    _: JNIEnv,
    _: JClass,
    streaming: u8, // jboolean
    frame_idx: i64,
) {
    unsafe { render(streaming == 1, frame_idx) };
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onFrameInputNative(
    _: JNIEnv,
    _: JClass,
    frame_idx: i64,
) {
    STATISTICS.lock().report_frame_to_be_decoded(frame_idx);
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onFrameOutputNative(
    _: JNIEnv,
    _: JClass,
    frame_idx: i64,
) {
    STATISTICS.lock().report_decoded_frame(frame_idx);
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onStreamStopNative(
    _: JNIEnv,
    _: JClass,
) {
    if let Some(notifier) = MAYBE_ON_STREAM_STOP_NOTIFIER.lock().take() {
        notifier.send(()).ok();
    }
    unsafe { onStreamStop() };
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onPauseNative(_: JNIEnv, _: JClass) {
    if let Some(notifier) = MAYBE_ON_PAUSE_NOTIFIER.lock().take() {
        notifier.send(()).ok();
    }

    unsafe { onPause() };

    // shutdown and wait for tasks to finish
    drop(MAYBE_RUNTIME.lock().take());
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onDestroyNative(
    env: JNIEnv,
    _: JClass,
) {
    unsafe { onDestroy(env.get_native_interface() as _) };
}
