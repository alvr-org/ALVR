use alvr_common::prelude::*;
use alvr_graphics::GraphicsContext;
use rend3::{ExtendedAdapterInfo, InstanceAdapterDevice, RendererMode, Vendor};
use std::sync::Arc;
use wgpu::{Backend, DeviceType};

// Responsible for rendering the lobby room or HUD
pub struct SceneRenderer {
    inner: Arc<rend3::Renderer>,
}

impl SceneRenderer {
    pub fn new(graphics_context: &GraphicsContext) -> StrResult<Self> {
        let iad = InstanceAdapterDevice {
            instance: Arc::clone(&graphics_context.instance),
            adapter: Arc::clone(&graphics_context.adapter),
            device: Arc::clone(&graphics_context.device),
            queue: Arc::clone(&graphics_context.queue),
            mode: RendererMode::GPUPowered,
            info: ExtendedAdapterInfo {
                name: "".into(),
                vendor: Vendor::Unknown(0),
                device: 0,
                device_type: DeviceType::Other,
                backend: Backend::Vulkan,
            },
        };

        let renderer = trace_err!(rend3::Renderer::new(iad, None))?;

        Ok(Self { inner: renderer })
    }
}
