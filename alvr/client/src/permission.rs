use jni::JavaVM;

const MICROPHONE_PERMISSION: &str = "android.permission.RECORD_AUDIO";

pub fn try_get_microphone_permission() {
    let android_context = ndk_context::android_context();

    let vm = unsafe { JavaVM::from_raw(android_context.vm().cast()).unwrap() };
    let env = vm.attach_current_thread().unwrap();
    let context = android_context.context().cast();

    let mic_perm_jstring = env.new_string(MICROPHONE_PERMISSION).unwrap();

    let permission_status = env
        .call_method(
            context,
            "checkSelfPermission",
            "(Ljava/lang/String;)I",
            &[mic_perm_jstring.into()],
        )
        .unwrap()
        .i()
        .unwrap();

    if permission_status != 0 {
        let string_class = env.find_class("java/lang/String").unwrap();
        let perm_array = env
            .new_object_array(1, string_class, mic_perm_jstring)
            .unwrap();

        env.call_method(
            context,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[perm_array.into(), 0.into()],
        )
        .unwrap();

        // todo: handle case where permission is rejected
    }
}
