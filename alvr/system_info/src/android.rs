use alvr_common::warn;
use jni::{
    errors::Error,
    objects::{JObject, JString, JValue, JValueOwned},
    sys::jobject,
    JNIEnv, JavaVM,
};
use std::net::{IpAddr, Ipv4Addr};

pub const MICROPHONE_PERMISSION: &str = "android.permission.RECORD_AUDIO";

// Will call a JNI method and handle any Java exceptions that might occur
fn env_call_method_handled<'local>(
    env: &mut JNIEnv<'local>,
    object: &JObject,
    method: &str,
    sig: &str,
    args: &[JValue],
) -> Result<JValueOwned<'local>, Error> {
    env.call_method(object, method, sig, args)
        .map_err(|e| match e {
            jni::errors::Error::JavaException => {
                env.exception_describe().unwrap();
                let jthorw = env.exception_occurred().unwrap();
                env.exception_clear().unwrap();
                let exception_to_jstring: JString = env
                    .call_method(jthorw, "toString", "()Ljava/lang/String;", &[])
                    .unwrap()
                    .l()
                    .unwrap()
                    .into();
                let excp_string = env.get_string(&exception_to_jstring).unwrap();
                let err = String::from("Java Exception Occured: ") + excp_string.to_str().unwrap();
                // At this point even if we let this error propagate, the next unwrap will panic with only unclear "JavaException" literal,
                // so we can panic here itself with more detailed JavaException reason message and let crate::backtrace::backtrace() be printed
                // This can also be synced with server side in future if we have that feature
                panic!("{err}");
            }
            _ => e,
        })
}

pub fn vm() -> JavaVM {
    unsafe { JavaVM::from_raw(ndk_context::android_context().vm().cast()).unwrap() }
}

pub fn context() -> jobject {
    ndk_context::android_context().context().cast()
}

fn get_api_level() -> i32 {
    let vm = vm();
    let mut env = vm.attach_current_thread().unwrap();

    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
        .unwrap()
        .i()
        .unwrap()
}

pub fn try_get_permission(permission: &str) {
    let vm = vm();
    let mut env = vm.attach_current_thread().unwrap();

    let mic_perm_jstring = env.new_string(permission).unwrap();

    let permission_status = env_call_method_handled(
        &mut env,
        unsafe { &JObject::from_raw(context()) },
        "checkSelfPermission",
        "(Ljava/lang/String;)I",
        &[(&mic_perm_jstring).into()],
    )
    .unwrap()
    .i()
    .unwrap();

    if permission_status != 0 {
        let string_class = env.find_class("java/lang/String").unwrap();
        let perm_array = env
            .new_object_array(1, string_class, mic_perm_jstring)
            .unwrap();

        env_call_method_handled(
            &mut env,
            unsafe { &JObject::from_raw(context()) },
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[(&perm_array).into(), 0.into()],
        )
        .unwrap();

        // todo: handle case where permission is rejected
    }
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

pub fn product_name() -> String {
    build_string("PRODUCT")
}

fn get_system_service<'a>(env: &mut JNIEnv<'a>, service_name: &str) -> JObject<'a> {
    let service_str = env.new_string(service_name).unwrap();

    env_call_method_handled(
        env,
        unsafe { &JObject::from_raw(context()) },
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[(&service_str).into()],
    )
    .unwrap()
    .l()
    .unwrap()
}

// Note: tried and failed to use libc
pub fn local_ip() -> IpAddr {
    let vm = vm();
    let mut env = vm.attach_current_thread().unwrap();

    let wifi_manager = get_system_service(&mut env, "wifi");
    let wifi_info = env_call_method_handled(
        &mut env,
        &wifi_manager,
        "getConnectionInfo",
        "()Landroid/net/wifi/WifiInfo;",
        &[],
    )
    .unwrap()
    .l()
    .unwrap();
    let ip_i32 = env_call_method_handled(&mut env, &wifi_info, "getIpAddress", "()I", &[])
        .unwrap()
        .i()
        .unwrap();

    let ip_arr = ip_i32.to_le_bytes();

    IpAddr::V4(Ipv4Addr::new(ip_arr[0], ip_arr[1], ip_arr[2], ip_arr[3]))
}

