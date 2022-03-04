mod convert;
mod graphics_interop;
mod interaction;

use alvr_session::Fov;
use alvr_sockets::MotionData;
pub use graphics_interop::create_graphics_context;
use xr::HAND_JOINT_COUNT;

use self::interaction::XrInteractionContext;
use super::{XrActionType, XrActionValue, XrProfileDesc};
use alvr_common::{
    glam::{Quat, UVec2, Vec3},
    log,
    prelude::*,
    HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_graphics::{ash::vk::Handle, wgpu::TextureView, GraphicsContext};
use openxr as xr;
use parking_lot::{Mutex, MutexGuard};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub struct XrContext {
    pub instance: xr::Instance,
    pub system: xr::SystemId,
    pub environment_blend_modes: Vec<xr::EnvironmentBlendMode>,
}

impl XrContext {
    pub fn new() -> Self {
        let entry = if cfg!(feature = "oculus") {
            xr::Entry::load_from(Path::new("libopenxr_loader_oculus.so")).unwrap()
        } else {
            xr::Entry::load().unwrap()
        };

        #[cfg(target_os = "android")]
        entry.initialize_android_loader().unwrap();

        let available_extensions = entry.enumerate_extensions().unwrap();
        error!("{available_extensions:#?}");

        let mut enabled_extensions = xr::ExtensionSet::default();
        // Mandatory
        enabled_extensions.khr_vulkan_enable2 = true;
        #[cfg(target_os = "android")]
        {
            enabled_extensions.khr_android_create_instance = true;
        }
        // Optional
        enabled_extensions.ext_hand_tracking = available_extensions.ext_hand_tracking;
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

#[derive(Clone)]
pub struct XrViewConfig {
    pub orientation: Quat,
    pub position: Vec3,
    pub fov: Fov,
}

pub struct XrSwapchain {
    handle: Arc<Mutex<xr::Swapchain<xr::Vulkan>>>,
    views: Vec<Arc<TextureView>>,
    size: UVec2,
}

pub struct AcquiredXrSwapchain<'a> {
    handle_lock: MutexGuard<'a, xr::Swapchain<xr::Vulkan>>,
    pub texture_view: Arc<TextureView>,
    pub size: UVec2,
}

fn create_layer_views<'a>(
    acquired_swapchains: &'a mut [AcquiredXrSwapchain],
    view_configs: &'a [XrViewConfig],
) -> Vec<xr::CompositionLayerProjectionView<'a, xr::Vulkan>> {
    acquired_swapchains
        .iter_mut()
        .enumerate()
        .map(|(index, swapchain)| {
            let view_config = view_configs
                .get(index)
                .cloned()
                .unwrap_or_else(|| XrViewConfig {
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
pub struct XrPresentationGuard<'a> {
    frame_stream_lock: MutexGuard<'a, xr::FrameStream<xr::Vulkan>>,
    interaction_context: &'a XrInteractionContext,
    environment_blend_mode: xr::EnvironmentBlendMode,
    pub acquired_scene_swapchains: Vec<AcquiredXrSwapchain<'a>>,
    pub acquired_stream_swapchains: Vec<AcquiredXrSwapchain<'a>>,
    pub predicted_frame_interval: Duration,
    pub display_timestamp: Duration,            // output/input
    pub scene_view_configs: Vec<XrViewConfig>,  // input
    pub stream_view_configs: Vec<XrViewConfig>, // input
}

impl<'a> Drop for XrPresentationGuard<'a> {
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
                            &mut self.acquired_stream_swapchains,
                            &self.stream_view_configs,
                        )),
                    &xr::CompositionLayerProjection::new()
                        .space(reference_space)
                        .views(&create_layer_views(
                            &mut self.acquired_scene_swapchains,
                            &self.scene_view_configs,
                        )),
                ],
            )
            .ok();
        // Note: in case of error, the next usage of the session will error, triggering a recreation
        // of the session
    }
}

pub struct SceneButtons {
    pub select: bool,
    pub menu: bool,
}

pub struct XrSceneInput {
    pub view_configs: Vec<XrViewConfig>,
    pub left_hand_motion: MotionData,
    pub right_hand_motion: MotionData,
    pub buttons: SceneButtons,
    pub is_focused: bool,
}

pub struct XrStreamingInput {
    pub device_motions: Vec<(u64, MotionData)>,
    pub left_hand_tracking_input: Option<[MotionData; HAND_JOINT_COUNT]>,
    pub right_hand_tracking_input: Option<[MotionData; HAND_JOINT_COUNT]>,
    pub button_values: Vec<(u64, XrActionValue)>,
}

pub struct XrSession {
    xr_context: Arc<XrContext>,
    inner: xr::Session<xr::Vulkan>,
    recommended_view_sizes: Vec<UVec2>,
    scene_swapchains: Vec<XrSwapchain>,
    stream_swapchains: Vec<XrSwapchain>,
    environment_blend_mode: xr::EnvironmentBlendMode,
    interaction_context: XrInteractionContext,
    running_state: AtomicBool,
    focused_state: AtomicBool,
    frame_stream: Mutex<xr::FrameStream<xr::Vulkan>>,
    frame_waiter: Mutex<xr::FrameWaiter>,
    scene_predicted_display_timestamp: Mutex<xr::Time>,
}

pub enum XrEvent<'a> {
    ShouldRender(XrPresentationGuard<'a>),
    Idle,
    Shutdown,
}

