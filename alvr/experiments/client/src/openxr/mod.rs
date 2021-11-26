mod graphics_interop;
mod interaction;

pub use graphics_interop::*;
use xr::{Fovf, Posef, Quaternionf, Vector3f};

use self::interaction::OpenxrInteractionContext;
use alvr_common::{
    glam::{Quat, UVec2, Vec3},
    prelude::*,
    Fov,
};
use alvr_graphics::GraphicsContext;
use ash::vk::Handle;
use openxr as xr;
use parking_lot::{Mutex, MutexGuard};
use std::{sync::Arc, time::Duration};
use wgpu::TextureView;

pub struct OpenxrContext {
    pub instance: xr::Instance,
    pub system: xr::SystemId,
    pub environment_blend_modes: Vec<xr::EnvironmentBlendMode>,
}

impl OpenxrContext {
    pub fn new() -> Self {
        let entry = xr::Entry::load().unwrap();

        #[cfg(target_os = "android")]
        entry.initialize_android_loader().unwrap();

        let available_extensions = entry.enumerate_extensions().unwrap();

        let mut enabled_extensions = xr::ExtensionSet::default();
        enabled_extensions.khr_vulkan_enable2 = true;
        #[cfg(target_os = "android")]
        {
            enabled_extensions.khr_android_create_instance = true;
        }
        let instance = entry
            .create_instance(
                &xr::ApplicationInfo {
                    application_name: "ALVR client",
                    application_version: 0,
                    engine_name: "ALVR",
                    engine_version: 0,
                },
                &enabled_extensions,
                &[],
            )
            .unwrap();

        let system = instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .unwrap();

        let environment_blend_modes = instance
            .enumerate_environment_blend_modes(system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .unwrap();

        // Call required by spec
        // todo: actually check requirements
        let _reqs = instance
            .graphics_requirements::<openxr::Vulkan>(system)
            .unwrap();

        Self {
            instance,
            system,
            environment_blend_modes,
        }
    }
}

pub struct OpenxrSwapchain {
    handle: Arc<Mutex<xr::Swapchain<xr::Vulkan>>>,
    views: Vec<Arc<TextureView>>,
    size: UVec2,
}

pub struct AcquiredOpenxrSwapchain<'a> {
    handle_lock: MutexGuard<'a, xr::Swapchain<xr::Vulkan>>,
    pub size: UVec2,
    pub texture_view: Arc<TextureView>,
}

pub struct OpenxrSessionLock<'a> {
    pub acquired_scene_swapchain: Vec<AcquiredOpenxrSwapchain<'a>>,
    pub acquired_stream_swapchain: Vec<AcquiredOpenxrSwapchain<'a>>,
    pub frame_state: xr::FrameState,
}

pub struct PresentationView<'a> {
    pub acquired_swapchain: AcquiredOpenxrSwapchain<'a>,
    pub orientation: Quat,
    pub position: Vec3,
    pub fov: Fov,
}

pub struct OpenxrSession {
    pub xr_context: Arc<OpenxrContext>,
    pub graphics_context: Arc<GraphicsContext>,
    pub inner: xr::Session<xr::Vulkan>,
    pub frame_stream: Mutex<xr::FrameStream<xr::Vulkan>>,
    pub frame_waiter: Mutex<xr::FrameWaiter>,
    scene_swapchains: Vec<OpenxrSwapchain>,
    stream_swapchains: Vec<OpenxrSwapchain>,
    pub environment_blend_mode: xr::EnvironmentBlendMode,
    pub interaction_context: OpenxrInteractionContext,
}

