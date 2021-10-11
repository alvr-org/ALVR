mod color_correction;
mod compositing;

use alvr_common::{prelude::*, Fov};
use alvr_graphics::{
    convert::{self, SwapchainCreateData, SwapchainCreateInfo, TextureType},
    foveated_rendering::{FoveatedRenderingPass, FrDirection},
    slicing::{AlignmentDirection, SlicingPass},
    Context, TARGET_FORMAT,
};
use alvr_session::{ColorCorrectionDesc, FoveatedRenderingDesc};
use color_correction::ColorCorrectionPass;
use compositing::{CompositingPass, Layer};
use std::sync::Arc;
use wgpu::{
    BindGroup, CommandEncoderDescriptor, Extent3d, Texture, TextureDescriptor, TextureDimension,
    TextureUsages, TextureView,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub struct Swapchain {
    textures: Vec<Texture>,
    bind_groups: Vec<Vec<BindGroup>>, //[0]: texture index, [1]: array index
    current_index: usize,
}

impl Swapchain {
    pub fn enumerate_images(&self) -> &[Texture] {
        &self.textures
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
// FFR and changing the target view size require recreating the compositor completely, which might
// cause a lag spike.
pub struct Compositor {
    context: Arc<Context>,

    inner: CompositingPass,
    color_corrector: ColorCorrectionPass,
    foveation_encoder: Option<FoveatedRenderingPass>,
    slicer: SlicingPass,

    output_textures: Vec<Texture>,
    output_texture_views: Vec<TextureView>,
    output_size: (u32, u32),
}

impl Compositor {
    pub fn new(
        context: Arc<Context>,
        target_view_size: (u32, u32), // expected size of a layer after cropping
        foveation_desc: Option<&FoveatedRenderingDesc>,
        slices_count: usize,
    ) -> Self {
        let inner = CompositingPass::new(context.device());

        let color_corrector = ColorCorrectionPass::new(context.device(), target_view_size);

        let mut output_size = target_view_size;

        let foveation_encoder = foveation_desc
            .map(|desc| {
                FoveatedRenderingPass::new(
                    FrDirection::Encoding,
                    target_view_size,
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

        let combined_size = (output_size.0 * 2, output_size.1);

        let slicer = SlicingPass::new(
            context.device(),
            combined_size,
            2,
            slices_count,
            AlignmentDirection::Output,
        );

        let output_size = slicer.output_size();

        let output_textures = (0..slices_count)
            .map(|_| {
                context.device().create_texture(&TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: output_size.0,
                        height: output_size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TARGET_FORMAT,
                    usage: TextureUsages::RENDER_ATTACHMENT,
                })
            })
            .collect::<Vec<_>>();
        let output_texture_views = output_textures
            .iter()
            .map(|tex| tex.create_view(&Default::default()))
            .collect();

        Self {
            context,
            inner,
            color_corrector,
            foveation_encoder,
            slicer,
            output_textures,
            output_texture_views,
            output_size,
        }
    }

    // corresponds to xrCreateSwapchain
    pub fn create_swapchain(
        &self,
        data: SwapchainCreateData,
        info: SwapchainCreateInfo,
    ) -> Swapchain {
        let array_size = match info.texture_type {
            TextureType::Flat { array_size } => array_size,
            TextureType::Cubemap => 1,
        };

        let textures = convert::create_texture_set(self.context.device(), data, info);

        let bind_groups = textures
            .iter()
            .map(|texture| {
                (0..array_size)
                    .map(|array_index| {
                        self.inner
                            .create_bind_group(self.context.device(), texture, array_index)
                    })
                    .collect()
            })
            .collect();

        Swapchain {
            textures,
            bind_groups,
            current_index: 0,
        }
    }

    // image size used for encoding
    pub fn output_size(&self) -> (u32, u32) {
        self.output_size
    }

    pub fn output(&self) -> &[Texture] {
        &self.output_textures
    }

    // The function is blocking but it should finish quite fast. Corresponds to xrEndFrame
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
            .device()
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

            let render_target = if color_correction.is_some() {
                self.color_corrector.input()
            } else if let Some(encoder) = &self.foveation_encoder {
                encoder.input()
            } else {
                &self.slicer.input_views()[view_index]
            };

            self.inner.draw(&mut encoder, layers, render_target);

            if let Some(desc) = color_correction {
                let render_target = if let Some(encoder) = &self.foveation_encoder {
                    encoder.input()
                } else {
                    &self.slicer.input_views()[view_index]
                };

                self.color_corrector
                    .draw(&mut encoder, &desc, render_target)
            }

            if let Some(foveation_encoder) = &self.foveation_encoder {
                // todo: get correct fov
                let fov = Fov::default();
                foveation_encoder.draw(&mut encoder, fov);
            }
        }

        for slice_idx in 0..self.output_texture_views.len() {
            self.slicer.draw(
                &mut encoder,
                slice_idx,
                &self.output_texture_views[slice_idx],
            )
        }

        // For the best performance, all compositing work is submitted at once.
        self.context.queue().submit(Some(encoder.finish()));

        pollster::block_on(self.context.queue().on_submitted_work_done());
    }
}

fn run() -> StrResult {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let context = Arc::new(Context::new(None)?);

    let compositor = Compositor::new(context.clone(), (400, 300), None, 1);

    compositor.end_frame(&[], None);

    let surface = unsafe { context.instance().create_surface(&window) };

    event_loop.run(move |event, _, control| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control = ControlFlow::Exit,
        Event::WindowEvent { .. } => (),
        _ => (),
    })
}

fn main() {
    env_logger::init();

    show_err(run());
}
