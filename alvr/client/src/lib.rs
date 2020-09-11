mod connection;
mod logging_backend;
mod statistics_manager;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{data::*, logging::*, sockets::*, *};
use jni::{objects::*, *};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use statistics_manager::StatisticsManager;
use std::thread;
use tokio::{sync::broadcast, runtime::Runtime};

lazy_static! {
    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
    static ref MAYBE_SHUTDOWN_NOTIFIER: Mutex<Option<broadcast::Sender<()>>> = Mutex::new(None);
    static ref MAYBE_INPUT_SENDER: Mutex<Option<StreamSender<VideoPacket>>> = Mutex::new(None);
    static ref MAYBE_MICROPHONE_SENDER: Mutex<Option<StreamSender<AudioPacket>>> = Mutex::new(None);
    static ref STATISTICS: Mutex<StatisticsManager> = Mutex::new(StatisticsManager::new());
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

fn init() -> StrResult {
    Ok(())
}

// This is the native entry point. It does not need any parameters
#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_initNativeRuntime(
    _: JNIEnv,
    _: JClass,
) {
    logging_backend::init_logging();

    show_err(|| -> StrResult {
        *MAYBE_RUNTIME.lock() = Some(trace_err!(Runtime::new())?);

        Ok(())
    }())
    .ok();

    // thread::spawn(|| {
    //     show_err(|| -> StrResult {
    //         let mut runtime = Runtime::new().unwrap();
    //         runtime.block_on(async move {
    //             connection::
    //         });
    //         Ok(())
    //     }())
    //     .ok()
    // });
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_surfaceCreatedNative(
    _: JNIEnv,
    _: JClass,
    surface: JObject,
) {
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_surfaceChangedNative(
    _: JNIEnv,
    _: JClass,
    surface: JObject,
) {
}

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_OvrActivity_surfaceDestroyedNative(
    _: JNIEnv,
    _: JClass,
) {
}
