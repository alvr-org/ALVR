#![windows_subsystem = "windows"]

use alvr_common::process::*;

fn main() {
    let mutex = single_instance::SingleInstance::new("alvr_server_bootstrap_mutex").unwrap();
    if mutex.is_single() {
        maybe_launch_web_server(&std::env::current_dir().unwrap());

        let window = alcro::UIBuilder::new()
            .content(alcro::Content::Url("http://127.0.0.1:8082"))
            .size(800, 600)
            .run();
        window.wait_finish();

        maybe_kill_web_server();
    }
}
