use crate::{
    graphics::{self, CompositionLayerBuilder},
    interaction,
};
use alvr_common::{anyhow::Result, glam::UVec2};
use alvr_graphics::{GraphicsContext, LobbyRenderer, VulkanBackend};
use openxr as xr;

// todo: add interaction?
pub struct Lobby {
    xr_session: xr::Session<xr::Vulkan>,

    reference_space: xr::Space,
    renderer: LobbyRenderer,
    swapchains: [xr::Swapchain<xr::Vulkan>; 2],
    view_resolution: UVec2,
}

impl Lobby {
    pub fn new(
        graphics_context: &GraphicsContext<VulkanBackend>,
        xr_session: xr::Session<xr::Vulkan>,
        view_resolution: UVec2,
    ) -> Result<Self> {
        let reference_space = interaction::get_stage_reference_space(&xr_session);

        let swapchains = [
            graphics::create_swapchain(&xr_session, view_resolution, 1),
            graphics::create_swapchain(&xr_session, view_resolution, 1),
        ];

        let swapchain_handles = [
            swapchains[0].enumerate_images()?,
            swapchains[1].enumerate_images()?,
        ];

        let renderer = LobbyRenderer::new(
            graphics_context,
            [
                graphics_context.create_vulkan_swapchain_external(
                    &swapchain_handles[0],
                    view_resolution,
                    1,
                ),
                graphics_context.create_vulkan_swapchain_external(
                    &swapchain_handles[1],
                    view_resolution,
                    1,
                ),
            ],
            view_resolution,
        )?;

        Ok(Self {
            xr_session,
            reference_space,
            renderer,
            swapchains,
            view_resolution,
        })
    }

    pub fn update_hud_message(&mut self, message: &str) -> Result<()> {
        self.renderer.update_hud_message(message)
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

        let left_swapchain_idx = self.swapchains[0].acquire_image().unwrap() as usize;
        let right_swapchain_idx = self.swapchains[1].acquire_image().unwrap() as usize;

        self.swapchains[0]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();
        self.swapchains[1]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();

        self.renderer.render(
            [
                crate::from_xr_pose(views[0].pose),
                crate::from_xr_pose(views[1].pose),
            ],
            [
                crate::from_xr_fov(views[0].fov),
                crate::from_xr_fov(views[1].fov),
            ],
            [left_swapchain_idx, right_swapchain_idx],
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
