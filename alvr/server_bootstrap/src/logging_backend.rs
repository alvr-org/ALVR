use alvr_common::logging::*;

pub fn init_logging() {
    set_show_error_fn_and_panic_hook(|message| {
        let message = message.to_owned();
        msgbox::create("Failed to launch ALVR", &message, msgbox::IconType::Error);
    });
}
