use alvr_common::{lazy_static, prelude::*};
use wgpu::Adapter;

lazy_static! {
    static ref GPU_ADAPTERS: Vec<Adapter> = {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

        instance
            .enumerate_adapters(wgpu::Backends::PRIMARY)
            .collect()
    };
}

pub enum GpuVendor {
    Nvidia,
    Amd,
    Other,
}

pub fn get_gpu_vendor() -> GpuVendor {
    match GPU_ADAPTERS[0].get_info().vendor {
        0x10de => GpuVendor::Nvidia,
        0x1002 => GpuVendor::Amd,
        _ => GpuVendor::Other,
    }
}

pub fn get_gpu_names() -> Vec<String> {
    GPU_ADAPTERS
        .iter()
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
