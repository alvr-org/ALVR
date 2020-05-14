#![windows_subsystem = "windows"]

use alvr_common::{data::*, *};

fn main() {
    let mutex = single_instance::SingleInstance::new("alvr_server_bootstrap_mutex").unwrap();
    if mutex.is_single() {
        maybe_launch_web_server(&std::env::current_dir().unwrap());

        web_view::builder()
            .title(&format!("{} v{}", ALVR_NAME, ALVR_SERVER_VERSION))
            .content(web_view::Content::Url("http://127.0.0.1:8080"))
            .size(800, 600)
            .resizable(false)
            .frameless(false)
            .user_data(())
            .invoke_handler(|_, _| Ok(()))
            .run()
            .unwrap();
 
        maybe_kill_web_server();
    }
}