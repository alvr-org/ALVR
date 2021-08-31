use crate::TARGET_FORMAT;
use wgpu::{
    BindGroup, CommandEncoder, Device, Extent3d, RenderPipeline, TextureDescriptor,
    TextureDimension, TextureUsages, TextureView, TextureViewDescriptor,
};

pub struct SlicingLayout {
    slice_width: i32,
    slice_height: i32,
    columns: i32,
}

fn get_slicing_layout(combined_size: (u32, u32), slice_count: usize) -> SlicingLayout {
    // only 1 or 2 slices are handled for now.
    // todo: port complete algorithm from zarik5/bridgevr-dev. It can also split vertically after
    // a certain slice count.
    if slice_count == 1 {
        SlicingLayout {
            slice_width: combined_size.0 as i32,
            slice_height: combined_size.1 as i32,
            columns: 1,
        }
    } else if slice_count == 2 {
        SlicingLayout {
            slice_width: combined_size.0 as i32 / 2,
            slice_height: combined_size.1 as i32,
            columns: 2,
        }
    } else {
        unimplemented!()
    }
}

pub fn align_to_32(size: (i32, i32)) -> (i32, i32) {
    (
        (size.0 as f32 / 32_f32).ceil() as i32 * 32,
        (size.1 as f32 / 32_f32).ceil() as i32 * 32,
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
    inputs: Vec<TextureView>,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    input_slicing_layout: SlicingLayout,
    combined_size: (i32, i32),
    output_slicing_layout: SlicingLayout,
    target_size: (i32, i32),
}

impl SlicingPass {
    pub fn new(
        device: &Device,
        combined_size: (u32, u32),
        input_slices_count: usize,
        output_slices_count: usize,
        alignment_direction: AlignmentDirection,
    ) -> Self {
        let input_slicing_layout = get_slicing_layout(combined_size, input_slices_count);
        let mut input_size = (
            input_slicing_layout.slice_width,
            input_slicing_layout.slice_height,
        );
        if matches!(alignment_direction, AlignmentDirection::Input) {
            input_size = align_to_32(input_size);
        }

        let input_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: input_size.0 as u32,
                height: input_size.1 as u32,
                // make sure the texture is still an array, even if the second texture is unused
                depth_or_array_layers: u32::max(input_slices_count as _, 2),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::STORAGE_BINDING,
        });

        let inputs_view = input_texture.create_view(&Default::default());

        let inputs = (0..input_slices_count)
            .map(|idx| {
                input_texture.create_view(&TextureViewDescriptor {
                    base_array_layer: idx as _,
                    ..Default::default()
                })
            })
            .collect();

        let pipeline = super::create_default_render_pipeline(
            device,
            include_str!("../../resources/slicing.wgsl"),
        );

        let bind_group = super::create_default_bind_group(device, &pipeline, &inputs_view);

        let output_slicing_layout = get_slicing_layout(combined_size, output_slices_count);
        let mut target_size = (
            output_slicing_layout.slice_width,
            output_slicing_layout.slice_height,
        );
        if matches!(alignment_direction, AlignmentDirection::Output) {
            target_size = align_to_32(target_size);
        }

        Self {
            inputs,
            pipeline,
            bind_group,
            input_slicing_layout,
            combined_size: (combined_size.0 as i32, combined_size.1 as i32),
            output_slicing_layout,
            target_size,
        }
    }

    // Aligned slice size
    pub fn output_size(&self) -> (u32, u32) {
        (self.target_size.0 as u32, self.target_size.1 as u32)
    }

    pub fn input(&self) -> &[TextureView] {
        &self.inputs
    }

    pub fn draw(&self, encoder: &mut CommandEncoder, slice_index: usize, output: &TextureView) {
        let data = [
            self.input_slicing_layout.slice_width,
            self.input_slicing_layout.slice_height,
            self.input_slicing_layout.columns,
            self.combined_size.0,
            self.combined_size.1,
            self.output_slicing_layout.slice_width
                * (slice_index as i32 % self.output_slicing_layout.columns),
            self.output_slicing_layout.slice_height
                * (slice_index as i32 / self.output_slicing_layout.columns),
            self.target_size.0,
            self.target_size.1,
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
