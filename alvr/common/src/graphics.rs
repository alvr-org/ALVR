use crate::prelude::*;
use gfx_hal::Instance;

#[cfg(any(windows, target_os = "linux"))]
pub fn get_gpu_names() -> Vec<String> {
    #[cfg(windows)]
    let instance = gfx_backend_dx11::Instance::create("ALVR", 0).unwrap();
    #[cfg(target_os = "linux")]
    let instance = gfx_backend_vulkan::Instance::create("ALVR", 0).unwrap();
    let adapters = instance.enumerate_adapters();

    adapters
        .into_iter()
        .map(|a| a.info.name)
        .collect::<Vec<_>>()
}
#[cfg(not(any(windows, target_os = "linux")))]
pub fn get_gpu_names() -> Vec<String> {
    vec![]
}

pub fn get_screen_size() -> StrResult<(u32, u32)> {
    #[cfg(not(windows))]
    use winit::platform::unix::EventLoopExtUnix;
    #[cfg(windows)]
    use winit::platform::windows::EventLoopExtWindows;
    use winit::{window::*, *};

    let event_loop = event_loop::EventLoop::<Window>::new_any_thread();
    let window_handle = trace_none!(trace_err!(WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop))?
    .primary_monitor())?;
    let size = window_handle
        .size()
        .to_logical(window_handle.scale_factor());

    Ok((size.width, size.height))
}