impl XrSession {
    pub fn new(
        xr_context: Arc<XrContext>,
        graphics_context: Arc<GraphicsContext>,
        stream_views_size: UVec2,
        stream_action_types: &[(String, XrActionType)],
        stream_profile_descs: Vec<XrProfileDesc>,
        environment_blend_mode: xr::EnvironmentBlendMode,
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

        let view_configs = xr_context
            .instance
            .enumerate_view_configuration_views(
                xr_context.system,
                xr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .unwrap();

        let recommended_view_sizes = view_configs
            .into_iter()
            .map(|config| {
                UVec2::new(
                    config.recommended_image_rect_width,
                    config.recommended_image_rect_height,
                )
            })
            .collect::<Vec<_>>();

        let scene_swapchains = recommended_view_sizes
            .iter()
            .cloned()
            .map(|size| {
                graphics_interop::create_swapchain(&graphics_context.device, &session, size)
            })
            .collect();

        let stream_swapchains = (0..2)
            .map(|_| {
                graphics_interop::create_swapchain(
                    &graphics_context.device,
                    &session,
                    stream_views_size,
                )
            })
            .collect();

        let interaction_context = XrInteractionContext::new(
            &xr_context,
            session.clone(),
            stream_action_types,
            stream_profile_descs,
        )?;

        Ok(Self {
            xr_context,
            inner: session,
            recommended_view_sizes,
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

    pub fn recommended_view_sizes(&self) -> &[UVec2] {
        &self.recommended_view_sizes
    }

    fn acquire_views(swapchains: &[XrSwapchain]) -> Vec<AcquiredXrSwapchain> {
        swapchains
            .iter()
            .map(|swapchain| {
                let mut handle_lock = swapchain.handle.lock();

                let index = handle_lock.acquire_image().unwrap();
                handle_lock.wait_image(xr::Duration::INFINITE).unwrap();

                AcquiredXrSwapchain {
                    handle_lock,
                    size: swapchain.size,
                    texture_view: Arc::clone(&swapchain.views[index as usize]),
                }
            })
            .collect()
    }

    pub fn begin_frame(&self) -> StrResult<XrEvent> {
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
                xr::Event::InstanceLossPending(_) => return Ok(XrEvent::Shutdown),
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
                            return Ok(XrEvent::Shutdown)
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
            return Ok(XrEvent::Idle);
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

            return Ok(XrEvent::Idle);
        }

        let acquired_scene_swapchains = Self::acquire_views(&self.scene_swapchains);
        let acquired_stream_swapchains = Self::acquire_views(&self.stream_swapchains);

        *self.scene_predicted_display_timestamp.lock() = frame_state.predicted_display_time;
        let display_timestamp =
            Duration::from_nanos(frame_state.predicted_display_time.as_nanos() as _);

        Ok(XrEvent::ShouldRender(XrPresentationGuard {
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

    pub fn get_scene_input(&self) -> StrResult<XrSceneInput> {
        let display_time = *self.scene_predicted_display_timestamp.lock();
        let ctx = &self.interaction_context;

        ctx.sync_input()?;

        Ok(XrSceneInput {
            view_configs: ctx.get_views(xr::ViewConfigurationType::PRIMARY_STEREO, display_time)?,
            left_hand_motion: ctx.get_tracker_pose(&ctx.left_hand_tracker_context, display_time)?,
            right_hand_motion: ctx
                .get_tracker_pose(&ctx.right_hand_tracker_context, display_time)?,
            buttons: ctx.get_scene_buttons()?,
            is_focused: self.focused_state.load(Ordering::Relaxed),
        })
    }

    pub fn get_streaming_input(&self, display_timestamp: Duration) -> StrResult<XrStreamingInput> {
        let display_time = xr::Time::from_nanos(display_timestamp.as_nanos() as _);
        let ctx = &self.interaction_context;

        ctx.sync_input()?;

        let views = ctx.get_views(xr::ViewConfigurationType::PRIMARY_STEREO, display_time)?;

        // this assumes 2 views and on the same plane. This is not the case with Pimax. todo: fix
        assert_eq!(views.len(), 2);
        let head_pose = MotionData {
            orientation: views[0].orientation,
            position: (views[0].position + views[1].position) / 2.0,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        };

        let left_hand_pose = ctx.get_tracker_pose(&ctx.left_hand_tracker_context, display_time)?;
        let right_hand_pose =
            ctx.get_tracker_pose(&ctx.right_hand_tracker_context, display_time)?;

        let device_motions = [
            (*HEAD_ID, head_pose),
            (*LEFT_HAND_ID, left_hand_pose),
            (*RIGHT_HAND_ID, right_hand_pose),
        ]
        .into_iter()
        .collect();

        let left_hand_tracking_input =
            if let Some(tracking_context) = &ctx.left_hand_skeleton_tracker {
                ctx.get_hand_skeleton(tracking_context, display_time)?
            } else {
                None
            };

        let right_hand_tracking_input =
            if let Some(tracking_context) = &ctx.right_hand_skeleton_tracker {
                ctx.get_hand_skeleton(tracking_context, display_time)?
            } else {
                None
            };

        Ok(XrStreamingInput {
            device_motions,
            left_hand_tracking_input,
            right_hand_tracking_input,
            button_values: ctx.get_streming_buttons()?,
        })
    }
}
