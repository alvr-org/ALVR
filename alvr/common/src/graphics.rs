use crate::prelude::*;

pub fn get_gpu_names() -> Vec<String> {
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let adapters = instance.enumerate_adapters(wgpu::BackendBit::PRIMARY);

    adapters
        .into_iter()
        .map(|a| a.get_info().name)
        .collect::<Vec<_>>()
}

pub fn get_screen_size() -> StrResult<(u32, u32)> {
    use winit::{event_loop::EventLoop, window::WindowBuilder};

    let event_loop = EventLoop::new();
    let window_handle = trace_none!(trace_err!(WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop))?
    .primary_monitor())?;
    let size = window_handle
        .size()
        .to_logical(window_handle.scale_factor());

    Ok((size.width, size.height))
}
