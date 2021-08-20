use super::{Compositor, Context, Swapchain};
use alvr_common::prelude::*;
use wgpu::{Backends, DeviceDescriptor, Features, Instance, Limits, RequestAdapterOptions};

impl Context {
    // For the Vulkan layer. The Vulkan objects must not be destroyed before the Context is dropped.
    #[cfg(target_os = "linux")]
    pub unsafe fn from_vulkan(/* ... */) -> StrResult<Self> {
        // currently wgpu does not support externally managed vulkan objects
        todo!()
    }

    // For OpenVR on Windows
    #[cfg(windows)]
    pub fn new_d3d11(adapter_index: usize) -> StrResult<Self> {
        let instance = Instance::new(Backends::DX11);
        let adapter = instance
            .enumerate_adapters(Backends::DX11)
            .nth(adapter_index)
            .ok_or_else(|| format!("Adapter at index {} not available", adapter_index))?;

        let (device, queue) = trace_err!(pollster::block_on(
            adapter.request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::PUSH_CONSTANTS,
                    limits: Limits {
                        max_push_constant_size: 128,
                        ..Limits::downlevel_defaults()
                    }
                    .using_resolution(adapter.limits()),
                },
                None,
            )
        ))?;

        Ok(Self {
            instance,
            device,
            queue,
        })
    }

    // For debug
    pub fn new_any() -> StrResult<Self> {
        let instance = Instance::new(Backends::PRIMARY);
        let adapter =
            pollster::block_on(instance.request_adapter(&RequestAdapterOptions::default()))
                .unwrap();

        let (device, queue) = trace_err!(pollster::block_on(
            adapter.request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::PUSH_CONSTANTS,
                    limits: Limits {
                        max_push_constant_size: 128,
                        ..Limits::downlevel_defaults()
                    }
                    .using_resolution(adapter.limits()),
                },
                None,
            )
        ))?;

        Ok(Self {
            instance,
            device,
            queue,
        })
    }
}

impl Compositor {
    // For the Vulkan layer. The textures must not be destroyed before the Compositor is dropped.
    #[cfg(target_os = "linux")]
    pub unsafe fn swapchain_from_vulkan(&self /* ... */) -> StrResult<Swapchain> {
        // currently wgpu does not support externally managed vulkan objects
        todo!()
    }
}
