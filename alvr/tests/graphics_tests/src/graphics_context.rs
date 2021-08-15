use alvr_common::prelude::*;
use wgpu::{
    Backends, Device, DeviceDescriptor, Instance, PowerPreference, Queue, RequestAdapterOptions,
};

pub struct GraphicsContext {
    instance: Instance,
    device: Device,
    queue: Queue,
}

impl GraphicsContext {
    pub fn new() -> StrResult<Self> {
        let instance = Instance::new(Backends::PRIMARY); // todo: use Vulkan
        let adapter = trace_none!(pollster::block_on(instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: None,
            }
        )))?;
        let (device, queue) = trace_err!(pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            },
            None,
        )))?;

        Ok(Self {
            instance,
            device,
            queue,
        })
    }
}
