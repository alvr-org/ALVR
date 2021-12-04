use alvr_common::glam::UVec2;
use alvr_graphics::{BindingDesc, TARGET_FORMAT};
use alvr_session::ColorCorrectionDesc;
use wgpu::{
    BindGroup, BindingResource, BindingType, CommandEncoder, Device, Extent3d, RenderPipeline,
    TextureDescriptor, TextureDimension, TextureSampleType, TextureUsages, TextureView,
    TextureViewDimension,
};

pub struct ColorCorrectionPass {
    input: TextureView,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
}

impl ColorCorrectionPass {
    pub fn new(device: &Device, input_size: UVec2) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: input_size.x,
                height: input_size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::STORAGE_BINDING,
        });

        let input = texture.create_view(&Default::default());

        let (pipeline, bind_group) = alvr_graphics::create_default_render_pipeline(
            "color correction",
            device,
            include_str!("../resources/color_correction.wgsl"),
            vec![BindingDesc {
                index: 0,
                binding_type: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                array_size: None,
                resource: BindingResource::TextureView(&input),
            }],
            0,
        );

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
        alvr_graphics::execute_default_pass(
            encoder,
            &self.pipeline,
            &self.bind_group,
            bytemuck::bytes_of(desc),
            output,
        )
    }
}
