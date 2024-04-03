use crate::{
    graphics::{self, CompositionLayerBuilder},
    interaction,
};
use alvr_client_core::opengl::RenderViewInput;
use alvr_common::glam::UVec2;
use openxr as xr;

// todo: add interaction?
pub struct Lobby {
    xr_session: xr::Session<xr::OpenGlEs>,
    reference_space: xr::Space,
    swapchains: [xr::Swapchain<xr::OpenGlEs>; 2],
    view_resolution: UVec2,
}

impl Lobby {
    pub fn new(xr_session: xr::Session<xr::OpenGlEs>, view_resolution: UVec2) -> Self {
        let reference_space = interaction::get_stage_reference_space(&xr_session);

        let swapchains = [
            graphics::create_swapchain(&xr_session, view_resolution, None, false),
            graphics::create_swapchain(&xr_session, view_resolution, None, false),
        ];

        alvr_client_core::opengl::initialize_lobby(
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
            false, // TODO: correct lobby sRGB for some headsets
        );

        Self {
            xr_session,
            reference_space,
            swapchains,
            view_resolution,
        }
    }

    pub fn update_reference_space(&mut self) {
        self.reference_space = interaction::get_stage_reference_space(&self.xr_session);
    }

    pub fn render(&mut self, predicted_display_time: xr::Time) -> CompositionLayerBuilder {
        let (flags, maybe_views) = self
            .xr_session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                predicted_display_time,
                &self.reference_space,
            )
            .unwrap();

        let views = if flags.contains(xr::ViewStateFlags::ORIENTATION_VALID) {
            maybe_views
        } else {
            vec![crate::default_view(), crate::default_view()]
        };

        let left_swapchain_idx = self.swapchains[0].acquire_image().unwrap();
        let right_swapchain_idx = self.swapchains[1].acquire_image().unwrap();

        self.swapchains[0]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();
        self.swapchains[1]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();

        alvr_client_core::opengl::render_lobby([
            RenderViewInput {
                pose: crate::from_xr_pose(views[0].pose),
                fov: crate::from_xr_fov(views[0].fov),
                swapchain_index: left_swapchain_idx,
            },
            RenderViewInput {
                pose: crate::from_xr_pose(views[1].pose),
                fov: crate::from_xr_fov(views[1].fov),
                swapchain_index: right_swapchain_idx,
            },
        ]);

        self.swapchains[0].release_image().unwrap();
        self.swapchains[1].release_image().unwrap();

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: self.view_resolution.x as _,
                height: self.view_resolution.y as _,
            },
        };

        CompositionLayerBuilder::new(
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
        )
    }
}