impl OpenxrSession {
    pub fn new(
        xr_context: Arc<OpenxrContext>,
        graphics_context: Arc<GraphicsContext>,
    ) -> StrResult<Self> {
        let (session, frame_waiter, frame_stream) = unsafe {
            trace_err!(xr_context.instance.create_session_with_guard::<xr::Vulkan>(
                xr_context.system,
                &xr::vulkan::SessionCreateInfo {
                    instance: graphics_context.raw_instance.handle().as_raw() as _,
                    physical_device: graphics_context.raw_physical_device.as_raw() as _,
                    device: graphics_context.raw_device.handle().as_raw() as _,
                    queue_family_index: graphics_context.queue_family_index,
                    queue_index: graphics_context.queue_index,
                },
                Box::new(Arc::clone(&graphics_context.device)),
            ))?
        };

        let views = xr_context
            .instance
            .enumerate_view_configuration_views(
                xr_context.system,
                xr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .unwrap();

        let scene_swapchains = views
            .into_iter()
            .map(|config| {
                create_swapchain(
                    &graphics_context.device,
                    &session,
                    UVec2::new(
                        config.recommended_image_rect_width,
                        config.recommended_image_rect_height,
                    ),
                )
            })
            .collect();

        // Recreated later
        let stream_swapchains = (0..2)
            .map(|_| create_swapchain(&graphics_context.device, &session, UVec2::new(1, 1)))
            .collect();

        let environment_blend_mode = *xr_context.environment_blend_modes.first().unwrap();

        Ok(Self {
            xr_context,
            graphics_context,
            inner: session,
            frame_stream: Mutex::new(frame_stream),
            frame_waiter: Mutex::new(frame_waiter),
            scene_swapchains,
            stream_swapchains,
            environment_blend_mode,
            interaction_context: todo!(),
        })
    }

    pub fn recreate_stream_swapchains(&mut self, view_size: UVec2) {
        self.stream_swapchains = (0..2)
            .map(|_| create_swapchain(&self.graphics_context.device, &self.inner, view_size))
            .collect();
    }

    fn acquire_views(swapchains: &[OpenxrSwapchain]) -> Vec<AcquiredOpenxrSwapchain> {
        swapchains
            .iter()
            .map(|swapchain| {
                let mut handle_lock = swapchain.handle.lock();

                let index = handle_lock.acquire_image().unwrap();
                handle_lock.wait_image(xr::Duration::INFINITE).unwrap();

                AcquiredOpenxrSwapchain {
                    handle_lock,
                    size: swapchain.size,
                    texture_view: Arc::clone(&swapchain.views[index as usize]),
                }
            })
            .collect()
    }

    // fixme: release swapchains if not consumed by end_frame()
    pub fn begin_frame(&self) -> StrResult<Option<OpenxrSessionLock>> {
        let frame_state = trace_err!(self.frame_waiter.lock().wait())?;

        trace_err!(self.frame_stream.lock().begin())?;

        if !frame_state.should_render {
            trace_err!(self.frame_stream.lock().end(
                frame_state.predicted_display_time,
                self.environment_blend_mode,
                &[],
            ))?;

            return Ok(None);
        }

        let acquired_scene_swapchain = Self::acquire_views(&self.scene_swapchains);
        let acquired_stream_swapchain = Self::acquire_views(&self.stream_swapchains);

        Ok(Some(OpenxrSessionLock {
            acquired_scene_swapchain,
            acquired_stream_swapchain,
            frame_state,
        }))
    }

    fn create_layer_views<'a>(
        views: &'a mut [PresentationView],
    ) -> Vec<xr::CompositionLayerProjectionView<'a, xr::Vulkan>> {
        views
            .iter_mut()
            .map(|view| {
                view.acquired_swapchain.handle_lock.release_image().unwrap();

                let rect = xr::Rect2Di {
                    offset: xr::Offset2Di { x: 0, y: 0 },
                    extent: xr::Extent2Di {
                        width: view.acquired_swapchain.size.x as _,
                        height: view.acquired_swapchain.size.y as _,
                    },
                };

                xr::CompositionLayerProjectionView::new()
                    .pose(Posef {
                        orientation: Quaternionf {
                            x: view.orientation.x,
                            y: view.orientation.y,
                            z: view.orientation.z,
                            w: view.orientation.w,
                        },
                        position: Vector3f {
                            x: view.position.x,
                            y: view.position.y,
                            z: view.position.z,
                        },
                    })
                    .fov(Fovf {
                        angle_left: view.fov.left,
                        angle_right: view.fov.right,
                        angle_up: view.fov.top,
                        angle_down: view.fov.bottom,
                    })
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&view.acquired_swapchain.handle_lock)
                            .image_array_index(0)
                            .image_rect(rect),
                    )
            })
            .collect()
    }

    pub fn end_frame(
        &self,
        display_timestamp: Duration,
        mut stream_views: Vec<PresentationView>,
        mut scene_views: Vec<PresentationView>,
    ) -> StrResult {
        //Note: scene layers are drawn always on top of the stream layers
        trace_err!(self.frame_stream.lock().end(
            xr::Time::from_nanos(display_timestamp.as_nanos() as _),
            self.environment_blend_mode,
            &[
                &xr::CompositionLayerProjection::new()
                    .space(&self.interaction_context.reference_space)
                    .views(&Self::create_layer_views(&mut stream_views)),
                &xr::CompositionLayerProjection::new()
                    .space(&self.interaction_context.reference_space)
                    .views(&Self::create_layer_views(&mut scene_views)),
            ],
        ))
    }
}
