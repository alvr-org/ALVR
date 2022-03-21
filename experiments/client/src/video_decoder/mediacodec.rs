use alvr_common::{glam::UVec2, log, prelude::*};
use alvr_graphics::{
    ash::{self, vk},
    wgpu::{
        CommandEncoder, Device, Extent3d, ImageCopyTexture, Origin3d, ShaderModuleDescriptor,
        ShaderSource, Surface, Texture, TextureAspect, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsages, TextureView,
    },
    wgpu_hal as hal, GraphicsContext, QUAD_SHADER_WGSL,
};
use alvr_session::{CodecType, MediacodecDataType};
use ndk::{
    hardware_buffer::{HardwareBuffer, HardwareBufferUsage},
    media::{
        image_reader::{Image, ImageFormat, ImageReader},
        media_codec::{MediaCodec, MediaCodecDirection, MediaFormat},
        Result,
    },
};
use ndk_sys as sys;
use parking_lot::{Condvar, Mutex};
use std::{
    collections::HashMap,
    ffi::CString,
    io::Cursor,
    mem,
    ptr::{self, NonNull},
    sync::Arc,
    time::Duration,
};
use sys::AMediaCodec;

struct AcquiredImage {
    graphics_context: Arc<GraphicsContext>,
    memory: vk::DeviceMemory,
    image: vk::Image,
    image_view: vk::ImageView,
    timestamp: Duration,
}

