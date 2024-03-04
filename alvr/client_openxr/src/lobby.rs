use crate::graphics;
use alvr_client_core::opengl::RenderViewInput;
use alvr_common::{glam::UVec2, parking_lot::RwLock};
use openxr as xr;
use std::sync::Arc;

// todo: add interaction?
pub struct Lobby {
    xr_session: xr::Session<xr::OpenGlEs>,
    reference_space: Arc<RwLock<xr::Space>>,
    rect: xr::Rect2Di,
    swapchains: [xr::Swapchain<xr::OpenGlEs>; 2],
}

impl Lobby {
    pub fn new(
        xr_session: xr::Session<xr::OpenGlEs>,
        reference_space: Arc<RwLock<xr::Space>>,
        default_view_resolution: UVec2,
    ) -> Self {
        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: default_view_resolution.x as _,
                height: default_view_resolution.y as _,
            },
        };

        let swapchains = [
            graphics::create_swapchain(&xr_session, default_view_resolution, None),
            graphics::create_swapchain(&xr_session, default_view_resolution, None),
        ];

        alvr_client_core::opengl::initialize_lobby(
            default_view_resolution,
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
        );

        Self {
            xr_session,
            reference_space,
            rect,
            swapchains,
        }
    }

    pub fn render(
        &mut self,
        predicted_display_time: xr::Time,
    ) -> [xr::CompositionLayerProjectionView<xr::OpenGlEs>; 2] {
        let (flags, maybe_views) = self
            .xr_session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                predicted_display_time,
                &self.reference_space.read(),
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
                pose: crate::to_pose(views[0].pose),
                fov: crate::to_fov(views[0].fov),
                swapchain_index: left_swapchain_idx,
            },
            RenderViewInput {
                pose: crate::to_pose(views[1].pose),
                fov: crate::to_fov(views[1].fov),
                swapchain_index: right_swapchain_idx,
            },
        ]);

        self.swapchains[0].release_image().unwrap();
        self.swapchains[1].release_image().unwrap();

        [
            xr::CompositionLayerProjectionView::new()
                .pose(views[0].pose)
                .fov(views[0].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchains[0])
                        .image_array_index(0)
                        .image_rect(self.rect),
                ),
            xr::CompositionLayerProjectionView::new()
                .pose(views[1].pose)
                .fov(views[1].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchains[1])
                        .image_array_index(0)
                        .image_rect(self.rect),
                ),
        ]
    }
}
