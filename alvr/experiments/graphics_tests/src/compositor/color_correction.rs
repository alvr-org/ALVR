use alvr_session::ColorCorrectionDesc;
use wgpu::{BindGroup, CommandEncoder, Device, RenderPipeline, TextureView};

pub struct ColorCorrectionPass {
    input: TextureView,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
}

impl ColorCorrectionPass {
    pub fn new(device: &Device, input_size: (u32, u32)) -> Self {
        let input = super::create_default_texture(device, input_size);

        let pipeline = super::create_default_render_pipeline(
            device,
            include_str!("../../resources/color_correction.wgsl"),
        );

        let bind_group = super::create_default_bind_group(device, &pipeline, &input);

        Self {
            input,
            pipeline,
            bind_group,
        }
    }

    pub fn input(&self) -> &TextureView {
        &self.input
    }

    pub fn draw(
        &self,
        encoder: &mut CommandEncoder,
        desc: &ColorCorrectionDesc,
        output: &TextureView,
    ) {
        super::execute_default_pass(
            encoder,
            &self.pipeline,
            &self.bind_group,
            bytemuck::bytes_of(desc),
            output,
        )
    }
}
