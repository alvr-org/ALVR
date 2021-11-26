use alvr_common::log;

pub fn init_logging() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("[ALVR NATIVE-RUST]")
            .with_min_level(log::Level::Info),
    );

    alvr_common::set_panic_hook();
}
