use alvr_common::{glam::UVec2, log, prelude::*};
use alvr_graphics::{
    ash::{self, vk},
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
    ptr::{self, NonNull},
    sync::Arc,
    time::Duration,
};
use sys::AMediaCodec;

struct InnerSwapchainImage {
    device: ash::Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
}

impl Drop for InnerSwapchainImage {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None)
        }
    }
}

// The format used is RGBA8
unsafe fn create_swapchain_texture_from_hardware_buffer(
    graphics_context: &GraphicsContext,
    size: UVec2,
    memory: *mut sys::AHardwareBuffer,
) -> wgpu::Texture {
    let image_create_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(vk::Extent3D {
            width: size.x,
            height: size.y,
            depth: 1,
        })
        .mip_levels(1)
        .usage(vk::ImageUsageFlags::TRANSFER_SRC);
    let image = graphics_context
        .raw_device
        .create_image(&image_create_info, None)
        .unwrap();

    let requirements = graphics_context
        .raw_device
        .get_image_memory_requirements(image);

    let mut hardware_buffer_info = vk::ImportAndroidHardwareBufferInfoANDROID::builder()
        .buffer(memory as _)
        .build();
    let memory_allocate_info = vk::MemoryAllocateInfo::builder()
        .allocation_size((size.x * size.y * 4) as _)
        .memory_type_index(requirements.memory_type_bits)
        .push_next(&mut hardware_buffer_info);
    let memory = graphics_context
        .raw_device
        .allocate_memory(&memory_allocate_info, None)
        .unwrap();

    let hal_texture = <hal::api::Vulkan as hal::Api>::Device::texture_from_raw(
        image,
        &hal::TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: hal::TextureUses::COPY_SRC,
            memory_flags: hal::MemoryFlags::empty(),
        },
        Some(Box::new(InnerSwapchainImage {
            device: graphics_context.raw_device.clone(),
            image,
            memory,
        })),
    );

    graphics_context
        .device
        .create_texture_from_hal::<hal::api::Vulkan>(
            hal_texture,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::COPY_SRC,
            },
        )
}

// 'AndroidSurface failed: ERROR_NATIVE_WINDOW_IN_USE_KHR', /Users/ric/.cargo/registry/src/github.com-1ecc6299db9ec823/wgpu-hal-0.12.0/src/vulkan/instance.rs:331:69

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
    video_size: UVec2,
    swapchain: ImageReader,
    swapchain_textures: HashMap<usize, Texture>,
    image_receiver: crossbeam_channel::Receiver<Image>,
}

unsafe impl Send for VideoDecoderFrameGrabber {}

impl VideoDecoderFrameGrabber {
    // Block until one frame is available or timeout is reached. Returns the frame timestamp (as
    // specified in push_frame_nals()). Returns None if timeout.
    pub fn get_output_frame(
        &mut self,
        output: &Texture,
        slice_index: u32,
        timeout: Duration,
    ) -> StrResult<Duration> {
        let image = trace_err!(self.image_receiver.recv_timeout(timeout))?;

        let hardware_buffer = trace_err!(image.get_hardware_buffer())?;

        let source = self
            .swapchain_textures
            .entry(hardware_buffer.as_ptr() as usize)
            .or_insert_with(|| unsafe {
                create_swapchain_texture_from_hardware_buffer(
                    &self.graphics_context,
                    self.video_size,
                    hardware_buffer.as_ptr(),
                )
            });

        let mut encoder = self
            .graphics_context
            .device
            .create_command_encoder(&Default::default());

        // Copy surface/OES texture to normal texture
        encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: source,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &output,
                mip_level: 0,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: slice_index,
                },
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.video_size.x,
                height: self.video_size.y,
                depth_or_array_layers: 1,
            },
        );

        self.graphics_context.queue.submit(Some(encoder.finish()));

        Ok(Duration::from_nanos(trace_err!(image.get_timestamp())? as _))
    }
}

pub fn split(
    graphics_context: Arc<GraphicsContext>,
    codec_type: CodecType,
    video_size: UVec2,
    csd_0: Vec<u8>,
    extra_options: &[(String, MediacodecDataType)],
) -> StrResult<(
    VideoDecoderEnqueuer,
    VideoDecoderDequeuer,
    VideoDecoderFrameGrabber,
)> {
    log::error!("create video decoder");

    let mut swapchain = trace_err!(ImageReader::new_with_usage(
        // video_size.x as _,
        // video_size.y as _,
        1,
        1,
        // ImageFormat::RGBA_8888,
        ImageFormat::YUV_420_888,
        HardwareBufferUsage::GPU_SAMPLED_IMAGE,
        3, // double buffered
    ))?;

    let (image_sender, image_receiver) = crossbeam_channel::unbounded();

    swapchain.set_image_listener(Box::new(move |swapchain| {
        let maybe_image = show_err(trace_err!(swapchain.acquire_next_image())).flatten();
        error!("maybe acquired image");

        // if let Some(image) = maybe_image {
        //     image_sender.send(image).ok();
        // }
    }));

    // let swapchain = trace_err!(ImageReader::new(1, 1, ImageFormat::YUV_420_888, 2))?;

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
            video_size,
            swapchain,
            swapchain_textures: HashMap::new(),
            image_receiver,
        },
    ))
}
