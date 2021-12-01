mod convert;
mod graphics_interop;
mod interaction;

pub use graphics_interop::create_graphics_context;

use self::interaction::{
    OpenxrActionType, OpenxrActionValue, OpenxrInteractionContext, OpenxrProfileDesc,
};
use crate::ViewConfig;
use alvr_common::{
    glam::{Quat, UVec2, Vec3},
    log,
    prelude::*,
    Fov, MotionData,
};
use alvr_graphics::GraphicsContext;
use alvr_session::TrackingSpace;
use ash::vk::Handle;
use openxr as xr;
use parking_lot::{Mutex, MutexGuard};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
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
        enabled_extensions.ext_hand_tracking = true;
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
    pub texture_view: Arc<TextureView>,
    pub size: UVec2,
}

fn create_layer_views<'a>(
    acquired_swapchains: &'a mut [AcquiredOpenxrSwapchain],
    view_configs: &'a [ViewConfig],
) -> Vec<xr::CompositionLayerProjectionView<'a, xr::Vulkan>> {
    acquired_swapchains
        .iter_mut()
        .enumerate()
        .map(|(index, swapchain)| {
            let view_config = view_configs
                .get(index)
                .cloned()
                .unwrap_or_else(|| ViewConfig {
                    orientation: Quat::IDENTITY,
                    position: Vec3::ZERO,
                    fov: Fov::default(),
                });

            swapchain.handle_lock.release_image().unwrap();

            let rect = xr::Rect2Di {
                offset: xr::Offset2Di { x: 0, y: 0 },
                extent: xr::Extent2Di {
                    width: swapchain.size.x as _,
                    height: swapchain.size.y as _,
                },
            };

            xr::CompositionLayerProjectionView::new()
                .pose(xr::Posef {
                    orientation: convert::to_xr_orientation(view_config.orientation),
                    position: convert::to_xr_vec3(view_config.position),
                })
                .fov(convert::to_xr_fov(view_config.fov))
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&swapchain.handle_lock)
                        .image_array_index(0)
                        .image_rect(rect),
                )
        })
        .collect()
}

// End frame and submit swapchains once dropped
pub struct OpenxrPresentationGuard<'a> {
    frame_stream_lock: MutexGuard<'a, xr::FrameStream<xr::Vulkan>>,
    interaction_context: &'a OpenxrInteractionContext,
    environment_blend_mode: xr::EnvironmentBlendMode,
    pub acquired_scene_swapchains: Vec<AcquiredOpenxrSwapchain<'a>>,
    pub acquired_stream_swapchains: Vec<AcquiredOpenxrSwapchain<'a>>,
    pub predicted_frame_interval: Duration,
    pub display_timestamp: Duration,          // output/input
    pub scene_view_configs: Vec<ViewConfig>,  // input
    pub stream_view_configs: Vec<ViewConfig>, // input
}

impl<'a> Drop for OpenxrPresentationGuard<'a> {
    fn drop(&mut self) {
        let reference_space = &self.interaction_context.reference_space;

        //Note: scene layers are drawn always on top of the stream layers
        self.frame_stream_lock
            .end(
                xr::Time::from_nanos(self.display_timestamp.as_nanos() as _),
                self.environment_blend_mode,
                &[
                    &xr::CompositionLayerProjection::new()
                        .space(reference_space)
                        .views(&create_layer_views(
                            &mut self.acquired_scene_swapchains,
                            &self.scene_view_configs,
                        )),
                    &xr::CompositionLayerProjection::new()
                        .space(reference_space)
                        .views(&create_layer_views(
                            &mut self.acquired_stream_swapchains,
                            &self.stream_view_configs,
                        )),
                ],
            )
            .ok();
        // Note: in case of error, the next usage of the session will error, triggering a recreation
        // of the session
    }
}

pub struct HandTrackingInput {
    pub target_ray_motion: MotionData,
    pub skeleton_motion: Vec<MotionData>,
}

pub struct OpenxrHandPoseInput {
    pub grip_motion: MotionData,
    pub hand_tracking_input: Option<HandTrackingInput>,
}

pub struct SceneButtons {
    pub select: bool,
    pub menu: bool,
}

