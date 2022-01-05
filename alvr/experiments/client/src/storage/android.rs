use alvr_common::prelude::*;
use jni::{
    objects::{JObject, JString, JValue},
    AttachGuard, JNIEnv,
};
use std::ffi::CString;

const MODE_PRIVATE: i32 = 0;
const CONFIG_KEY: &str = "config";

pub fn load_asset(fname: &str) -> Vec<u8> {
    let asset_manager = ndk_glue::native_activity().asset_manager();
    let fname_cstring = CString::new(fname).unwrap();
    let mut asset = asset_manager.open(fname_cstring.as_c_str()).unwrap();
    asset.get_buffer().unwrap().to_vec()
}

fn get_preferences_object<'a>(env: &'a JNIEnv<'a>) -> JObject<'a> {
    let pref_name = env.new_string("alvr-pref").unwrap();

    env.call_method(
        ndk_glue::native_activity().activity(),
        "getSharedPreferences",
        "(Ljava/lang/String;I)Landroid/content/SharedPreferences;",
        &[pref_name.into(), MODE_PRIVATE.into()],
    )
    .unwrap()
    .l()
    .unwrap()
}

pub fn load_config_string() -> String {
    let vm_ptr = ndk_glue::native_activity().vm();
    let vm = unsafe { jni::JavaVM::from_raw(vm_ptr).unwrap() };
    let env = vm.attach_current_thread().unwrap();

    let shared_preferences = get_preferences_object(&env);

    let key = env.new_string("config").unwrap();
    let default = env.new_string("").unwrap();

    let config = env
        .call_method(
            shared_preferences,
            "getString",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
            &[key.into(), default.into()],
        )
        .unwrap();

    env.get_string(config.l().unwrap().into()).unwrap().into()
}

pub fn store_config_string(config: String) {
    let vm_ptr = ndk_glue::native_activity().vm();
    let vm = unsafe { jni::JavaVM::from_raw(vm_ptr).unwrap() };
    let env = vm.attach_current_thread().unwrap();

    let shared_preferences = get_preferences_object(&env);

    let editor = env
        .call_method(
            shared_preferences,
            "edit",
            "()Landroid/content/SharedPreferences$Editor;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();

    let key = env.new_string("config").unwrap();
    let value = env.new_string(config).unwrap();
    env.call_method(
        editor,
        "putString",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;",
        &[key.into(), value.into()],
    )
    .unwrap();

    env.call_method(editor, "apply", "()V", &[]).unwrap();
}