// This is needed to avoid wifi scans that disrupt streaming.
// Code inspired from https://github.com/Meumeu/WiVRn/blob/master/client/application.cpp
pub fn set_wifi_lock(enabled: bool) {
    let vm = vm();
    let mut env = vm.attach_current_thread().unwrap();

    let wifi_manager = get_system_service(&mut env, "wifi");

    fn set_lock<'a>(env: &mut JNIEnv<'a>, lock: &JObject, enabled: bool) {
        env_call_method_handled(env, lock, "setReferenceCounted", "(Z)V", &[false.into()]).unwrap();
        env_call_method_handled(
            env,
            lock,
            if enabled { "acquire" } else { "release" },
            "()V",
            &[],
        )
        .unwrap();

        let lock_is_aquired = env_call_method_handled(env, &lock, "isHeld", "()Z", &[])
            .unwrap()
            .z()
            .unwrap();

        if lock_is_aquired != enabled {
            warn!("Failed to set wifi lock: expected {enabled}, got {lock_is_aquired}");
        }
    }

    let wifi_lock_jstring = env.new_string("alvr_wifi_lock").unwrap();
    let wifi_lock = env_call_method_handled(
        &mut env,
        &wifi_manager,
        "createWifiLock",
        "(ILjava/lang/String;)Landroid/net/wifi/WifiManager$WifiLock;",
        &[
            if get_api_level() >= 29 {
                // Recommended for virtual reality since it disables WIFI scans
                4 // WIFI_MODE_FULL_LOW_LATENCY
            } else {
                3 // WIFI_MODE_FULL_HIGH_PERF
            }
            .into(),
            (&wifi_lock_jstring).into(),
        ],
    )
    .unwrap()
    .l()
    .unwrap();
    set_lock(&mut env, &wifi_lock, enabled);

    let multicast_lock_jstring = env.new_string("alvr_multicast_lock").unwrap();
    let multicast_lock = env_call_method_handled(
        &mut env,
        &wifi_manager,
        "createMulticastLock",
        "(Ljava/lang/String;)Landroid/net/wifi/WifiManager$MulticastLock;",
        &[(&multicast_lock_jstring).into()],
    )
    .unwrap()
    .l()
    .unwrap();
    set_lock(&mut env, &multicast_lock, enabled);
}

pub fn get_battery_status() -> (f32, bool) {
    let vm = vm();
    let mut env = vm.attach_current_thread().unwrap();

    let intent_action_jstring = env
        .new_string("android.intent.action.BATTERY_CHANGED")
        .unwrap();
    let intent_filter = env
        .new_object(
            "android/content/IntentFilter",
            "(Ljava/lang/String;)V",
            &[(&intent_action_jstring).into()],
        )
        .unwrap();
    let battery_intent = env_call_method_handled(&mut env,
            unsafe { &JObject::from_raw(context()) },
            "registerReceiver",
            "(Landroid/content/BroadcastReceiver;Landroid/content/IntentFilter;)Landroid/content/Intent;",
            &[(&JObject::null()).into(), (&intent_filter).into()],
        )
        .unwrap()
        .l()
        .unwrap();

    let level_jstring = env.new_string("level").unwrap();
    let level = env_call_method_handled(
        &mut env,
        &battery_intent,
        "getIntExtra",
        "(Ljava/lang/String;I)I",
        &[(&level_jstring).into(), (-1).into()],
    )
    .unwrap()
    .i()
    .unwrap();
    let scale_jstring = env.new_string("scale").unwrap();
    let scale = env_call_method_handled(
        &mut env,
        &battery_intent,
        "getIntExtra",
        "(Ljava/lang/String;I)I",
        &[(&scale_jstring).into(), (-1).into()],
    )
    .unwrap()
    .i()
    .unwrap();

    let plugged_jstring = env.new_string("plugged").unwrap();
    let plugged = env_call_method_handled(
        &mut env,
        &battery_intent,
        "getIntExtra",
        "(Ljava/lang/String;I)I",
        &[(&plugged_jstring).into(), (-1).into()],
    )
    .unwrap()
    .i()
    .unwrap();

    (level as f32 / scale as f32, plugged > 0)
}