pub struct OpenxrSceneInput {
    pub view_configs: Vec<ViewConfig>,
    pub left_pose_input: OpenxrHandPoseInput,
    pub right_pose_input: OpenxrHandPoseInput,
    pub buttons: SceneButtons,
    pub is_focused: bool,
}

pub struct OpenxrStreamingInput {
    pub view_configs: Vec<ViewConfig>,
    pub left_pose_input: OpenxrHandPoseInput,
    pub right_pose_input: OpenxrHandPoseInput,
    pub button_values: HashMap<String, OpenxrActionValue>,
}

pub struct OpenxrSession {
    xr_context: Arc<OpenxrContext>,
    graphics_context: Arc<GraphicsContext>,
    inner: xr::Session<xr::Vulkan>,
    scene_swapchains: Vec<OpenxrSwapchain>,
    stream_swapchains: Vec<OpenxrSwapchain>,
    environment_blend_mode: xr::EnvironmentBlendMode,
    interaction_context: OpenxrInteractionContext,
    running_state: AtomicBool,
    focused_state: AtomicBool,
    frame_stream: Mutex<xr::FrameStream<xr::Vulkan>>,
    frame_waiter: Mutex<xr::FrameWaiter>,
    scene_predicted_display_timestamp: Mutex<xr::Time>,
}

