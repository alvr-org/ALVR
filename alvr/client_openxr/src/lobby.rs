use crate::{
    graphics::{self, ProjectionLayerAlphaConfig, ProjectionLayerBuilder},
    interaction::{self, InteractionContext},
};
use alvr_common::{glam::UVec2, parking_lot::RwLock, Pose};
use alvr_graphics::{
    BodyTrackingType, GraphicsContext, LobbyRenderer, LobbyViewParams, SDR_FORMAT_GL,
};
use alvr_system_info::Platform;
use openxr as xr;
use std::{rc::Rc, sync::Arc, time::Duration};

// todo: add interaction?
pub struct Lobby {
    xr_session: xr::Session<xr::OpenGlEs>,
    interaction_ctx: Arc<RwLock<InteractionContext>>,
    platform: Platform,
    reference_space: xr::Space,
    swapchains: [xr::Swapchain<xr::OpenGlEs>; 2],
    view_resolution: UVec2,
    reference_space_type: xr::ReferenceSpaceType,
    renderer: LobbyRenderer,
}

impl Lobby {
    pub fn new(
        xr_session: xr::Session<xr::OpenGlEs>,
        gfx_ctx: Rc<GraphicsContext>,
        interaction_ctx: Arc<RwLock<InteractionContext>>,
        platform: Platform,
        view_resolution: UVec2,
        initial_hud_message: &str,
    ) -> Self {
        let reference_space_type = if xr_session.instance().exts().ext_local_floor.is_some() {
            xr::ReferenceSpaceType::LOCAL_FLOOR_EXT
        } else {
            // The Quest 1 doesn't support LOCAL_FLOOR_EXT, recentering is required for AppLab, but
            // the Quest 1 is excluded from AppLab anyway.
            xr::ReferenceSpaceType::STAGE
        };

        let reference_space = interaction::get_reference_space(&xr_session, reference_space_type);

        let swapchains = [
            graphics::create_swapchain(&xr_session, &gfx_ctx, view_resolution, SDR_FORMAT_GL, None),
            graphics::create_swapchain(&xr_session, &gfx_ctx, view_resolution, SDR_FORMAT_GL, None),
        ];

        let renderer = LobbyRenderer::new(
            gfx_ctx,
            view_resolution,
            [
                swapchains[0]
                    .enumerate_images()
                    .unwrap()
                    .iter()
                    .map(|i| *i as _)
                    .collect(),
                swapchains[1]
                    .enumerate_images()
                    .unwrap()
                    .iter()
                    .map(|i| *i as _)
                    .collect(),
            ],
            initial_hud_message,
        );

        Self {
            xr_session,
            interaction_ctx,
            platform,
            reference_space,
            swapchains,
            view_resolution,
            reference_space_type,
            renderer,
        }
    }

    pub fn update_reference_space(&mut self) {
        self.reference_space =
            interaction::get_reference_space(&self.xr_session, self.reference_space_type);
    }

    pub fn update_hud_message(&mut self, message: &str) {
        self.renderer.update_hud_message(message);
    }

    pub fn render(&mut self, vsync_time: Duration) -> ProjectionLayerBuilder {
        let xr_vsync_time = xr::Time::from_nanos(vsync_time.as_nanos() as _);

        let (flags, maybe_views) = self
            .xr_session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                xr_vsync_time,
                &self.reference_space,
            )
            .unwrap();

        let views = if flags.contains(xr::ViewStateFlags::ORIENTATION_VALID) {
            maybe_views
        } else {
            vec![crate::default_view(), crate::default_view()]
        };

        self.xr_session
            .sync_actions(&[(&self.interaction_ctx.read().action_set).into()])
            .ok();

        // future_time doesn't have to be any particular value, just something after vsync_time
        let future_time = vsync_time + Duration::from_millis(80);
        let left_hand_data = interaction::get_hand_data(
            &self.xr_session,
            self.platform,
            &self.reference_space,
            vsync_time,
            future_time,
            &self.interaction_ctx.read().hands_interaction[0],
            &mut Pose::default(),
            &mut Pose::default(),
        );
        let right_hand_data = interaction::get_hand_data(
            &self.xr_session,
            self.platform,
            &self.reference_space,
            vsync_time,
            future_time,
            &self.interaction_ctx.read().hands_interaction[1],
            &mut Pose::default(),
            &mut Pose::default(),
        );

        let body_skeleton_fb = self
            .interaction_ctx
            .read()
            .body_sources
            .body_tracker_fb
            .as_ref()
            .and_then(|(tracker, joint_count)| {
                interaction::get_fb_body_skeleton(
                    &self.reference_space,
                    xr_vsync_time,
                    tracker,
                    *joint_count,
                )
            });

        let body_skeleton_bd = self
            .interaction_ctx
            .read()
            .body_sources
            .body_tracker_bd
            .as_ref()
            .and_then(|tracker| {
                interaction::get_bd_body_skeleton(&self.reference_space, xr_vsync_time, tracker)
            });

        let (body_skeleton, body_tracking_type) = {
            if body_skeleton_fb.is_some() {
                (body_skeleton_fb, Some(BodyTrackingType::Meta))
            } else if body_skeleton_bd.is_some() {
                (body_skeleton_bd, Some(BodyTrackingType::Pico))
            } else {
                (None, None)
            }
        };

        let left_swapchain_idx = self.swapchains[0].acquire_image().unwrap();
        let right_swapchain_idx = self.swapchains[1].acquire_image().unwrap();

        self.swapchains[0]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();
        self.swapchains[1]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();

        self.renderer.render(
            [
                LobbyViewParams {
                    pose: crate::from_xr_pose(views[0].pose),
                    fov: crate::from_xr_fov(views[0].fov),
                    swapchain_index: left_swapchain_idx,
                },
                LobbyViewParams {
                    pose: crate::from_xr_pose(views[1].pose),
                    fov: crate::from_xr_fov(views[1].fov),
                    swapchain_index: right_swapchain_idx,
                },
            ],
            [left_hand_data, right_hand_data],
            body_skeleton,
            body_tracking_type,
            false,
            cfg!(debug_assertions),
        );

        self.swapchains[0].release_image().unwrap();
        self.swapchains[1].release_image().unwrap();

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: self.view_resolution.x as _,
                height: self.view_resolution.y as _,
            },
        };

        ProjectionLayerBuilder::new(
            &self.reference_space,
            [
                xr::CompositionLayerProjectionView::new()
                    .pose(views[0].pose)
                    .fov(views[0].fov)
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchains[0])
                            .image_array_index(0)
                            .image_rect(rect),
                    ),
                xr::CompositionLayerProjectionView::new()
                    .pose(views[1].pose)
                    .fov(views[1].fov)
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchains[1])
                            .image_array_index(0)
                            .image_rect(rect),
                    ),
            ],
            Some(ProjectionLayerAlphaConfig {
                premultiplied: true,
            }),
        )
    }
}
