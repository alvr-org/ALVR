use crate::TARGET_FORMAT;
use alvr_common::glam::UVec2;
use wgpu::{
    BindGroup, CommandEncoder, Device, Extent3d, RenderPipeline, Texture, TextureDescriptor,
    TextureDimension, TextureUsages, TextureView, TextureViewDescriptor,
};

pub struct SlicingLayout {
    slice_size: UVec2,
    columns: u32,
}

fn get_slicing_layout(combined_size: UVec2, slice_count: usize) -> SlicingLayout {
    // only 1 or 2 slices are handled for now.
    // todo: port complete algorithm from zarik5/bridgevr-dev. It can also split vertically after
    // a certain slice count.
    if slice_count == 1 {
        SlicingLayout {
            slice_size: combined_size,
            columns: 1,
        }
    } else if slice_count == 2 {
        SlicingLayout {
            slice_size: UVec2::new(combined_size.x / 2, combined_size.y),
            columns: 2,
        }
    } else {
        unimplemented!()
    }
}

pub fn align_to_32(size: UVec2) -> UVec2 {
    UVec2::new(
        (size.x as f32 / 32_f32).ceil() as u32 * 32,
        (size.y as f32 / 32_f32).ceil() as u32 * 32,
    )
}

pub enum AlignmentDirection {
    Input,
    Output,
}

// Merge k slices then split the result into n slices
// Slices are assumed to be packed and unpacked by this same pass, following a particular layout
// determined by the number of slices and the shape of the reconstructed frame.
pub struct SlicingPass {
    input_texture: Texture,
    input_views: Vec<TextureView>,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    input_slicing_layout: SlicingLayout,
    combined_size: UVec2,
    output_slicing_layout: SlicingLayout,
    target_size: UVec2,
}

impl SlicingPass {
    pub fn new(
        device: &Device,
        combined_size: UVec2,
        input_slices_count: usize,
        output_slices_count: usize,
        alignment_direction: AlignmentDirection,
    ) -> Self {
        let input_slicing_layout = get_slicing_layout(combined_size, input_slices_count);
        let mut input_size = input_slicing_layout.slice_size;
        if matches!(alignment_direction, AlignmentDirection::Input) {
            input_size = align_to_32(input_size);
        }

        let input_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: input_size.x,
                height: input_size.y,
                // make sure the texture is still an array, even if the second texture is unused
                depth_or_array_layers: u32::max(input_slices_count as _, 2),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::STORAGE_BINDING,
        });

        let texture_view = input_texture.create_view(&Default::default());

        let input_views = (0..input_slices_count)
            .map(|idx| {
                input_texture.create_view(&TextureViewDescriptor {
                    base_array_layer: idx as _,
                    ..Default::default()
                })
            })
            .collect();

        let pipeline = super::create_default_render_pipeline(
            device,
            include_str!("../resources/slicing.wgsl"),
        );

        let bind_group = super::create_default_bind_group(device, &pipeline, &texture_view);

        let output_slicing_layout = get_slicing_layout(combined_size, output_slices_count);
        let mut target_size = output_slicing_layout.slice_size;
        if matches!(alignment_direction, AlignmentDirection::Output) {
            target_size = align_to_32(target_size);
        }

        Self {
            input_texture,
            input_views,
            pipeline,
            bind_group,
            input_slicing_layout,
            combined_size,
            output_slicing_layout,
            target_size,
        }
    }

    // Aligned slice size
    pub fn output_size(&self) -> UVec2 {
        self.target_size
    }

    // The texture has one layer for each slice
    pub fn input_texture(&self) -> &Texture {
        &self.input_texture
    }

    pub fn input_views(&self) -> &[TextureView] {
        &self.input_views
    }

    pub fn draw(&self, encoder: &mut CommandEncoder, slice_index: usize, output: &TextureView) {
        let data = [
            self.input_slicing_layout.slice_size.x as i32,
            self.input_slicing_layout.slice_size.y as i32,
            self.input_slicing_layout.columns as i32,
            self.combined_size.x as i32,
            self.combined_size.y as i32,
            (self.output_slicing_layout.slice_size.x
                * (slice_index as u32 % self.output_slicing_layout.columns)) as i32,
            (self.output_slicing_layout.slice_size.y
                * (slice_index as u32 / self.output_slicing_layout.columns)) as i32,
            self.target_size.x as i32,
            self.target_size.y as i32,
        ];

        super::execute_default_pass(
            encoder,
            &self.pipeline,
            &self.bind_group,
            bytemuck::cast_slice(&data),
            output,
        )
    }
}
