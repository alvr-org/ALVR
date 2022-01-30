use alvr_common::prelude::*;

pub fn get_gpu_names() -> Vec<String> {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
    let adapters = instance.enumerate_adapters(wgpu::Backends::PRIMARY);

    adapters
        .into_iter()
        .map(|a| a.get_info().name)
        .collect::<Vec<_>>()
}

#[cfg(not(target_os = "macos"))]
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

#[cfg(target_os = "macos")]
pub fn get_screen_size() -> StrResult<(u32, u32)> {
    Ok((0, 0))
}
