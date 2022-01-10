use alvr_common::{glam::UVec2, log, prelude::*};
use alvr_graphics::{
    ash::{self, vk},
    wgpu::{
        CommandEncoder, Device, Extent3d, ImageCopyTexture, Origin3d, Surface, Texture,
        TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        TextureView,
    },
    wgpu_hal as hal, GraphicsContext, QUAD_SHADER_WGSL,
};
use alvr_session::{CodecType, MediacodecDataType};
use naga::{
    back::spv::{self, Options, PipelineOptions, WriterFlags},
    front::{glsl, wgsl},
    proc::{BoundsCheckPolicies, BoundsCheckPolicy},
    valid::{Capabilities, ModuleInfo, ValidationFlags, Validator},
    ShaderStage,
};
use ndk::{
    hardware_buffer::{HardwareBuffer, HardwareBufferUsage},
    media::{
        image_reader::{Image, ImageFormat, ImageReader},
        media_codec::{MediaCodec, MediaCodecDirection, MediaFormat},
        Result,
    },
};
use ndk_sys as sys;
use std::{
    collections::HashMap,
    ffi::CString,
    mem,
    ptr::{self, NonNull},
    sync::Arc,
    time::Duration,
};
use sys::AMediaCodec;

pub struct ConversionPass {
    graphics_context: Arc<GraphicsContext>,
    queue: vk::Queue,
    ycbcr_conversion: vk::SamplerYcbcrConversion,
    sampler: vk::Sampler,
    input_image: vk::Image,
    input_image_view: vk::ImageView,
    input_memory: vk::DeviceMemory,
    input_allocation_size: vk::DeviceSize,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    vertex_shader_module: vk::ShaderModule,
    fragment_shader_module: vk::ShaderModule,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    output_image_view: vk::ImageView,
    framebuffer: vk::Framebuffer,
    descriptor_pool: vk::DescriptorPool,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
}

impl ConversionPass {
    unsafe fn bind_hardware_buffer(
        device: &ash::Device,
        image: vk::Image,
        buffer_ptr: *mut sys::AHardwareBuffer,
        allocation_size: vk::DeviceSize,
    ) -> StrResult<vk::DeviceMemory> {
        let mut dedicated_allocate_info = vk::MemoryDedicatedAllocateInfo::builder().image(image);
        let mut hardware_buffer_info =
            vk::ImportAndroidHardwareBufferInfoANDROID::builder().buffer(buffer_ptr as _);
        let memory = trace_err!(device.allocate_memory(
            &vk::MemoryAllocateInfo::builder()
                .allocation_size(allocation_size)
                .memory_type_index(1)
                .push_next(&mut dedicated_allocate_info)
                .push_next(&mut hardware_buffer_info),
            None,
        ))?;

        trace_err!(
            device.bind_image_memory2(&[vk::BindImageMemoryInfo::builder()
                .image(image)
                .memory(memory)
                .build()])
        )?;

        Ok(memory)
    }

    unsafe fn new(
        graphics_context: Arc<GraphicsContext>,
        input_size: UVec2,
        input_buffer_ptr: *mut sys::AHardwareBuffer,
        output_texture: &Texture,
        output_size: UVec2,
        slice_index: u32,
    ) -> StrResult<Self> {
        error!("creating conversion pass");

        let device = &graphics_context.raw_device;

        let queue = device.get_device_queue(
            graphics_context.queue_family_index,
            graphics_context.queue_index,
        );

        let mut hardware_buffer_format_properties =
            vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
        let mut hardware_buffer_properties = vk::AndroidHardwareBufferPropertiesANDROID::builder()
            .push_next(&mut hardware_buffer_format_properties)
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
            let res = ext_fns.get_android_hardware_buffer_properties_android(
                device.handle(),
                input_buffer_ptr as _,
                &mut hardware_buffer_properties as _,
            );
            if res != vk::Result::SUCCESS {
                return fmt_e!("{}", res);
            }
        }

        // error!("buffer properties: {:?}", hardware_buffer_format_properties);

