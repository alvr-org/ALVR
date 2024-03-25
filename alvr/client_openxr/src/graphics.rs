use alvr_common::{
    anyhow::{anyhow, Result},
    glam::UVec2,
    ToAny,
};
use alvr_graphics::{GraphicsContext, VulkanBackend, VulkanInitCallbacks, TARGET_FORMAT_VK};
use openxr as xr;

pub fn create_graphics_context(
    xr_instance: &xr::Instance,
    xr_system: xr::SystemId,
) -> Result<GraphicsContext<VulkanBackend>> {
    let _ = xr_instance.graphics_requirements::<xr::Vulkan>(xr_system)?;

    GraphicsContext::<VulkanBackend>::new_vulkan_external(VulkanInitCallbacks {
        create_instance: &move |get_instance_proc_addr, create_info_ptr| unsafe {
            xr_instance
                .create_vulkan_instance(xr_system, get_instance_proc_addr, create_info_ptr)?
                .map_err(|i| anyhow!("Failed to create Vulkan instance: {:?}", i))
        },
        get_physical_device: &move |instance_ptr| unsafe {
            xr_instance
                .vulkan_graphics_device(xr_system, instance_ptr)
                .to_any()
        },
        create_device: &move |get_instance_proc_addr, phisical_device_ptr, create_info_ptr| unsafe {
            xr_instance
                .create_vulkan_device(
                    xr_system,
                    get_instance_proc_addr,
                    phisical_device_ptr,
                    create_info_ptr,
                )?
                .map_err(|i| anyhow!("Failed to create Vulkan device: {:?}", i))
        },
    })
}

pub fn session_create_info(
    graphics_context: &GraphicsContext<VulkanBackend>,
) -> xr::vulkan::SessionCreateInfo {
    let handles = graphics_context.vulkan_handles();

    xr::vulkan::SessionCreateInfo {
        instance: handles.instance,
        physical_device: handles.physical_device,
        device: handles.device,
        queue_family_index: handles.queue_family_index,
        queue_index: handles.queue_index,
    }
}

// array_size: number of layers per texture. 2 is used for multiview
pub fn create_swapchain(
    session: &xr::Session<xr::Vulkan>,
    resolution: UVec2,
    // foveation: Option<&xr::FoveationProfileFB>,
    array_size: u32,
) -> xr::Swapchain<xr::Vulkan> {
    let swapchain_info = xr::SwapchainCreateInfo {
        create_flags: xr::SwapchainCreateFlags::EMPTY,
        usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT, // check if needed: xr::SwapchainUsageFlags::SAMPLED,
        format: TARGET_FORMAT_VK.as_raw() as u32,
        sample_count: 1,
        width: resolution.x,
        height: resolution.y,
        face_count: 1,
        array_size,
        mip_count: 1,
    };

    // if let Some(foveation) = foveation {
    //     let swapchain = session
    //         .create_swapchain_with_foveation(
    //             &swapchain_info,
    //             xr::SwapchainCreateFoveationFlagsFB::SCALED_BIN,
    //         )
    //         .unwrap();

    //     swapchain.update_foveation(foveation).unwrap();

    //     swapchain
    // } else {
    session.create_swapchain(&swapchain_info).unwrap()
    // }
}

// This is needed to work around lifetime limitations
pub struct CompositionLayerBuilder<'a> {
    reference_space: &'a xr::Space,
    layers: [xr::CompositionLayerProjectionView<'a, xr::Vulkan>; 2],
}

impl<'a> CompositionLayerBuilder<'a> {
    pub fn new(
        reference_space: &'a xr::Space,
        layers: [xr::CompositionLayerProjectionView<'a, xr::Vulkan>; 2],
    ) -> Self {
        Self {
            reference_space,
            layers,
        }
    }

    pub fn build(&self) -> xr::CompositionLayerProjection<xr::Vulkan> {
        xr::CompositionLayerProjection::new()
            .space(self.reference_space)
            .views(&self.layers)
    }
}
