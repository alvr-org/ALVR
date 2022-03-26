use crate::{SERVER_DATA_MANAGER, WINDOW};
use alvr_common::prelude::*;
use std::{fs, sync::Arc};

#[cfg(not(target_os = "macos"))]
pub fn get_screen_size() -> StrResult<(u32, u32)> {
    #[cfg(not(windows))]
    use winit::platform::unix::EventLoopExtUnix;
    #[cfg(windows)]
    use winit::platform::windows::EventLoopExtWindows;
    use winit::{window::*, *};

    let event_loop = event_loop::EventLoop::<Window>::new_any_thread();
    let window_handle = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .map_err(err!())?
        .primary_monitor()
        .ok_or_else(enone!())?;
    let size = window_handle
        .size()
        .to_logical(window_handle.scale_factor());

    Ok((size.width, size.height))
}

#[cfg(target_os = "macos")]
pub fn get_screen_size() -> StrResult<(u32, u32)> {
    Ok((0, 0))
}

// this thread gets interrupted when SteamVR closes
// todo: handle this in a better way
pub fn ui_thread() -> StrResult {
    const WINDOW_WIDTH: u32 = 800;
    const WINDOW_HEIGHT: u32 = 600;

    let (pos_left, pos_top) = if let Ok((screen_width, screen_height)) = get_screen_size() {
        (
            (screen_width - WINDOW_WIDTH) / 2,
            (screen_height - WINDOW_HEIGHT) / 2,
        )
    } else {
        (0, 0)
    };

    let temp_dir = tempfile::TempDir::new().map_err(err!())?;
    let user_data_dir = temp_dir.path();
    fs::File::create(temp_dir.path().join("FirstLaunchAfterInstallation")).map_err(err!())?;

    let window = Arc::new(
        alcro::UIBuilder::new()
            .content(alcro::Content::Url("http://127.0.0.1:8082"))
            .user_data_dir(user_data_dir)
            .size(WINDOW_WIDTH as _, WINDOW_HEIGHT as _)
            .custom_args(&[
                "--disk-cache-size=1",
                &format!("--window-position={pos_left},{pos_top}"),
                if SERVER_DATA_MANAGER
                    .lock()
                    .session()
                    .session_settings
                    .extra
                    .patches
                    .remove_sync_popup
                {
                    "--enable-automation"
                } else {
                    ""
                },
            ])
            .run()
            .map_err(err!())?,
    );

    *WINDOW.lock() = Some(Arc::clone(&window));

    window.wait_finish();

    // prevent panic on window.close()
    *WINDOW.lock() = None;
    crate::shutdown_runtime();

    unsafe { crate::ShutdownSteamvr() };

    Ok(())
}
