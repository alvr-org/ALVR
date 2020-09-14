mod connection;
mod logging_backend;
mod statistics_manager;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{data::*, logging::*, sockets::*, *};
use jni::{objects::*, *};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use statistics_manager::StatisticsManager;
use tokio::{runtime::Runtime, sync::broadcast};

lazy_static! {
    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
    static ref MAYBE_ON_PAUSE_NOTIFIER: Mutex<Option<broadcast::Sender<()>>> = Mutex::new(None);
    static ref MAYBE_INPUT_SENDER: Mutex<Option<StreamSender<VideoPacket>>> = Mutex::new(None);
    static ref MAYBE_MICROPHONE_SENDER: Mutex<Option<StreamSender<AudioPacket>>> = Mutex::new(None);
    static ref STATISTICS: Mutex<StatisticsManager> = Mutex::new(StatisticsManager::new());
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
    jparams: JObject,
) {
    show_err(|| -> StrResult {
        let jasset_manager = trace_err!(env.get_field(
            jparams,
            "assetManager",
            "Landroid/content/res/AssetManager;"
        ))?;
        let jasset_manager = trace_err!(jasset_manager.l())?;

        let result: OnCreateResult = unsafe {
            onCreate(
                env.get_native_interface() as _,
                **jactivity as _,
                *jasset_manager as _,
            )
        };

        trace_err!(env.set_field(
            jparams,
            "streamSurfaceHandle",
            "I",
            result.surfaceTextureHandle.into()
        ))?;
        trace_err!(env.set_field(
            jparams,
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
    _: JClass,
    jhostname: JString,
    jcertificate_pem: JString,
    jprivate_key: JString,
    jscreen_surface: JObject,
) -> f32 {
    show_err(|| -> StrResult<f32> {
        let result = unsafe { onResume(env.get_native_interface() as _, *jscreen_surface as _) };

        let device_name = if result.deviceType == DeviceType_OCULUS_QUEST {
            "Oculus Quest"
        } else if result.deviceType == DeviceType_OCULUS_QUEST_2 {
            "Oculus Quest 2"
        } else {
            "Unknown device"
        };

        // fov cannot be retrieved from oculus sdk at this point, use apriori values.
        let recommended_left_eye_fov = if result.deviceType == DeviceType_OCULUS_QUEST {
            Fov {
                left: 52.,
                right: 42.,
                top: 53.,
                bottom: 47.,
            }
        } else if result.deviceType == DeviceType_OCULUS_QUEST_2 {
            // todo: use correct values
            Fov {
                left: 50.,
                right: 50.,
                top: 50.,
                bottom: 50.,
            }
        } else {
            Fov {
                left: 45.,
                right: 45.,
                top: 45.,
                bottom: 45.,
            }
        };

        let available_refresh_rates = result
            .refreshRates
            .iter()
            .cloned()
            .take(result.refreshRatesCount as _)
            .collect::<Vec<_>>();

        let headset_info = HeadsetInfoPacket {
            device_name: device_name.into(),
            recommended_eye_resolution: (
                result.recommendedEyeWidth as _,
                result.recommendedEyeHeight as _,
            ),
            recommended_left_eye_fov,
            available_refresh_rates,
            reserved: serde_json::json!({}),
        };

        let private_identity = PrivateIdentity {
            hostname: trace_err!(env.get_string(jhostname))?.into(),
            certificate_pem: trace_err!(env.get_string(jcertificate_pem))?.into(),
            key_pem: trace_err!(env.get_string(jprivate_key))?.into(),
        };

        let (on_pause_notifier, _) = broadcast::channel(1);

        let runtime = trace_err!(Runtime::new())?;
        runtime.spawn(connection::connection_loop(
            headset_info,
            private_identity,
            on_pause_notifier.clone(),
        ));

        *MAYBE_RUNTIME.lock() = Some(runtime);
        *MAYBE_ON_PAUSE_NOTIFIER.lock() = Some(on_pause_notifier);

        Ok(result.defaultRefreshRate)
    }())
    .unwrap()
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
    STATISTICS.lock().reportFrameToBeDecoded(frame_idx);
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onFrameOutputNative(
    _: JNIEnv,
    _: JClass,
    frame_idx: i64,
) {
    STATISTICS.lock().reportDecodedFrame(frame_idx);
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onPauseNative(_: JNIEnv, _: JClass) {
    if let Some(notifier) = MAYBE_ON_PAUSE_NOTIFIER.lock().take() {
        notifier.send(()).ok();
    }

    // shutdown and wait for tasks to finish
    drop(MAYBE_RUNTIME.lock().take());

    unsafe { onPause() };
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onDestroyNative(
    env: JNIEnv,
    _: JClass,
) {
    unsafe { onDestroy(env.get_native_interface() as _) };
}