        let ycbcr_conversion = trace_err!(device.create_sampler_ycbcr_conversion(
            &vk::SamplerYcbcrConversionCreateInfo::builder()
                .format(hardware_buffer_format_properties.format)
                .ycbcr_model(hardware_buffer_format_properties.suggested_ycbcr_model)
                .ycbcr_range(hardware_buffer_format_properties.suggested_ycbcr_range)
                .components(hardware_buffer_format_properties.sampler_ycbcr_conversion_components)
                .x_chroma_offset(hardware_buffer_format_properties.suggested_x_chroma_offset)
                .y_chroma_offset(hardware_buffer_format_properties.suggested_y_chroma_offset)
                .chroma_filter(vk::Filter::LINEAR)
                .push_next(
                    &mut vk::ExternalFormatANDROID::builder()
                        .external_format(hardware_buffer_format_properties.external_format),
                ),
            None
        ))?;

        let sampler = trace_err!(device.create_sampler(
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
                    &mut vk::SamplerYcbcrConversionInfo::builder().conversion(ycbcr_conversion)
                ),
            None,
        ))?;

        let input_image = trace_err!(device.create_image(
            &vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .format(hardware_buffer_format_properties.format)
                .extent(vk::Extent3D {
                    width: input_size.x,
                    height: input_size.y,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .push_next(
                    &mut vk::ExternalFormatANDROID::builder()
                        .external_format(hardware_buffer_format_properties.external_format),
                ),
            None,
        ))?;

        let input_memory = Self::bind_hardware_buffer(
            device,
            input_image,
            input_buffer_ptr,
            hardware_buffer_properties.allocation_size,
        )?;

        let input_image_view = trace_err!(device.create_image_view(
            &vk::ImageViewCreateInfo::builder()
                .image(input_image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(hardware_buffer_format_properties.format)
                .components(hardware_buffer_format_properties.sampler_ycbcr_conversion_components)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .push_next(
                    &mut vk::SamplerYcbcrConversionInfo::builder().conversion(ycbcr_conversion)
                ),
            None
        ))?;

        let descriptor_set_layout = trace_err!(device.create_descriptor_set_layout(
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
        ))?;

        let pipeline_layout = trace_err!(device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder().set_layouts(&[descriptor_set_layout]),
            None,
        ))?;

        let mut shader_validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        let spv_out_options = Options {
            lang_version: (1, 0),
            flags: WriterFlags::DEBUG | WriterFlags::FORCE_POINT_SIZE,
            capabilities: None,
            bounds_check_policies: BoundsCheckPolicies {
                index: BoundsCheckPolicy::Unchecked,
                buffer: BoundsCheckPolicy::Unchecked,
                image: BoundsCheckPolicy::Unchecked,
            },
        };

        let naga_vertex_shader_module = trace_err!(wgsl::parse_str(QUAD_SHADER_WGSL))?;
        let quad_shader_spirv = trace_err!(spv::write_vec(
            &naga_vertex_shader_module,
            &trace_err!(shader_validator.validate(&naga_vertex_shader_module))?,
            &spv_out_options,
            Some(&PipelineOptions {
                shader_stage: ShaderStage::Vertex,
                entry_point: "main".to_owned(),
            }),
        ))?;
        let vertex_shader_module = trace_err!(device.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(&quad_shader_spirv),
            None
        ))?;

        let naga_fragment_shader_module = trace_err_dbg!(glsl::Parser::default().parse(
            &glsl::Options {
                stage: ShaderStage::Fragment,
                defines: Default::default(),
            },
            include_str!("../../resources/ycbcr_conversion.glsl")
        ))?;
        let fragment_shader_spirv = trace_err!(spv::write_vec(
            &naga_fragment_shader_module,
            &trace_err!(shader_validator.validate(&naga_fragment_shader_module))?,
            &spv_out_options,
            Some(&PipelineOptions {
                shader_stage: ShaderStage::Fragment,
                entry_point: "main".to_owned(),
            }),
        ))?;
        let fragment_shader_module = trace_err!(device.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(&fragment_shader_spirv),
            None
        ))?;

        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            compare_mask: 0,
            write_mask: 0,
            reference: 0,
        };

        let render_pass = trace_err!(device.create_render_pass(
            &vk::RenderPassCreateInfo::builder()
                .attachments(&[vk::AttachmentDescription {
                    format: vk::Format::R8G8B8A8_SRGB,
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
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
        ))?;

        let pipelines = trace_err!(device
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
                        }
                    ])
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
            .map_err(|(_, err)| err))?;
        let pipeline = pipelines[0];

        let mut output_image = vk::Image::null();
        output_texture.as_hal::<hal::api::Vulkan, _>(|tex| {
            output_image = tex.unwrap().raw_handle();
        });

        let output_image_view = trace_err!(device.create_image_view(
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
            None
        ))?;

        let framebuffer = trace_err!(device.create_framebuffer(
            &vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .width(output_size.x)
                .height(output_size.y)
                .attachments(&[output_image_view])
                .layers(1),
            None,
        ))?;

        let descriptor_pool = trace_err!(device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&[vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1
                }]),
            None,
        ))?;

        let descriptor_sets = trace_err!(device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&[descriptor_set_layout]),
        ))?;

        device.update_descriptor_sets(
            &[vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_sets[0])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[vk::DescriptorImageInfo {
                    sampler,
                    image_view: input_image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }])
                .build()],
            &[],
        );

        let command_pool = trace_err!(device.create_command_pool(
            &vk::CommandPoolCreateInfo::builder()
                .flags(vk::CommandPoolCreateFlags::empty()) // no transient, no reset
                .queue_family_index(graphics_context.queue_family_index),
            None,
        ))?;

        let command_buffers = trace_err!(device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        ))?;
        let command_buffer = command_buffers[0];

        trace_err!(device.begin_command_buffer(command_buffer, &Default::default()))?;
        device.cmd_begin_render_pass(
            command_buffer,
            &vk::RenderPassBeginInfo::builder()
                .render_pass(render_pass)
                .framebuffer(framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: output_size.x,
                        height: output_size.y,
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
        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline_layout,
            0,
            &descriptor_sets,
            &[],
        );
        device.cmd_draw_indexed(command_buffer, 6, 1, 0, 0, 0);
        device.cmd_end_render_pass(command_buffer);
        trace_err!(device.end_command_buffer(command_buffer))?;

        let fence = trace_err!(device.create_fence(
            &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
            None
        ))?;

        error!("conversion pass created");

        Ok(Self {
            graphics_context,
            queue,
            ycbcr_conversion,
            sampler,
            input_image,
            input_image_view,
            input_memory,
            input_allocation_size: hardware_buffer_properties.allocation_size,
            descriptor_set_layout,
            pipeline_layout,
            vertex_shader_module,
            fragment_shader_module,
            render_pass,
            pipeline,
            output_image_view,
            framebuffer,
            descriptor_pool,
            command_pool,
            command_buffer,
            fence,
        })
    }

    // returns false for fence wait timeout
    unsafe fn execute(&mut self, new_buffer_ptr: *mut sys::AHardwareBuffer) -> StrResult {
        error!("Execute conversion pass");

        let device = &self.graphics_context.raw_device;

        trace_err!(device.wait_for_fences(&[self.fence], true, !0))?;
        trace_err!(device.reset_fences(&[self.fence]))?;

        device.free_memory(self.input_memory, None);
        // the old hardware buffer is released here

        self.input_memory = Self::bind_hardware_buffer(
            device,
            self.input_image,
            new_buffer_ptr,
            self.input_allocation_size,
        )?;

        // conversion happens here
        trace_err!(device.queue_submit(
            self.queue,
            &[vk::SubmitInfo::builder()
                .command_buffers(&[self.command_buffer])
                .build()],
            self.fence,
        ))
    }
}

