#![allow(non_upper_case_globals, non_snake_case, clippy::missing_safety_doc)]

mod audio;
mod connection;
mod logging_backend;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{data::*, logging::show_err, *};
use jni::{objects::*, *};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::{ptr, slice, sync::Arc};
use tokio::{runtime::Runtime, sync::mpsc, sync::Notify};

lazy_static! {
    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
    static ref IDR_REQUEST_NOTIFIER: Notify = Notify::new();
    static ref MAYBE_LEGACY_SENDER: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>> =Mutex::new(None);
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
    show_err(|| -> StrResult {
        let identity = create_identity(None)?;

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
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onCreateNative(
    env: JNIEnv,
    activity: JObject,
    asset_manager: JObject,
    jout_result: JObject,
) {
    extern "C" fn legacy_send(buffer_ptr: *const u8, len: u32) {
        if let Some(sender) = &*MAYBE_LEGACY_SENDER.lock() {
            let mut vec_buffer = vec![0; len as _];

            // use copy_nonoverlapping (aka memcpy) to avoid freeing memory allocated by C++
            unsafe {
                ptr::copy_nonoverlapping(buffer_ptr, vec_buffer.as_mut_ptr(), len as _);
            }

            sender.send(vec_buffer).ok();
        }
    }

    legacySend = Some(legacy_send);

    show_err(|| -> StrResult {
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
    show_err(|| -> StrResult {
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
            reserved: "".into(),
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

        *MAYBE_RUNTIME.lock() = Some(runtime);

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
    drop(MAYBE_RUNTIME.lock().take());

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
) {
    onBatteryChangedNative(battery)
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
