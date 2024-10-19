

use jni::{objects::JObject, sys::jobject, JNIEnv, JavaVM};

pub fn vm() -> JavaVM {
    unsafe { JavaVM::from_raw(ndk_context::android_context().vm().cast()).unwrap() }
}

pub fn context() -> jobject {
    ndk_context::android_context().context().cast()
}

pub fn build_string(ty: &str) -> String {
    let vm = vm();
    let mut env = vm.attach_current_thread().unwrap();

    let jname = env
        .get_static_field("android/os/Build", ty, "Ljava/lang/String;")
        .unwrap()
        .l()
        .unwrap();
    let name_raw = env.get_string((&jname).into()).unwrap();

    name_raw.to_string_lossy().as_ref().to_owned()
}

pub fn device_name() -> String {
    build_string("DEVICE")
}

pub fn model_name() -> String {
    build_string("MODEL")
}

pub fn manufacturer_name() -> String {
    build_string("MANUFACTURER")
}