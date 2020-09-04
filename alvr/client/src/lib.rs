// include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use jni::*;
use jni::objects::*;

#[no_mangle]
pub extern "system" fn Java_com_polygraphene_alvr_Utils_setFrameLogEnabled(env: JNIEnv, _class: JClass) {
}

// #[no_mangle]
// pub extern "C" fn Java_com_polygraphene_alvr_Utils_setFrameLogEnabled(env: JNIEnv, _class: JClass) {
// }

// #[no_mangle]
// pub extern "C" fn