impl Drop for AcquiredImage {
    fn drop(&mut self) {
        let device = &self.graphics_context.raw_device;

        unsafe {
            device.destroy_image_view(self.image_view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}

struct ConversionPass {
    graphics_context: Arc<GraphicsContext>,
    queue: vk::Queue,
    ycbcr_conversion: vk::SamplerYcbcrConversion,
    sampler: vk::Sampler,
    input_size: UVec2,
    input_allocation_size: vk::DeviceSize,
    input_format_properties: vk::AndroidHardwareBufferFormatPropertiesANDROID,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    vertex_shader_module: vk::ShaderModule,
    fragment_shader_module: vk::ShaderModule,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    output_size: UVec2,
    output_image: vk::Image,
    output_image_view: vk::ImageView,
    framebuffer: vk::Framebuffer,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    fence: vk::Fence,
}

impl ConversionPass {
    unsafe fn new(
        graphics_context: Arc<GraphicsContext>,
        input_android_image: Image,
        output_texture: &Texture,
        output_size: UVec2,
        slice_index: u32,
    ) -> StrResult<(Self, AcquiredImage)> {
        error!("creating conversion pass");

        let device = &graphics_context.raw_device;

        let queue = device.get_device_queue(
            graphics_context.queue_family_index,
            graphics_context.queue_index,
        );

        let input_size = UVec2::new(
            input_android_image.get_width().map_err(err!())? as _,
            input_android_image.get_height().map_err(err!())? as _,
        );
        let input_buffer_ptr = input_android_image
            .get_hardware_buffer()
            .map_err(err!())?
            .as_ptr();

        let mut input_format_properties =
            vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
        let mut hardware_buffer_properties = vk::AndroidHardwareBufferPropertiesANDROID::builder()
            .push_next(&mut input_format_properties)
            .build();

        {
            let ext_fns =
                vk::AndroidExternalMemoryAndroidHardwareBufferFn::load(|name: &std::ffi::CStr| {
                    mem::transmute(
                        graphics_context
                            .raw_instance
                            .get_device_proc_addr(device.handle(), name.as_ptr()),
                    )
                });
            ext_fns
                .get_android_hardware_buffer_properties_android(
                    device.handle(),
                    input_buffer_ptr as _,
                    &mut hardware_buffer_properties as _,
                )
                .result()
                .map_err(err!())?
        }

        // error!("buffer properties: {hardware_buffer_format_properties:?}");

        let ycbcr_conversion = device
            .create_sampler_ycbcr_conversion(
                &vk::SamplerYcbcrConversionCreateInfo::builder()
                    .format(input_format_properties.format)
                    .ycbcr_model(input_format_properties.suggested_ycbcr_model)
                    .ycbcr_range(input_format_properties.suggested_ycbcr_range)
                    .components(input_format_properties.sampler_ycbcr_conversion_components)
                    .x_chroma_offset(input_format_properties.suggested_x_chroma_offset)
                    .y_chroma_offset(input_format_properties.suggested_y_chroma_offset)
                    .chroma_filter(vk::Filter::LINEAR)
                    .push_next(
                        &mut vk::ExternalFormatANDROID::builder()
                            .external_format(input_format_properties.external_format),
                    ),
                None,
            )
            .map_err(err!())?;

        let sampler = device
            .create_sampler(
                &vk::SamplerCreateInfo::builder()
                    .mag_filter(vk::Filter::LINEAR)
                    .min_filter(vk::Filter::LINEAR)
                    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .min_lod(0.0)
                    .max_lod(1.0)
                    .push_next(
                        &mut vk::SamplerYcbcrConversionInfo::builder().conversion(ycbcr_conversion),
                    ),
                None,
            )
            .map_err(err!())?;

        let descriptor_set_layout = device
            .create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[
                    vk::DescriptorSetLayoutBinding::builder()
                        .binding(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                        .immutable_samplers(&[sampler])
                        .build(),
                ]),
                None,
            )
            .map_err(err!())?;

        let pipeline_layout = device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder().set_layouts(&[descriptor_set_layout]),
                None,
            )
            .map_err(err!())?;

        let vertex_shader_module = device
            .create_shader_module(
                &vk::ShaderModuleCreateInfo::builder().code(
                    &ash::util::read_spv(&mut Cursor::new(&include_bytes!(
                        "../../resources/quad.spv"
                    )))
                    .map_err(err!())?,
                ),
                None,
            )
            .map_err(err!())?;
        let fragment_shader_module = device
            .create_shader_module(
                &vk::ShaderModuleCreateInfo::builder().code(
                    &ash::util::read_spv(&mut Cursor::new(&include_bytes!(
                        "../../resources/ycbcr_conversion.spv"
                    )))
                    .map_err(err!())?,
                ),
                None,
            )
            .map_err(err!())?;

        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            compare_mask: 0,
            write_mask: 0,
            reference: 0,
        };

        let render_pass = device
            .create_render_pass(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&[vk::AttachmentDescription {
                        format: vk::Format::R8G8B8A8_SRGB,
                        samples: vk::SampleCountFlags::TYPE_1,
                        load_op: vk::AttachmentLoadOp::CLEAR,
                        store_op: vk::AttachmentStoreOp::STORE,
                        initial_layout: vk::ImageLayout::UNDEFINED,
                        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        ..Default::default()
                    }])
                    .subpasses(&[vk::SubpassDescription::builder()
                        .color_attachments(&[vk::AttachmentReference {
                            attachment: 0,
                            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        }])
                        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                        .build()]),
                None,
            )
            .map_err(err!())?;

