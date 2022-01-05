use alvr_common::{glam::UVec2, log, prelude::*};
use alvr_graphics::{
    ash::{
        self,
        vk::{self, FenceCreateFlags, SampleCountFlags, SharingMode},
    },
    wgpu::{
        self, CommandEncoder, Device, Extent3d, ImageCopyTexture, Origin3d, Surface, Texture,
        TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        TextureView,
    },
    wgpu_hal as hal, GraphicsContext,
};
use alvr_session::{CodecType, MediacodecDataType};
use ndk::{
    hardware_buffer::{HardwareBuffer, HardwareBufferUsage},
    media::{
        image_reader::{Image, ImageFormat, ImageReader},
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
    input_image: vk::Image,
    input_image_view: vk::ImageView,
    input_memory: vk::DeviceMemory,
    input_allocation_size: vk::DeviceSize,
    sampler: vk::Sampler,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    command_pool: vk::CommandPool,
    output_image_view: vk::ImageView,
    framebuffer: vk::Framebuffer,
    fence: vk::Fence,
}

impl ConversionPass {
    unsafe fn new(
        graphics_context: Arc<GraphicsContext>,
        input_size: UVec2,
        input_buffer_ptr: *mut sys::AHardwareBuffer,
        output_texture: &Texture,
        output_size: UVec2,
        slice_index: u32,
    ) -> StrResult<Self> {
        error!("creating conversion context");

        let mut hardware_buffer_format_properties =
            vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
        let mut hardware_buffer_properties = vk::AndroidHardwareBufferPropertiesANDROID::builder()
            .push_next(&mut hardware_buffer_format_properties)
            .build();

        {
            let ext_fns =
                vk::AndroidExternalMemoryAndroidHardwareBufferFn::load(|name: &std::ffi::CStr| {
                    mem::transmute(
                        graphics_context.raw_instance.get_device_proc_addr(
                            graphics_context.raw_device.handle(),
                            name.as_ptr(),
                        ),
                    )
                });
            ext_fns.get_android_hardware_buffer_properties_android(
                graphics_context.raw_device.handle(),
                input_buffer_ptr as _,
                &mut hardware_buffer_properties as _,
            );
        }

        // error!("buffer properties: {:?}", hardware_buffer_format_properties);

        let conversion = trace_err!(graphics_context.raw_device.create_sampler_ycbcr_conversion(
            &vk::SamplerYcbcrConversionCreateInfo::builder()
                .format(hardware_buffer_format_properties.format)
                .ycbcr_model(hardware_buffer_format_properties.suggested_ycbcr_model)
                .ycbcr_range(hardware_buffer_format_properties.suggested_ycbcr_range)
                .components(hardware_buffer_format_properties.sampler_ycbcr_conversion_components)
                .x_chroma_offset(hardware_buffer_format_properties.suggested_x_chroma_offset)
                .y_chroma_offset(hardware_buffer_format_properties.suggested_y_chroma_offset)
                .chroma_filter(vk::Filter::LINEAR),
            None
        ))?;

        let sampler = trace_err!(graphics_context.raw_device.create_sampler(
            &vk::SamplerCreateInfo::builder()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .min_lod(0.0)
                .max_lod(1.0)
                .push_next(&mut vk::SamplerYcbcrConversionInfo::builder().conversion(conversion)),
            None,
        ))?;

        let input_image = trace_err!(graphics_context.raw_device.create_image(
            &vk::ImageCreateInfo::builder()
                // .flags(DISJOINT)
                .image_type(vk::ImageType::TYPE_2D)
                .format(hardware_buffer_format_properties.format)
                .extent(vk::Extent3D {
                    width: input_size.x,
                    height: input_size.y,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::LINEAR)
                .usage(vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                // .initial_layout(vk::ImageLayout::PREINITIALIZED)
                .push_next(
                    &mut vk::ExternalFormatANDROID::builder()
                        .external_format(hardware_buffer_format_properties.external_format),
                ),
            None,
        ))?;

        let input_image_view = trace_err!(graphics_context.raw_device.create_image_view(
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
                .push_next(&mut vk::SamplerYcbcrConversionInfo::builder().conversion(conversion)),
            None
        ))?;

        let render_pass = trace_err!(graphics_context.raw_device.create_render_pass(
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
        ))?;

        let descriptor_set_layout =
            trace_err!(graphics_context.raw_device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[]),
                None
            ))?;

        let pipeline_layout = trace_err!(graphics_context.raw_device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder().set_layouts(&[descriptor_set_layout]),
            None,
        ))?;

        // let pipeline = graphics_context.raw_device.pipeline

        let mut output_image = vk::Image::null();
        output_texture.as_hal::<hal::api::Vulkan, _>(|tex| {
            output_image = tex.unwrap().raw_handle();
        });

        let output_image_view = trace_err!(graphics_context.raw_device.create_image_view(
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

        let framebuffer = trace_err!(graphics_context.raw_device.create_framebuffer(
            &vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .width(output_size.x)
                .height(output_size.y)
                .attachments(&[output_image_view])
                .layers(1),
            None,
        ))?;

        let fence = trace_err!(graphics_context.raw_device.create_fence(
            &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
            None
        ))?;

        error!("conversion context created");

        Ok(Self {
            graphics_context,
            input_image,
            input_image_view,
            input_memory: vk::DeviceMemory::null(),
            input_allocation_size: hardware_buffer_properties.allocation_size,
            sampler,
            descriptor_set_layout,
            pipeline_layout,
            pipeline: todo!(),
            command_pool: todo!(),
            output_image_view,
            framebuffer,
            fence,
        })
    }

    unsafe fn update_input_memory(&mut self, buffer_ptr: *mut sys::AHardwareBuffer) {
        // Create memory from external buffer
        let allocate_info = vk::MemoryDedicatedAllocateInfo::builder()
            .image(self.input_image)
            .build();
        let mut hardware_buffer_info = vk::ImportAndroidHardwareBufferInfoANDROID::builder()
            .buffer(buffer_ptr as _)
            .build();
        hardware_buffer_info.p_next = &allocate_info as *const _ as _;
        self.input_memory = self
            .graphics_context
            .raw_device
            .allocate_memory(
                &vk::MemoryAllocateInfo::builder()
                    .allocation_size(self.input_allocation_size)
                    .memory_type_index(1) // memory_type_bits must be 1 << 1 -> 2
                    .push_next(&mut hardware_buffer_info),
                None,
            )
            .unwrap();

        self.graphics_context
            .raw_device
            .bind_image_memory(self.input_image, self.input_memory, 0);
    }
}

impl Drop for ConversionPass {
    fn drop(&mut self) {
        unsafe {
            self.graphics_context
                .raw_device
                .destroy_fence(self.fence, None);

            self.graphics_context
                .raw_device
                .destroy_framebuffer(self.framebuffer, None);
            self.graphics_context
                .raw_device
                .destroy_image_view(self.output_image_view, None);

            self.graphics_context
                .raw_device
                .destroy_image(self.input_image, None);
            self.graphics_context
                .raw_device
                .free_memory(self.input_memory, None);
        }
    }
}

pub struct MediaCodec {
    inner: *mut sys::AMediaCodec,
}

unsafe impl Send for MediaCodec {}
unsafe impl Sync for MediaCodec {}

impl Drop for MediaCodec {
    fn drop(&mut self) {
        unsafe { sys::AMediaCodec_delete(self.inner) };
    }
}

pub struct VideoDecoderEnqueuer {
    inner: Arc<MediaCodec>,
}

impl VideoDecoderEnqueuer {
    // Block until the buffer has been written or timeout is reached. Returns false if timeout.
    pub fn push_frame_nals(
        &self,
        timestamp: Duration,
        data: &[u8],
        timeout: Duration,
    ) -> StrResult<bool> {
        let index_or_error = unsafe {
            sys::AMediaCodec_dequeueInputBuffer(self.inner.inner, timeout.as_micros() as _)
        };
        if index_or_error >= 0 {
            unsafe {
                // todo: check for overflow
                let mut _out_size = 0;
                let buffer_ptr = sys::AMediaCodec_getInputBuffer(
                    self.inner.inner,
                    index_or_error as _,
                    &mut _out_size,
                );
                ptr::copy_nonoverlapping(data.as_ptr(), buffer_ptr, data.len());

                // NB: the function expects the timestamp in micros, but nanos is used to have
                // complete precision, so when converted back to Duration it can compare correctly
                // to other Durations
                sys::AMediaCodec_queueInputBuffer(
                    self.inner.inner,
                    index_or_error as _,
                    0,
                    data.len() as _,
                    timestamp.as_nanos() as _,
                    0,
                );
            }

            Ok(true)
        } else if index_or_error as i32 == sys::AMEDIACODEC_INFO_TRY_AGAIN_LATER {
            Ok(false)
        } else {
            return fmt_e!("Error dequeueing decoder input ({})", index_or_error);
        }
    }
}

pub struct VideoDecoderDequeuer {
    inner: Arc<MediaCodec>,
}

impl VideoDecoderDequeuer {
    pub fn poll(&self, timeout: Duration) -> StrResult {
        let mut info: sys::AMediaCodecBufferInfo = unsafe { std::mem::zeroed() }; // todo: derive default
        let index_or_error = unsafe {
            sys::AMediaCodec_dequeueOutputBuffer(
                self.inner.inner,
                &mut info,
                timeout.as_micros() as _,
            )
        };
        if index_or_error >= 0 {
            let res = unsafe {
                sys::AMediaCodec_releaseOutputBuffer(self.inner.inner, index_or_error as _, true)
            };
            if res != 0 {
                return fmt_e!("Error releasing decoder output buffer ({})", res);
            } else {
                Ok(())
            }
        } else if index_or_error as i32 == sys::AMEDIACODEC_INFO_TRY_AGAIN_LATER {
            Ok(())
        } else {
            return fmt_e!("Error dequeueing decoder output ({})", index_or_error);
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

        image.get_width();

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

        unsafe { conversion_pass.update_input_memory(hardware_buffer.as_ptr()) };

        // error!("swapchain images count: {:?}");

        // let mut encoder = self
        //     .graphics_context
        //     .device
        //     .create_command_encoder(&Default::default());

        // // Copy surface/OES texture to normal texture
        // encoder.copy_texture_to_texture(
        //     ImageCopyTexture {
        //         texture: source,
        //         mip_level: 0,
        //         origin: Origin3d::ZERO,
        //         aspect: TextureAspect::All,
        //     },
        //     ImageCopyTexture {
        //         texture: &output,
        //         mip_level: 0,
        //         origin: Origin3d {
        //             x: 0,
        //             y: 0,
        //             z: slice_index,
        //         },
        //         aspect: TextureAspect::All,
        //     },
        //     Extent3d {
        //         width: self.video_size.x,
        //         height: self.video_size.y,
        //         depth_or_array_layers: 1,
        //     },
        // );

        // self.graphics_context.queue.submit(Some(encoder.finish()));

        Ok(Duration::from_nanos(trace_err!(image.get_timestamp())? as _))
    }
}

pub fn split(
    graphics_context: Arc<GraphicsContext>,
    codec_type: CodecType,
    csd_0: Vec<u8>,
    extra_options: &[(String, MediacodecDataType)],
    output_texture: Arc<Texture>,
    output_size: UVec2,
    slice_index: u32,
) -> StrResult<(
    VideoDecoderEnqueuer,
    VideoDecoderDequeuer,
    VideoDecoderFrameGrabber,
)> {
    log::error!("create video decoder");

    let mut swapchain = trace_err!(ImageReader::new_with_usage(
        1,
        1,
        ImageFormat::YUV_420_888,
        HardwareBufferUsage::GPU_SAMPLED_IMAGE,
        3, // to avoid a deadlock, a triple buffered swapchain is required
    ))?;

    let (image_sender, image_receiver) = crossbeam_channel::unbounded();

    swapchain.set_image_listener(Box::new(move |swapchain| {
        let maybe_image = swapchain.acquire_next_image();
        error!("maybe acquired image: {:?}", maybe_image);

        if let Some(image) = maybe_image.ok().flatten() {
            image_sender.send(image).ok();
        }
    }));

    let surface_handle = trace_err!(swapchain.get_window())?.ptr().as_ptr();

    let decoder = unsafe {
        let mime = match codec_type {
            CodecType::H264 => "video/avc",
            CodecType::HEVC => "video/hevc",
        };
        let mime_cstring = CString::new(mime).unwrap();

        let format = sys::AMediaFormat_new();
        sys::AMediaFormat_setString(format, sys::AMEDIAFORMAT_KEY_MIME, mime_cstring.as_ptr());
        sys::AMediaFormat_setInt32(format, sys::AMEDIAFORMAT_KEY_WIDTH, 512);
        sys::AMediaFormat_setInt32(format, sys::AMEDIAFORMAT_KEY_HEIGHT, 1024);
        sys::AMediaFormat_setBuffer(
            format,
            sys::AMEDIAFORMAT_KEY_CSD_0,
            csd_0.as_ptr() as _,
            csd_0.len() as _,
        );

        // Note: string keys and values are memcpy-ed internally into AMediaFormat. CString is
        // only needed to add the trailing null character.
        for (key, value) in extra_options {
            let key_cstring = CString::new(key.clone()).unwrap();

            match value {
                MediacodecDataType::Float(value) => {
                    sys::AMediaFormat_setFloat(format, key_cstring.as_ptr(), *value)
                }
                MediacodecDataType::Int32(value) => {
                    sys::AMediaFormat_setInt32(format, key_cstring.as_ptr(), *value)
                }
                MediacodecDataType::Int64(value) => {
                    sys::AMediaFormat_setInt64(format, key_cstring.as_ptr(), *value)
                }
                MediacodecDataType::String(value) => {
                    let value_cstring = CString::new(value.clone()).unwrap();
                    sys::AMediaFormat_setString(
                        format,
                        key_cstring.as_ptr(),
                        value_cstring.as_ptr(),
                    )
                }
            }
        }

        let decoder = sys::AMediaCodec_createDecoderByType(mime_cstring.as_ptr());
        if decoder.is_null() {
            return fmt_e!("Decoder is null");
        }

        let res = sys::AMediaCodec_configure(decoder, format, surface_handle, ptr::null_mut(), 0);
        if res != 0 {
            return fmt_e!("Error configuring decoder ({})", res);
        }

        let res = sys::AMediaCodec_start(decoder);
        if res != 0 {
            return fmt_e!("Error starting decoder ({})", res);
        }

        let res = sys::AMediaFormat_delete(format);
        if res != 0 {
            error!("Error deleting format ({})", res);
        }

        log::error!("video decoder created");

        MediaCodec { inner: decoder }
    };

    let decoder = Arc::new(decoder);

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
