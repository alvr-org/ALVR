use alvr_common::warn;
use jni::{
    Env, JavaVM,
    errors::Result as JniResult,
    jni_sig, jni_str,
    objects::{JObject, JString},
    refs::Reference,
    strings::JNIStr,
    sys::jobject,
};
use std::{
    ffi::CStr,
    net::{IpAddr, Ipv4Addr},
};

pub const MICROPHONE_PERMISSION: &str = "android.permission.RECORD_AUDIO";

pub fn vm() -> JavaVM {
    unsafe { JavaVM::from_raw(ndk_context::android_context().vm().cast()) }
}

pub fn context() -> jobject {
    ndk_context::android_context().context().cast()
}

fn get_api_level() -> i32 {
    vm().attach_current_thread(|env| {
        env.get_static_field(
            jni_str!("android/os/Build$VERSION"),
            jni_str!("SDK_INT"),
            jni_sig!("I"),
        )?
        .i()
    })
    .unwrap()
}

pub fn try_get_permission(permission: &str) {
    vm().attach_current_thread(|env| {
        let mic_perm_jstring = env.new_string(permission)?;

        let permission_status = env
            .call_method(
                unsafe { JObject::global_kind_from_raw(context()) },
                jni_str!("checkSelfPermission"),
                jni_sig!("(Ljava/lang/String;)I"),
                &[(&mic_perm_jstring).into()],
            )?
            .i()?;

        if permission_status != 0 {
            let perm_array =
                env.new_object_array(1, jni_str!("java/lang/String"), mic_perm_jstring)?;

            env.call_method(
                unsafe { JObject::global_kind_from_raw(context()) },
                jni_str!("requestPermissions"),
                jni_sig!("([Ljava/lang/String;I)V"),
                &[(&perm_array).into(), 0.into()],
            )?;
            // todo: handle case where permission is rejected
        }

        JniResult::Ok(())
    })
    .unwrap();
}

pub fn build_string(ty: &CStr) -> String {
    vm().attach_current_thread(|env| {
        let jname = env
            .get_static_field(
                jni_str!("android/os/Build"),
                JNIStr::from_cstr(ty).unwrap(),
                jni_sig!("Ljava/lang/String;"),
            )?
            .l()?;
        JniResult::Ok(env.cast_local::<JString>(jname)?.to_string())
    })
    .unwrap()
}

pub fn device_name() -> String {
    build_string(c"DEVICE")
}

pub fn model_name() -> String {
    build_string(c"MODEL")
}

pub fn manufacturer_name() -> String {
    build_string(c"MANUFACTURER")
}

pub fn product_name() -> String {
    build_string(c"PRODUCT")
}

fn get_system_service<'a>(env: &mut Env<'a>, service_name: &str) -> JniResult<JObject<'a>> {
    let service_str = env.new_string(service_name)?;

    env.call_method(
        unsafe { JObject::global_kind_from_raw(context()) },
        jni_str!("getSystemService"),
        jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
        &[(&service_str).into()],
    )?
    .l()
}

// Note: tried and failed to use libc
pub fn local_ip() -> IpAddr {
    vm().attach_current_thread(|env| {
        let wifi_manager = get_system_service(env, "wifi")?;
        let wifi_info = env
            .call_method(
                wifi_manager,
                jni_str!("getConnectionInfo"),
                jni_sig!("()Landroid/net/wifi/WifiInfo;"),
                &[],
            )?
            .l()?;
        let ip_i32 = env
            .call_method(wifi_info, jni_str!("getIpAddress"), jni_sig!("()I"), &[])?
            .i()?;

        let ip_arr = ip_i32.to_le_bytes();

        JniResult::Ok(IpAddr::V4(Ipv4Addr::new(
            ip_arr[0], ip_arr[1], ip_arr[2], ip_arr[3],
        )))
    })
    .unwrap()
}

// This is needed to avoid wifi scans that disrupt streaming.
// Code inspired from https://github.com/Meumeu/WiVRn/blob/master/client/application.cpp
pub fn set_wifi_lock(enabled: bool) {
    vm().attach_current_thread(|env| {
        let wifi_manager = get_system_service(env, "wifi")?;

        fn set_lock<'a>(env: &mut Env<'a>, lock: &JObject, enabled: bool) -> JniResult<()> {
            env.call_method(
                lock,
                jni_str!("setReferenceCounted"),
                jni_sig!("(Z)V"),
                &[false.into()],
            )?;
            env.call_method(
                &lock,
                if enabled {
                    jni_str!("acquire")
                } else {
                    jni_str!("release")
                },
                jni_sig!("()V"),
                &[],
            )?;

            let lock_is_aquired = env
                .call_method(lock, jni_str!("isHeld"), jni_sig!("()Z"), &[])?
                .z()?;

            if lock_is_aquired != enabled {
                warn!("Failed to set wifi lock: expected {enabled}, got {lock_is_aquired}");
            }

            JniResult::Ok(())
        }

        let wifi_lock_jstring = env.new_string("alvr_wifi_lock")?;
        let wifi_lock = env
            .call_method(
                &wifi_manager,
                jni_str!("createWifiLock"),
                jni_sig!("(ILjava/lang/String;)Landroid/net/wifi/WifiManager$WifiLock;"),
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
            )?
            .l()?;
        set_lock(env, &wifi_lock, enabled)?;

        let multicast_lock_jstring = env.new_string("alvr_multicast_lock")?;
        let multicast_lock = env
            .call_method(
                wifi_manager,
                jni_str!("createMulticastLock"),
                jni_sig!("(Ljava/lang/String;)Landroid/net/wifi/WifiManager$MulticastLock;"),
                &[(&multicast_lock_jstring).into()],
            )?
            .l()?;
        set_lock(env, &multicast_lock, enabled)?;

        JniResult::Ok(())
    })
    .unwrap();
}

pub fn get_battery_status() -> (f32, bool) {
    vm().attach_current_thread(|env| {
        let intent_action_jstring = env.new_string("android.intent.action.BATTERY_CHANGED")?;
        let intent_filter = env.new_object(
            jni_str!("android/content/IntentFilter"),
            jni_sig!("(Ljava/lang/String;)V"),
            &[(&intent_action_jstring).into()],
        )?;
        let battery_intent = env
            .call_method(
                unsafe { JObject::global_kind_from_raw(context()) },
                jni_str!("registerReceiver"),
                jni_sig!(
                    "(Landroid/content/BroadcastReceiver;Landroid/content/IntentFilter;)Landroid/content/Intent;"
                ),
                &[(&JObject::null()).into(), (&intent_filter).into()],
            )?
            .l()?;

        fn get_battery_value<'a>(env: &mut Env<'a>, battery_intent: &JObject, key: &str) -> JniResult<i32> {
            let key_jstring = env.new_string(key)?;
            env.call_method(
                battery_intent,
                jni_str!("getIntExtra"),
                jni_sig!("(Ljava/lang/String;I)I"),
                &[(&key_jstring).into(), (-1).into()],
            )?
            .i()
        }

        let level = get_battery_value(env, &battery_intent, "level")?;
        let scale = get_battery_value(env, &battery_intent, "scale")?;
        let plugged = get_battery_value(env, &battery_intent, "plugged")?;

        JniResult::Ok((level as f32 / scale as f32, plugged > 0))
    })
    .unwrap()
}
