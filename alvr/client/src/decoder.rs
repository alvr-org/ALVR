use alvr_common::{once_cell::sync::Lazy, parking_lot::Mutex};
use jni::{
    objects::{GlobalRef, JObject},
    JNIEnv,
};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

pub static DECODER_REF: Lazy<Mutex<Option<GlobalRef>>> = Lazy::new(|| Mutex::new(None));
pub static IDR_PARSED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
pub static IDR_REQUEST_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);
pub static STREAM_TEAXTURE_HANDLE: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_DecoderThread_setWaitingNextIDR(
    _: JNIEnv,
    _: JObject,
    waiting: bool,
) {
    IDR_PARSED.store(!waiting, Ordering::Relaxed);
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

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_DecoderThread_getStreamTextureHandle(
    _: JNIEnv,
    _: JObject,
) -> i32 {
    *STREAM_TEAXTURE_HANDLE.lock()
}
