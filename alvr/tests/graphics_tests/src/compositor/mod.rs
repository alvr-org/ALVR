mod alignment;
mod color_correction;
mod composing;
mod convert;
mod foveated_rendering;
mod slicing;

pub use convert::*;

use alignment::AlignmentPipeline;
use alvr_common::prelude::*;
use alvr_session::{ColorCorrectionDesc, Fov, FoveatedRenderingDesc};
use color_correction::ColorCorrectionPipeline;
use foveated_rendering::{Direction, FoveatedRenderingPipeline};
use parking_lot::{Mutex, MutexGuard};
use slicing::SlicingPipeline;
use std::sync::atomic::{AtomicUsize, Ordering};
use wgpu::{Device, Instance, Queue, Texture, TextureFormat, TextureUsages};

use self::{alignment::align_to_32, composing::ComposingPipeline};

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

    pub fn create_swapchain(
        &self,
        count: Option<usize>,
        usages: TextureUsages,
        format: TextureFormat,
        sample_count: u32,
        width: u32,
        height: u32,
        cubemap: bool,
        array_size: u32,
        mip_count: u32,
    ) -> Swapchain {
        let count = count.unwrap_or(3);

        todo!()
    }
}

pub struct Swapchain {
    textures: Vec<Mutex<Texture>>,
    last_presented_index: AtomicUsize,
}

impl Swapchain {
    pub fn enumerate_images(&self) -> Vec<MutexGuard<Texture>> {
        self.textures.iter().map(|tex| tex.lock()).collect()
    }

    // todo: acquire_image, wait_image, release_image
}

pub struct CompositionView<'a> {
    pub swapchain: &'a Swapchain,
    pub image_rect: openxr_sys::Rect2Di,
    pub image_array_index: usize,
}

pub struct Compositor {
    context: Context,
    composer: ComposingPipeline,
    color_corrector: Option<ColorCorrectionPipeline>,
    foveation_encoder: Option<FoveatedRenderingPipeline>,
    slicer: SlicingPipeline,
    aligner: AlignmentPipeline,

    // todo: move to client
    aligner2: AlignmentPipeline,
    slicer2: SlicingPipeline,
    foveation_decoder: Option<FoveatedRenderingPipeline>,

    output_textures: Mutex<Vec<Texture>>,
    output_size: (u32, u32),
}

impl Compositor {
    pub fn new(
        context: Context,
        target_eye_size: (u32, u32), // expected size of a layer after cropping
        foveation_desc: Option<&FoveatedRenderingDesc>,
        color_correction_desc: Option<&ColorCorrectionDesc>,
        slices_count: usize,
    ) -> Self {
        // todo: calculate final size after each transformation
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

        let (slicer2, _) = SlicingPipeline::new(output_size, slices_count, 1);

        let output_size = align_to_32(sliced_size);

        let aligner = AlignmentPipeline::new(sliced_size, output_size);

        let aligner2 = AlignmentPipeline::new(output_size, sliced_size);

        Self {
            context,
            composer: ComposingPipeline::new(),
            color_corrector: color_correction_desc.map(ColorCorrectionPipeline::new),
            foveation_encoder,
            slicer,
            aligner,
            aligner2,
            slicer2,
            foveation_decoder,
            // wgpu does not support planar texture formats. Software encoding cannot be supported
            output_textures: todo!(),
            output_size,
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    // image size used for encoding
    pub fn output_size(&self) -> (u32, u32) {
        self.output_size
    }

    // The function blocks the access to all textures but it should finish quite fast.
    // Output textures are ready to be encoded
    pub fn end_frame(&self, layers: &[&[CompositionView]]) -> MutexGuard<Vec<Texture>> {
        for layer in layers {
            for view in *layer {
                view.swapchain.last_presented_index.store(
                    (view.swapchain.last_presented_index.load(Ordering::SeqCst) + 1)
                        % view.swapchain.textures.len(),
                    Ordering::SeqCst,
                );
            }
        }

        todo!()
    }
}
