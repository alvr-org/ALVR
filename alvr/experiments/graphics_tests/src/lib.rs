pub mod compositor;
pub mod convert;

use wgpu::{Device, Instance, Queue, TextureFormat};

pub const TARGET_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

pub struct Context {
    instance: Instance,
    device: Device,
    queue: Queue,
}

impl Context {
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}
