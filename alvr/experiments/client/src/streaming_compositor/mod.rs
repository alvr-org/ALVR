use alvr_common::glam::UVec2;
use alvr_graphics::{
    slicing::{AlignmentDirection, SlicingPass},
    GraphicsContext,
};
use alvr_session::FoveatedRenderingDesc;
use std::sync::Arc;
use wgpu::{
    Color, CommandEncoder, CommandEncoderDescriptor, RenderPassDescriptor, Texture, TextureView,
};

// Responsible for manipulating the decoded frames
pub struct StreamingCompositor {
    graphics_context: Arc<GraphicsContext>,
    slicer: SlicingPass,
}

impl StreamingCompositor {
    pub fn new(
        graphics_context: Arc<GraphicsContext>,
        target_view_size: UVec2,
        slices_count: usize,
    ) -> Self {
        let combined_size = UVec2::new(target_view_size.x * 2, target_view_size.y);

        let slicer = SlicingPass::new(
            &graphics_context.device,
            combined_size,
            2, //slices_count,
            2,
            AlignmentDirection::Input,
        );

        Self {
            graphics_context,
            slicer,
        }
    }

    pub fn input_texture(&self) -> &Texture {
        self.slicer.input_texture()
    }

    pub fn render(&self, target: &[Arc<TextureView>]) {
        let mut encoder = self
            .graphics_context
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        for (view_index, output) in target.iter().enumerate() {
            self.slicer.draw(&mut encoder, view_index, output);
        }

        self.graphics_context.queue.submit(Some(encoder.finish()));
    }
}