pub enum OpenxrEvent<'a> {
    ShouldRender(OpenxrPresentationGuard<'a>),
    Idle,
    Shutdown,
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
                graphics_interop::create_swapchain(
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
            .map(|_| {
                graphics_interop::create_swapchain(
                    &graphics_context.device,
                    &session,
                    UVec2::new(1, 1),
                )
            })
            .collect();

        let environment_blend_mode = *xr_context.environment_blend_modes.first().unwrap();

        let interaction_context = OpenxrInteractionContext::new(
            &xr_context,
            session.clone(),
            &[],
            vec![],
            TrackingSpace::Local,
        )?;

        Ok(Self {
            xr_context,
            graphics_context,
            inner: session,
            scene_swapchains,
            stream_swapchains,
            environment_blend_mode,
            interaction_context,
            running_state: AtomicBool::new(false),
            focused_state: AtomicBool::new(false),
            frame_stream: Mutex::new(frame_stream),
            frame_waiter: Mutex::new(frame_waiter),
            scene_predicted_display_timestamp: Mutex::new(xr::Time::from_nanos(0)),
        })
    }

    pub fn update_for_stream(
        &mut self,
        view_size: UVec2,
        action_types: &[(String, OpenxrActionType)],
        profile_descs: Vec<OpenxrProfileDesc>,
        reference_space_type: TrackingSpace,
        environment_blend_mode: xr::EnvironmentBlendMode,
    ) -> StrResult {
        // Note: if called between begin_frame() and end_frame(), the old swapchains will live until
        // presented, then they will get dropped. It can't happen to present an unacquired swapchain.
        self.stream_swapchains = (0..2)
            .map(|_| {
                graphics_interop::create_swapchain(
                    &self.graphics_context.device,
                    &self.inner,
                    view_size,
                )
            })
            .collect();

        self.interaction_context = OpenxrInteractionContext::new(
            &self.xr_context,
            self.inner.clone(),
            action_types,
            profile_descs,
            reference_space_type,
        )?;

        self.environment_blend_mode = environment_blend_mode;

        Ok(())
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

    pub fn begin_frame(&self) -> StrResult<OpenxrEvent> {
        let mut event_storage = xr::EventDataBuffer::new();
        while let Some(event) = self
            .xr_context
            .instance
            .poll_event(&mut event_storage)
            .unwrap()
        {
            match event {
                xr::Event::EventsLost(event) => {
                    return fmt_e!("Lost {} events", event.lost_event_count())
                }
                xr::Event::InstanceLossPending(_) => return Ok(OpenxrEvent::Shutdown),
                xr::Event::SessionStateChanged(event) => {
                    log::error!("Enter OpenXR session state: {:?}", event.state());

                    match event.state() {
                        xr::SessionState::UNKNOWN | xr::SessionState::IDLE => (),
                        xr::SessionState::READY => {
                            trace_err!(self
                                .inner
                                .begin(xr::ViewConfigurationType::PRIMARY_STEREO))?;
                            self.running_state.store(true, Ordering::Relaxed);
                        }
                        xr::SessionState::SYNCHRONIZED => (),
                        xr::SessionState::VISIBLE => {
                            self.focused_state.store(false, Ordering::Relaxed)
                        }
                        xr::SessionState::FOCUSED => {
                            self.focused_state.store(true, Ordering::Relaxed)
                        }
                        xr::SessionState::STOPPING => {
                            self.running_state.store(false, Ordering::Relaxed);
                            trace_err!(self.inner.end())?;
                            
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            return Ok(OpenxrEvent::Shutdown)
                        }
                        _ => unreachable!(),
                    }
                }
                xr::Event::ReferenceSpaceChangePending(_) => (), // todo
                xr::Event::PerfSettingsEXT(_) => (),             // todo
                xr::Event::VisibilityMaskChangedKHR(_) => (),    // todo
                xr::Event::InteractionProfileChanged(_) => (),   // todo
                xr::Event::MainSessionVisibilityChangedEXTX(_) => (), // todo
                xr::Event::DisplayRefreshRateChangedFB(_) => (), // todo
                _ => log::debug!("OpenXR: Unknown event"),
            }
        }

        if !self.running_state.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(5));
            return Ok(OpenxrEvent::Idle);
        }

        // This is the blocking call that performs Phase Sync
        let frame_state = trace_err!(self.frame_waiter.lock().wait())?;

        let mut frame_stream_lock = self.frame_stream.lock();

        trace_err!(frame_stream_lock.begin())?;

        if !frame_state.should_render {
            trace_err!(frame_stream_lock.end(
                frame_state.predicted_display_time,
                self.environment_blend_mode,
                &[],
            ))?;

            return Ok(OpenxrEvent::Idle);
        }

        let acquired_scene_swapchains = Self::acquire_views(&self.scene_swapchains);
        let acquired_stream_swapchains = Self::acquire_views(&self.stream_swapchains);

        *self.scene_predicted_display_timestamp.lock() = frame_state.predicted_display_time;
        let display_timestamp =
            Duration::from_nanos(frame_state.predicted_display_time.as_nanos() as _);

        Ok(OpenxrEvent::ShouldRender(OpenxrPresentationGuard {
            frame_stream_lock,
            interaction_context: &self.interaction_context,
            environment_blend_mode: self.environment_blend_mode,
            acquired_scene_swapchains,
            acquired_stream_swapchains,
            predicted_frame_interval: Duration::from_nanos(
                frame_state.predicted_display_period.as_nanos() as _,
            ),
            display_timestamp,
            scene_view_configs: vec![],
            stream_view_configs: vec![],
        }))
    }

    pub fn get_scene_input(&self) -> StrResult<OpenxrSceneInput> {
        let display_time = *self.scene_predicted_display_timestamp.lock();
        let ctx = &self.interaction_context;

        ctx.sync_input()?;

        Ok(OpenxrSceneInput {
            view_configs: ctx.get_views(xr::ViewConfigurationType::PRIMARY_STEREO, display_time)?,
            left_pose_input: ctx.get_poses(&ctx.left_hand_interaction, display_time)?,
            right_pose_input: ctx.get_poses(&ctx.left_hand_interaction, display_time)?,
            buttons: ctx.get_scene_buttons()?,
            is_focused: self.focused_state.load(Ordering::Relaxed),
        })
    }

    pub fn get_streaming_input(
        &self,
        display_timestamp: Duration,
    ) -> StrResult<OpenxrStreamingInput> {
        let display_time = xr::Time::from_nanos(display_timestamp.as_nanos() as _);
        let ctx = &self.interaction_context;

        ctx.sync_input()?;

        Ok(OpenxrStreamingInput {
            view_configs: ctx.get_views(xr::ViewConfigurationType::PRIMARY_STEREO, display_time)?,
            left_pose_input: ctx.get_poses(&ctx.left_hand_interaction, display_time)?,
            right_pose_input: ctx.get_poses(&ctx.left_hand_interaction, display_time)?,
            button_values: ctx.get_streming_buttons()?,
        })
    }
}