impl Drop for ConversionPass {
    fn drop(&mut self) {
        let device = &self.graphics_context.raw_device;

        // Destroy in reverse order
        unsafe {
            device.destroy_fence(self.fence, None);

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

            device.destroy_image_view(self.input_image_view, None);
            device.destroy_image(self.input_image, None);
            device.destroy_sampler(self.sampler, None);
            device.destroy_sampler_ycbcr_conversion(self.ycbcr_conversion, None);

            device.free_memory(self.input_memory, None);
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
        if let Some(mut buffer) = trace_err!(self.inner.dequeue_input_buffer(timeout))? {
            buffer.get_mut()[..data.len()].copy_from_slice(data);

            // NB: the function expects the timestamp in micros, but nanos is used to have complete
            // precision, so when converted back to Duration it can compare correctly to other
            // Durations
            trace_err!(self.inner.queue_input_buffer(
                buffer,
                0,
                data.len(),
                timestamp.as_nanos() as _,
                0
            ))?;

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
        if let Some(buffer) = trace_err!(self.inner.dequeue_output_buffer(timeout))? {
            trace_err!(self.inner.release_output_buffer(buffer, true))
        } else {
            Ok(())
        }
    }
}

pub struct VideoDecoderFrameGrabber {
    graphics_context: Arc<GraphicsContext>,
    inner: Arc<MediaCodec>,
    swapchain: ImageReader,
    image_receiver: crossbeam_channel::Receiver<Image>,
    output_texture: Arc<Texture>,
    output_size: UVec2,
    slice_index: u32,
    conversion_pass: Option<ConversionPass>,
}

unsafe impl Send for VideoDecoderFrameGrabber {}

impl VideoDecoderFrameGrabber {
    // Block until one frame is available or timeout is reached. Returns the frame timestamp (as
    // specified in push_frame_nals())
    pub fn get_output_frame(&mut self, timeout: Duration) -> StrResult<Duration> {
        let image = trace_err!(self.image_receiver.recv_timeout(timeout))?;

        error!(
            "image: format {:?}, width: {:?}, height: {:?}, rect: {:?}, pixel stride (UV): {:?}, row_stride (UV): {:?}",
            image.get_format(),
            image.get_width(),
            image.get_height(),
            image.get_crop_rect(),
            image.get_plane_pixel_stride(1),
            image.get_plane_row_stride(1),
        );

        let hardware_buffer = trace_err!(image.get_hardware_buffer())?;

        let conversion_pass = if let Some(pass) = &mut self.conversion_pass {
            pass
        } else {
            self.conversion_pass = Some(unsafe {
                ConversionPass::new(
                    Arc::clone(&self.graphics_context),
                    UVec2::new(
                        trace_err!(image.get_width())? as _,
                        trace_err!(image.get_height())? as _,
                    ),
                    hardware_buffer.as_ptr(),
                    &self.output_texture,
                    self.output_size,
                    self.slice_index,
                )?
            });

            self.conversion_pass.as_mut().unwrap()
        };

        unsafe { conversion_pass.execute(hardware_buffer.as_ptr())? };

        Ok(Duration::from_nanos(trace_err!(image.get_timestamp())? as _))
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
    let mut swapchain = trace_err!(ImageReader::new_with_usage(
        1,
        1,
        ImageFormat::YUV_420_888,
        HardwareBufferUsage::GPU_SAMPLED_IMAGE,
        3, // to avoid a deadlock, a triple buffered swapchain is required
    ))?;

    let (image_sender, image_receiver) = crossbeam_channel::unbounded();

    trace_err!(swapchain.set_image_listener(Box::new(move |swapchain| {
        let maybe_image = swapchain.acquire_next_image();
        error!("maybe acquired image: {:?}", maybe_image);

        if let Some(image) = maybe_image.ok().flatten() {
            image_sender.send(image).ok();
        }
    })))?;

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

    let decoder = Arc::new(trace_none!(MediaCodec::from_decoder_type(mime))?);
    trace_err!(decoder.configure(
        &format,
        &trace_err!(swapchain.get_window())?,
        MediaCodecDirection::Decoder,
    ))?;
    trace_err!(decoder.start())?;

    Ok((
        VideoDecoderEnqueuer {
            inner: Arc::clone(&decoder),
        },
        VideoDecoderDequeuer {
            inner: Arc::clone(&decoder),
        },
        VideoDecoderFrameGrabber {
            inner: decoder,
            graphics_context,
            swapchain,
            image_receiver,
            output_texture,
            output_size,
            slice_index,
            conversion_pass: None,
        },
    ))
}