        let pipelines = device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[vk::GraphicsPipelineCreateInfo::builder()
                    .stages(&[
                        vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::VERTEX,
                            module: vertex_shader_module,
                            p_name: b"main\0".as_ptr() as _,
                            ..Default::default()
                        },
                        vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::FRAGMENT,
                            module: fragment_shader_module,
                            p_name: b"main\0".as_ptr() as _,
                            ..Default::default()
                        },
                    ])
                    .vertex_input_state(
                        &vk::PipelineVertexInputStateCreateInfo::builder()
                            .vertex_binding_descriptions(&[])
                            .vertex_attribute_descriptions(&[]),
                    )
                    .input_assembly_state(
                        &vk::PipelineInputAssemblyStateCreateInfo::builder()
                            .topology(vk::PrimitiveTopology::TRIANGLE_LIST),
                    )
                    .viewport_state(
                        &vk::PipelineViewportStateCreateInfo::builder()
                            .viewports(&[vk::Viewport {
                                x: 0.0,
                                y: 0.0,
                                width: output_size.x as _,
                                height: output_size.y as _,
                                min_depth: 0.0,
                                max_depth: 1.0,
                            }])
                            .scissors(&[vk::Rect2D {
                                offset: vk::Offset2D { x: 0, y: 0 },
                                extent: vk::Extent2D {
                                    width: output_size.x,
                                    height: output_size.y,
                                },
                            }]),
                    )
                    .rasterization_state(
                        &vk::PipelineRasterizationStateCreateInfo::builder()
                            .cull_mode(vk::CullModeFlags::NONE)
                            .line_width(1.0),
                    )
                    .multisample_state(
                        &vk::PipelineMultisampleStateCreateInfo::builder()
                            .rasterization_samples(vk::SampleCountFlags::TYPE_1),
                    )
                    .depth_stencil_state(
                        &vk::PipelineDepthStencilStateCreateInfo::builder()
                            .depth_test_enable(false)
                            .depth_write_enable(false)
                            .front(noop_stencil_state)
                            .back(noop_stencil_state),
                    )
                    .color_blend_state(
                        &vk::PipelineColorBlendStateCreateInfo::builder()
                            .logic_op_enable(false)
                            .logic_op(vk::LogicOp::NO_OP)
                            .attachments(&[vk::PipelineColorBlendAttachmentState {
                                blend_enable: vk::FALSE,
                                src_color_blend_factor: vk::BlendFactor::ONE,
                                dst_color_blend_factor: vk::BlendFactor::ZERO,
                                color_blend_op: vk::BlendOp::ADD,
                                src_alpha_blend_factor: vk::BlendFactor::ONE,
                                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                                alpha_blend_op: vk::BlendOp::ADD,
                                color_write_mask: vk::ColorComponentFlags::R
                                    | vk::ColorComponentFlags::G
                                    | vk::ColorComponentFlags::B
                                    | vk::ColorComponentFlags::A,
                            }])
                            .blend_constants([1.0, 1.0, 1.0, 1.0]),
                    )
                    .layout(pipeline_layout)
                    .render_pass(render_pass)
                    .subpass(0)
                    .build()],
                None,
            )
            .map_err(|(_, err)| err)
            .map_err(err!())?;
        let pipeline = pipelines[0];

        let mut output_image = vk::Image::null();
        output_texture.as_hal::<hal::api::Vulkan, _>(|tex| {
            output_image = tex.unwrap().raw_handle();
        });

        let output_image_view = device
            .create_image_view(
                &vk::ImageViewCreateInfo::builder()
                    .image(output_image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::R8G8B8A8_SRGB)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: slice_index,
                        layer_count: 1,
                    }),
                None,
            )
            .map_err(err!())?;

        let framebuffer = device
            .create_framebuffer(
                &vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .width(output_size.x)
                    .height(output_size.y)
                    .attachments(&[output_image_view])
                    .layers(1),
                None,
            )
            .map_err(err!())?;

        let descriptor_pool = device
            .create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(1)
                    .pool_sizes(&[vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count: 1,
                    }]),
                None,
            )
            .map_err(err!())?;

        let descriptor_sets = device
            .allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(descriptor_pool)
                    .set_layouts(&[descriptor_set_layout]),
            )
            .map_err(err!())?;

        let command_pool = device
            .create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                    .queue_family_index(graphics_context.queue_family_index),
                None,
            )
            .map_err(err!())?;

        let fence = device
            .create_fence(&vk::FenceCreateInfo::default(), None)
            .map_err(err!())?;

        let command_buffers = device
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
            .map_err(err!())?;
        let command_buffer = command_buffers[0];

        device
            .begin_command_buffer(command_buffer, &Default::default())
            .map_err(err!())?;
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[vk::ImageMemoryBarrier::builder()
                .old_layout(vk::ImageLayout::UNDEFINED)
                // this is what wgpu will set in later stages. Set it as initial layout for cycle consistency
                // .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .image(output_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build()],
        );
        device.end_command_buffer(command_buffer).map_err(err!())?;

        device
            .queue_submit(
                queue,
                &[vk::SubmitInfo::builder()
                    .command_buffers(&[command_buffer])
                    .build()],
                fence,
            )
            .map_err(err!())?;

        let pass = Self {
            graphics_context,
            queue,
            ycbcr_conversion,
            sampler,
            input_size,
            input_allocation_size: hardware_buffer_properties.allocation_size,
            input_format_properties,
            descriptor_set_layout,
            pipeline_layout,
            vertex_shader_module,
            fragment_shader_module,
            render_pass,
            pipeline,
            output_size,
            output_image,
            output_image_view,
            framebuffer,
            descriptor_pool,
            descriptor_sets,
            command_pool,
            command_buffers,
            fence,
        };

        // this is actually waiting for the output image layout conversison
        pass.wait_for_image()?;

        let image = pass.create_acquired_image(input_android_image)?;

        error!("conversion pass created");

        Ok((pass, image))
    }

    // returns false for fence wait timeout
    unsafe fn execute(&mut self, acquired_image: &AcquiredImage) -> StrResult {
        error!("Execute conversion pass");

        let device = &self.graphics_context.raw_device;

        device.reset_fences(&[self.fence]).map_err(err!())?;

        device.free_command_buffers(self.command_pool, &self.command_buffers);

        device.update_descriptor_sets(
            &[vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_sets[0])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[vk::DescriptorImageInfo {
                    sampler: self.sampler,
                    image_view: acquired_image.image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }])
                .build()],
            &[],
        );

        self.command_buffers = device
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(self.command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
            .map_err(err!())?;
        let command_buffer = self.command_buffers[0];

        device
            .begin_command_buffer(command_buffer, &Default::default())
            .map_err(err!())?;
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[
                vk::ImageMemoryBarrier::builder()
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image(acquired_image.image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .build(),
                // vk::ImageMemoryBarrier::builder()
                //     .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                //     .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                //     .image(self.output_image)
                //     .subresource_range(vk::ImageSubresourceRange {
                //         aspect_mask: vk::ImageAspectFlags::COLOR,
                //         base_mip_level: 0,
                //         level_count: 1,
                //         base_array_layer: 0,
                //         layer_count: 1,
                //     })
                //     .build(),
            ],
        );
        device.cmd_begin_render_pass(
            command_buffer,
            &vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: self.output_size.x,
                        height: self.output_size.y,
                    },
                })
                .clear_values(&[
                    vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [1.0, 1.0, 1.0, 1.0], // white
                        },
                    },
                    vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    },
                ]),
            vk::SubpassContents::INLINE,
        );
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        );
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_layout,
            0,
            &self.descriptor_sets,
            &[],
        );
        device.cmd_draw(command_buffer, 6, 1, 0, 0);
        device.cmd_end_render_pass(command_buffer);
        device.end_command_buffer(command_buffer).map_err(err!())?;

        device
            .queue_submit(
                self.queue,
                &[vk::SubmitInfo::builder()
                    .command_buffers(&[command_buffer])
                    .build()],
                self.fence,
            )
            .map_err(err!())?;

        error!("finished conversion pass");

        Ok(())
    }

    unsafe fn wait_for_image(&self) -> StrResult {
        self.graphics_context
            .raw_device
            .wait_for_fences(&[self.fence], true, !0)
            .map_err(err!())
        // NB the fence is not reset yet.
    }

    unsafe fn create_acquired_image(&self, android_image: Image) -> StrResult<AcquiredImage> {
        let device = &self.graphics_context.raw_device;

        let buffer_ptr = android_image
            .get_hardware_buffer()
            .map_err(err!())?
            .as_ptr();
        let timestamp = Duration::from_nanos(android_image.get_timestamp().map_err(err!())? as _);

        let image = device
            .create_image(
                &vk::ImageCreateInfo::builder()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(self.input_format_properties.format)
                    .extent(vk::Extent3D {
                        width: self.input_size.x,
                        height: self.input_size.y,
                        depth: 1,
                    })
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(vk::ImageUsageFlags::SAMPLED)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .push_next(
                        &mut vk::ExternalMemoryImageCreateInfo::builder().handle_types(
                            vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID,
                        ),
                    )
                    .push_next(
                        &mut vk::ExternalFormatANDROID::builder()
                            .external_format(self.input_format_properties.external_format),
                    ),
                None,
            )
            .map_err(err!())?;

        let mut dedicated_allocate_info = vk::MemoryDedicatedAllocateInfo::builder().image(image);
        let mut hardware_buffer_info =
            vk::ImportAndroidHardwareBufferInfoANDROID::builder().buffer(buffer_ptr as _);
        let memory = device
            .allocate_memory(
                &vk::MemoryAllocateInfo::builder()
                    .allocation_size(self.input_allocation_size)
                    .memory_type_index(1)
                    .push_next(&mut dedicated_allocate_info)
                    .push_next(&mut hardware_buffer_info),
                None,
            )
            .map_err(err!())?;

        device
            .bind_image_memory2(&[vk::BindImageMemoryInfo::builder()
                .image(image)
                .memory(memory)
                .build()])
            .map_err(err!())?;

        let image_view = device
            .create_image_view(
                &vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(self.input_format_properties.format)
                    .components(
                        self.input_format_properties
                            .sampler_ycbcr_conversion_components,
                    )
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .push_next(
                        &mut vk::SamplerYcbcrConversionInfo::builder()
                            .conversion(self.ycbcr_conversion),
                    ),
                None,
            )
            .map_err(err!())?;

        Ok(AcquiredImage {
            graphics_context: Arc::clone(&self.graphics_context),
            memory,
            image,
            image_view,
            timestamp,
        })
    }
}

