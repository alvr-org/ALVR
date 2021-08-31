use alvr_common::prelude::*;
use alvr_session::{CodecType, MediacodecDataType};
use graphics_tests::Context;
use ndk::{
    hardware_buffer::HardwareBufferUsage,
    media::image_reader::{ImageFormat, ImageReader},
};
use ndk_sys as sys;
use raw_window_handle::{android::AndroidHandle, HasRawWindowHandle, RawWindowHandle};
use std::{ffi::CString, ptr, sync::Arc, time::Duration};
use wgpu::{
    CommandEncoder, Device, Extent3d, ImageCopyTexture, Origin3d, Surface, TextureAspect,
    TextureView,
};

pub struct SurfaceHandle(*const sys::ANativeWindow);

impl HasRawWindowHandle for SurfaceHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Android(AndroidHandle {
            a_native_window: self.0,
            ..AndroidHandle::empty()
        })
    }
}

pub struct VideoDecoder {
    context: Arc<Context>,
    codec: *mut AMediaCodec,
    swapchain: ImageReader,
    swapchain_surface: Surface,
    video_size: (u32, u32),
}

impl VideoDecoder {
    pub fn new(
        context: Arc<Context>,
        codec_type: CodecType,
        video_size: (u32, u32),
        extra_options: &[(String, MediacodecDataType)],
    ) -> StrResult<Self> {
        let swapchain = trace_err!(ImageReader::new_with_usage(
            video_size.0 as _,
            video_size.1 as _,
            ImageFormat::RGBX_8888,
            HardwareBufferUsage::GPU_SAMPLED_IMAGE,
            2, // double buffered
        ))?;

        let surface_handle = trace_err!(swapchain.get_window())?.ptr().as_ptr();

        let swapchain_surface = unsafe {
            context
                .instance()
                .create_surface(SurfaceHandle(surface_handle))
        };

        let mime = match codec_type {
            CodecType::H264 => "video/avc",
            CodecType::HEVC => "video/hevc",
        };
        let mime_cstring = CString::new(mime).unwrap();

        unsafe {
            let codec = sys::AMediaCodec_createDecoderByType(mime_cstring.as_ptr());

            let format = sys::AMediaFormat_new();
            sys::AMediaFormat_setString(format, sys::AMEDIAFORMAT_KEY_MIME, mime_cstring.as_ptr());
            sys::AMediaFormat_setInt32(format, sys::AMEDIAFORMAT_KEY_WIDTH, video_size.0 as _);
            sys::AMediaFormat_setInt32(format, sys::AMEDIAFORMAT_KEY_HEIGHT, video_size.1 as _);

            // Note: string keys and values are memcpy-ed internally into AMediaFormat. CString is
            // only needed to add the trailing null character.
            for (key, value) in extra_options {
                let key_cstring = CString::new(key).unwrap();

                match value {
                    MediacodecDataType::Float(value) => {
                        sys::AMediaFormat_setFloat(format, key_cstring.as_ptr(), value)
                    }
                    MediacodecDataType::Int32(value) => {
                        sys::AMediaFormat_setInt32(format, key_cstring.as_ptr(), value)
                    }
                    MediacodecDataType::Int64(value) => {
                        sys::AMediaFormat_setInt64(format, key_cstring.as_ptr(), value)
                    }
                    MediacodecDataType::String(value) => {
                        let value_cstring = CString::new(value).unwrap();
                        sys::AMediaFormat_setString(
                            format,
                            key_cstring.as_ptr(),
                            value_cstring.as_ptr(),
                        )
                    }
                }
            }

            let res = sys::AMediaCodec_configure(codec, format, surface_handle, ptr::null_mut(), 0);
            if res != 0 {
                return fmt_e!("Error configuring decoder ({})", res);
            }

            let res = sys::AMediaCodec_start(codec);
            if res != 0 {
                return fmt_e!("Error starting decoder ({})", res);
            }

            let res = sys::AMediaFormat_delete(format);
            if res != 0 {
                error!("Error deleting format ({})", res);
            }

            Ok(Self {
                context,
                codec,
                swapchain,
                swapchain_surface,
                video_size,
            })
        }
    }

    // Block until the buffer has been written or timeout is reached. Returns false if timeout.
    pub fn push_frame_nals(
        &self,
        frame_index: usize,
        data: &[u8],
        timeout: Duration,
    ) -> StrResult<bool> {
        let index_or_error =
            unsafe { sys::AMediaCodec_dequeueInputBuffer(self.codec, timeout.as_micros() as _) };
        if index_or_error >= 0 {
            unsafe {
                let buffer_ptr =
                    sys::AMediaCodec_getInputBuffer(self.codec, index_or_error, out_size);
                ptr::copy_nonoverlapping(data.as_ptr(), buffer_ptr, data.len());

                sys::AMediaCodec_queueInputBuffer(
                    self.codec,
                    index_or_error,
                    0,
                    data.len(),
                    frame_index as _, // presentationTimeUs is reinterpreted as the frame index
                    0,
                );
            }
        } else if index_or_error == sys::AMEDIACODEC_INFO_TRY_AGAIN_LATER {
            Ok(false)
        } else {
            return fmt_e!("Error dequeueing decoder input ({})", index_or_error);
        }
    }

    // Block until one frame is available or timeout is reached. Returns the index of the frame
    // (as specified in push_buffer()). Returns None if timeout.
    pub fn get_output_frame(
        &self,
        output: &Texture,
        timeout: Duration,
    ) -> StrResult<Option<usize>> {
        let info: sys::AMediaCodecBufferInfo = unsafe { std::mem::zeroed() }; // todo: derive default
        let index_or_error = unsafe {
            sys::AMediaCodec_dequeueOutputBuffer(self.codec, &mut info, timeout.as_micros() as _)
        };
        if index_or_error >= 0 {
            // Draw to the surface
            if unsafe { sys::AMediaCodec_releaseOutputBuffer(self.codec, index_or_error, true) }
                != 0
            {
                return fmt_e!("Error releasing decoder output buffer ({})", res);
            };

            // Wgpu swapchain can throw Timeout or Outdated, but this should never happen here
            let source = trace_err!(self.swapchain_surface.get_current_frame())?
                .output
                .texture;

            let encoder = self
                .context
                .device()
                .create_command_encoder(&Default::default());

            // Copy surface/OES texture to normal texture. todo: skip this step?
            encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &source,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &output,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: self.video_size.0,
                    height: self.video_size.1,
                    depth_or_array_layers: 1,
                },
            );

            self.context.queue().submit(Some(encoder.finish()));
            pollster::block_on(self.context.queue().on_submitted_work_done());

            // presentationTimeUs is reinterpreted as the frame index
            Ok(Some(info.presentationTimeUs as _))
        } else if index_or_error == sys::AMEDIACODEC_INFO_TRY_AGAIN_LATER {
            Ok(None)
        } else {
            return fmt_e!("Error dequeueing decoder output ({})", index_or_error);
        }
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        let res = unsafe { sys::AMediaCodec_delete(self.codec) };
        if res != 0 {
            error!("Error deleting codec ({})", res);
        }
    }
}
