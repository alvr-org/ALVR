mod alignment;
mod color_correction;
mod compositing;
mod convert;
mod foveated_rendering;
mod slicing;

use ash::vk;
pub use convert::*;

use alignment::AlignmentPipeline;
use alvr_common::prelude::*;
use alvr_session::{ColorCorrectionDesc, Fov, FoveatedRenderingDesc};
use color_correction::ColorCorrectionPipeline;
use compositing::{CompositingPipeline, Layer};
use foveated_rendering::{Direction, FoveatedRenderingPipeline};
use gpu_alloc::MemoryBlock;
use slicing::SlicingPipeline;
use wgpu::{
    BindGroup, CommandEncoderDescriptor, Device, Instance, Queue, RenderPass,
    ShaderModuleDescriptor, ShaderSource, Texture, TextureFormat, TextureView, VertexState,
};

pub const TARGET_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

fn draw_quad(mut pass: RenderPass) {
    pass.draw_indexed(0..6, 0, 0..1);

    // here the pass is dropped and applied to the command encoder
}

pub struct Context {
    instance: Instance,
    device: Device,
    queue: Queue,
    raw_device: ash::Device,
}

impl Context {
    pub fn instance(&self) -> &Instance {
        &self.instance
    }
}

pub struct Swapchain {
    textures: Vec<Texture>,
    raw_images: Vec<vk::Image>,
    memory: Vec<MemoryBlock<vk::DeviceMemory>>,
    bind_groups: Vec<Vec<BindGroup>>, //[0]: texture index, [1]: array index
    current_index: usize,
}

impl Swapchain {
    pub fn enumerate_images(&self) -> &[vk::Image] {
        &self.raw_images
    }

    // This is used in place of acquire_image + wait_image + release_image
    pub fn next_index(&mut self) -> usize {
        self.current_index = (self.current_index + 1) % self.textures.len();

        self.current_index
    }
}

pub struct CompositionLayerView<'a> {
    pub swapchain: &'a Swapchain,
    pub image_rect: openxr_sys::Rect2Di,
    pub image_array_index: usize,
    pub fov: Fov,
}

// Most of the compositor structure cannot be modified after creation. Some parameters like FOV for
// FFR and color correction parameters (if enabled) can be changed on the fly. Enabling/disabling
// FFR and changing the target eye size require recreating the compositor completely, which might
// cause a lag spike.
pub struct Compositor {
    context: Context,
    inner: CompositingPipeline,
    color_corrector: ColorCorrectionPipeline,
    foveation_encoder: Option<FoveatedRenderingPipeline>,
    slicer: SlicingPipeline,
    aligner: AlignmentPipeline,

    // todo: move to client
    aligner2: AlignmentPipeline,
    slicer2: SlicingPipeline,
    foveation_decoder: Option<FoveatedRenderingPipeline>,

    output_texture_views: Vec<TextureView>,
    output_raw_images: Vec<vk::Image>,
    output_size: (u32, u32),
}

impl Compositor {
    pub fn new(
        context: Context,
        target_eye_size: (u32, u32), // expected size of a layer after cropping
        foveation_desc: Option<&FoveatedRenderingDesc>,
        slices_count: usize,
    ) -> Self {
        let quad_shader = context
            .device
            .create_shader_module(&ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Wgsl(include_str!("../../resources/quad.wgsl").into()),
            });

        let quad_vertex_state = VertexState {
            module: &quad_shader,
            entry_point: "main",
            buffers: &[],
        };

        let inner = CompositingPipeline::new(&context.device, quad_vertex_state.clone());

        let color_corrector = ColorCorrectionPipeline::new();

        let mut output_size = target_eye_size;

        let foveation_encoder = foveation_desc
            .map(|desc| {
                FoveatedRenderingPipeline::new(
                    Direction::Encoding,
                    target_eye_size,
                    desc,
                    Fov {
                        left: 45_f32,
                        right: 45_f32,
                        top: 45_f32,
                        bottom: 45_f32,
                    },
                )
            })
            .map(|(encoder, encoded_size)| {
                output_size = encoded_size;

                encoder
            });

        let foveation_decoder = foveation_desc
            .map(|desc| {
                FoveatedRenderingPipeline::new(
                    Direction::Decoding,
                    target_eye_size,
                    desc,
                    Fov {
                        left: 45_f32,
                        right: 45_f32,
                        top: 45_f32,
                        bottom: 45_f32,
                    },
                )
            })
            .map(|(decoder, _)| decoder);

        let (slicer, sliced_size) = SlicingPipeline::new(output_size, 2, slices_count);

        let (slicer2, _) = SlicingPipeline::new(output_size, slices_count, 2);

        let output_size = alignment::align_to_32(sliced_size);

        let aligner = AlignmentPipeline::new(sliced_size, output_size);

        let aligner2 = AlignmentPipeline::new(output_size, sliced_size);

        Self {
            context,
            inner,
            color_corrector,
            foveation_encoder,
            slicer,
            aligner,
            aligner2,
            slicer2,
            foveation_decoder,
            output_texture_views: todo!(),
            output_raw_images: todo!(),
            output_size,
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    fn swapchain(
        &self,
        textures: Vec<Texture>,
        raw_images: Vec<vk::Image>,
        memory: Vec<MemoryBlock<vk::DeviceMemory>>,
        array_size: u32,
    ) -> Swapchain {
        let bind_groups = textures
            .iter()
            .map(|texture| {
                (0..array_size)
                    .map(|array_index| {
                        self.inner
                            .create_bind_group(&self.context.device, texture, array_index)
                    })
                    .collect()
            })
            .collect();

        Swapchain {
            textures,
            raw_images,
            memory,
            bind_groups,
            current_index: 0,
        }
    }

    // image size used for encoding
    pub fn output_size(&self) -> (u32, u32) {
        self.output_size
    }

    pub fn output(&self) -> &[vk::Image] {
        &self.output_raw_images
    }

    // The function is blocking but it should finish quite fast.
    pub fn end_frame(
        &self,
        layers: &[&[CompositionLayerView]],
        color_correction: Option<ColorCorrectionDesc>,
    ) {
        for views in &*layers {
            assert_eq!(views.len(), 2);
        }

        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        for view_index in 0..2 {
            let layers = layers.iter().map(|layer| {
                let view = &layer[view_index];
                let swapchain = &view.swapchain;

                Layer {
                    bind_group: &swapchain.bind_groups[swapchain.current_index]
                        [view.image_array_index],
                    rect: view.image_rect,
                }
            });

            self.inner
                .draw(&mut encoder, layers, &self.output_texture_views[0]);
        }

        self.context.queue.submit(Some(encoder.finish()));
    }
}
