use alvr_graphics::{
    slicing::{AlignmentDirection, SlicingPass},
    Context,
};
use openxr::{self as xr, sys};
use std::sync::Arc;
use wgpu::{
    Color, CommandEncoder, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, TextureView,
};

struct StreamingCompositor {
    slicer: SlicingPass,
}

impl StreamingCompositor {
    pub fn new(context: Arc<Context>, target_view_size: (u32, u32)) -> Self {
        let combined_size = (target_view_size.0 * 2, target_view_size.1);

        let slicer = SlicingPass::new(
            context.device(),
            combined_size,
            1,
            2,
            AlignmentDirection::Input,
        );

        Self { slicer }
    }

    pub fn draw(&self, encoder: &mut CommandEncoder, output: &TextureView) {
        todo!()
    }
}

// This will be replaced by bevy renderer
pub struct Renderer {
    context: Arc<Context>,
    streaming_compositor: Option<StreamingCompositor>,
    idle_swapchain: [Vec<TextureView>; 2],
}

impl Renderer {
    pub fn new(context: Arc<Context>, idle_swapchain: [Vec<TextureView>; 2]) -> Self {
        Self {
            context,
            streaming_compositor: None,
            idle_swapchain,
        }
    }

    // None means the stream stopped
    pub fn set_streaming(&mut self, streaming_compositor: Option<StreamingCompositor>) {
        self.streaming_compositor = streaming_compositor;
    }

    pub fn render(&self, swapchain_index: usize) {
        let mut encoder = self
            .context
            .device()
            .create_command_encoder(&CommandEncoderDescriptor::default());

        if let Some(compositor) = &self.streaming_compositor {
            todo!()
        } else {
            for view_idx in 0..2 {
                // just clear with solid red
                let mut _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    color_attachments: &[RenderPassColorAttachment {
                        view: &self.idle_swapchain[view_idx][swapchain_index],
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::RED),
                            store: true,
                        },
                    }],
                    ..Default::default()
                });
            }
        }

        self.context.queue().submit(Some(encoder.finish()));
        pollster::block_on(self.context.queue().on_submitted_work_done());
    }
}
