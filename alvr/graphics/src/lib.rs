mod backend;
mod client_stream_renderer;
mod lobby;
mod passthrough_pass;

pub use backend::*;
pub use client_stream_renderer::*;
pub use lobby::*;
pub use wgpu;

use std::sync::Arc;
use wgpu::*;

#[derive(Clone)]
pub struct GraphicsContext<B: Clone = ()> {
    pub instance: Arc<Instance>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub backend_handles: B,
}