impl Drop for ConversionPass {
    fn drop(&mut self) {
        let device = &self.graphics_context.raw_device;

        // Destroy in reverse order
        unsafe {
            device.destroy_fence(self.fence, None);

            device.free_command_buffers(self.command_pool, &self.command_buffers);
            device.destroy_command_pool(self.command_pool, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);

            device.destroy_framebuffer(self.framebuffer, None);
            device.destroy_image_view(self.output_image_view, None);

            device.destroy_pipeline(self.pipeline, None);
            device.destroy_render_pass(self.render_pass, None);
            device.destroy_shader_module(self.fragment_shader_module, None);
            device.destroy_shader_module(self.vertex_shader_module, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            device.destroy_sampler(self.sampler, None);
            device.destroy_sampler_ycbcr_conversion(self.ycbcr_conversion, None);
        }
    }
}

pub struct VideoDecoderEnqueuer {
    inner: Arc<MediaCodec>,
}

unsafe impl Send for VideoDecoderEnqueuer {}

impl VideoDecoderEnqueuer {
    // Block until the buffer has been written or timeout is reached. Returns false if timeout.
    pub fn push_frame_nals(
        &self,
        timestamp: Duration,
        data: &[u8],
        timeout: Duration,
    ) -> StrResult<bool> {
        if let Some(mut buffer) = self.inner.dequeue_input_buffer(timeout).map_err(err!())? {
            buffer.get_mut()[..data.len()].copy_from_slice(data);

            // NB: the function expects the timestamp in micros, but nanos is used to have complete
            // precision, so when converted back to Duration it can compare correctly to other
            // Durations
            self.inner
                .queue_input_buffer(buffer, 0, data.len(), timestamp.as_nanos() as _, 0)
                .map_err(err!())?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub struct VideoDecoderDequeuer {
    inner: Arc<MediaCodec>,
}

unsafe impl Send for VideoDecoderDequeuer {}

impl VideoDecoderDequeuer {
    pub fn poll(&self, timeout: Duration) -> StrResult {
        if let Some(buffer) = self.inner.dequeue_output_buffer(timeout).map_err(err!())? {
            self.inner
                .release_output_buffer(buffer, true)
                .map_err(err!())
        } else {
            Ok(())
        }
    }
}

struct ConversionContext {
    pass: Option<ConversionPass>,
    ready_image: Option<AcquiredImage>,
    used_image: Option<AcquiredImage>,
}

pub struct VideoDecoderFrameGrabber {
    inner: Arc<MediaCodec>,
    swapchain: ImageReader,
    conversion_context: Arc<Mutex<ConversionContext>>,
    image_notifier: Arc<Condvar>,
}

unsafe impl Send for VideoDecoderFrameGrabber {}

impl VideoDecoderFrameGrabber {
    // Block until one frame is available or timeout is reached. Returns the frame timestamp (as
    // specified in push_frame_nals())
    pub fn get_output_frame(&self, timeout: Duration) -> StrResult<Duration> {
        let mut context_lock = self.conversion_context.lock();

        let ready_image = if let Some(image) = context_lock.ready_image.take() {
            image
        } else {
            let result = self.image_notifier.wait_for(&mut context_lock, timeout);

            if let Some(image) = context_lock.ready_image.take() {
                image
            } else {
                return fmt_e!("Decoded image unavailable or timeout");
            }
        };

        // This is executed on the render thread. It ensures that this render pass is submitted
        // before the other compositor passes and the texture is not used concurrently.
        unsafe { context_lock.pass.as_mut().unwrap().execute(&ready_image)? };

        let timestamp = ready_image.timestamp;

        context_lock.used_image = Some(ready_image);

        Ok(timestamp)
    }
}

pub fn split(
    graphics_context: Arc<GraphicsContext>,
    codec_type: CodecType,
    csd_0: &[u8],
    extra_options: &[(String, MediacodecDataType)],
    output_texture: Arc<Texture>,
    output_size: UVec2,
    slice_index: u32,
) -> StrResult<(
    VideoDecoderEnqueuer,
    VideoDecoderDequeuer,
    VideoDecoderFrameGrabber,
)> {
    let mut swapchain = ImageReader::new_with_usage(
        1,
        1,
        ImageFormat::PRIVATE,
        HardwareBufferUsage::GPU_SAMPLED_IMAGE,
        4, // 2 concurrent locks on application side, 1 render surface for Mediacodec, 1 for safety
    )
    .map_err(err!())?;

    let conversion_context = Arc::new(Mutex::new(ConversionContext {
        pass: None,
        ready_image: None,
        used_image: None,
    }));
    let image_notifier = Arc::new(Condvar::new());

    swapchain
        .set_image_listener(Box::new({
            let conversion_context = Arc::clone(&conversion_context);
            let image_notifier = Arc::clone(&image_notifier);
            move |swapchain| {
                // the used image outlives the lock. This is done so that the render thread can be
                // unblocked while the used image gets freed.
                let _used_image = {
                    let context = &mut *conversion_context.lock();

                    error!("Acquire image");

                    if let Some(pass) = &context.pass {
                        show_err(unsafe { pass.wait_for_image() });

                        if let Some(image) = swapchain.acquire_latest_image().ok().flatten() {
                            context.ready_image =
                                show_err(unsafe { pass.create_acquired_image(image) });

                            image_notifier.notify_one();
                        }

                        context.used_image.take()
                    } else {
                        if let Some(image) = swapchain.acquire_latest_image().ok().flatten() {
                            let maybe_pair = show_err(unsafe {
                                ConversionPass::new(
                                    Arc::clone(&graphics_context),
                                    image,
                                    &output_texture,
                                    output_size,
                                    slice_index,
                                )
                            });

                            if let Some((pass, image)) = maybe_pair {
                                error!("Conversion pass created");

                                context.pass = Some(pass);
                                context.ready_image = Some(image);

                                image_notifier.notify_one();
                            }
                        }

                        None
                    }
                };
            }
        }))
        .map_err(err!())?;

    let mime = match codec_type {
        CodecType::H264 => "video/avc",
        CodecType::HEVC => "video/hevc",
    };

    let format = MediaFormat::new();
    format.set_str("mime", mime);
    format.set_i32("width", 512);
    format.set_i32("height", 1024);
    format.set_buffer("csd-0", csd_0);

    for (key, value) in extra_options {
        match value {
            MediacodecDataType::Float(value) => format.set_f32(key, *value),
            MediacodecDataType::Int32(value) => format.set_i32(key, *value),
            MediacodecDataType::Int64(value) => format.set_i64(key, *value),
            MediacodecDataType::String(value) => format.set_str(key, value),
        }
    }

    let decoder = Arc::new(MediaCodec::from_decoder_type(mime).ok_or_else(enone!())?);
    decoder
        .configure(
            &format,
            &swapchain.get_window().map_err(err!())?,
            MediaCodecDirection::Decoder,
        )
        .map_err(err!())?;
    decoder.start().map_err(err!())?;

    Ok((
        VideoDecoderEnqueuer {
            inner: Arc::clone(&decoder),
        },
        VideoDecoderDequeuer {
            inner: Arc::clone(&decoder),
        },
        VideoDecoderFrameGrabber {
            inner: decoder,
            swapchain,
            conversion_context,
            image_notifier,
        },
    ))
}
