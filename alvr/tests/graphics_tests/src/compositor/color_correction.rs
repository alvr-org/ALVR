use alvr_session::ColorCorrectionDesc;
use wgpu::{CommandEncoder, RenderPassColorAttachment, RenderPassDescriptor, Texture};

pub struct ColorCorrectionPipeline {
    input: Texture,
}

impl ColorCorrectionPipeline {
    pub fn new() -> Self {
        todo!()
    }

    pub fn input(&self) -> &Texture {
        &self.input
    }
}
