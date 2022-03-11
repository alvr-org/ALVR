use alvr_common::{glam::UVec2, prelude::*};
use alvr_graphics::{ash::vk, wgpu::Texture, GraphicsContext};
use alvr_session::CodecType;
use std::{ffi::CStr, mem, ptr, sync::Arc};

// settings:
// VkVideoCodingQualityPresetFlagBitsKHR

const INPUT_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

pub struct VulkanEncoder {
    graphics_context: Arc<GraphicsContext>,
    queue: vk::Queue,
    video_queue_fns: vk::KhrVideoQueueFn,
    video_encode_queue_fns: vk::KhrVideoEncodeQueueFn,
    video_session: vk::VideoSessionKHR,
    video_session_parameters: vk::VideoSessionParametersKHR,
    command_buffers: Vec<vk::CommandBuffer>,
    quality_level: u32,
    coded_extent: vk::Extent2D,
    image_view: vk::ImageView,
    bitstream_buffer: vk::Buffer,
    bitstream_buffer_size: vk::DeviceSize,
}

impl VulkanEncoder {
    pub unsafe fn new(
        graphics_context: Arc<GraphicsContext>,

        size: UVec2,
        codec: CodecType,
        config: VulkanEncoderConfig,
        queue_family_index: u32,
        queue_index: u32,
    ) -> StrResult<Self> {
        let device = &graphics_context.raw.device;

        let codec_operation;
        match codec {
            CodecType::H264 => {
                codec_operation = vk::VideoCodecOperationFlagsKHR::ENCODE_H264_EXT;
            }
            CodecType::HEVC => {
                codec_operation = vk::VideoCodecOperationFlagsKHR::ENCODE_H265_EXT;
            }
        };

        let luma_bits = match config.luma_bits {
            BitDepth::Bit8 => vk::VideoComponentBitDepthFlagsKHR::TYPE_8,
            BitDepth::Bit10 => vk::VideoComponentBitDepthFlagsKHR::TYPE_10,
        };

        let queue = device.get_device_queue(
            graphics_context.raw.rendering_queue_family_index,
            graphics_context.raw.rendering_queue_index,
        );

        let get_proc_addr_cb = |name: &CStr| {
            mem::transmute(
                graphics_context
                    .raw.instance
                    .get_device_proc_addr(device.handle(), name.as_ptr()),
            )
        };
        let video_queue_fns = vk::KhrVideoQueueFn::load(get_proc_addr_cb);
        let video_encode_queue_fns = vk::KhrVideoEncodeQueueFn::load(get_proc_addr_cb);

        let coded_extent = vk::Extent2D {
            width: size.x,
            height: size.y,
        };

        let queue_family_props = {
            let size = graphics_context
                .raw.instance
                .get_physical_device_queue_family_properties2_len(
                    graphics_context.raw.physical_device,
                );
                
            let mut queue_family_props = vec![vk::QueueFamilyProperties2::default(); size];
            graphics_context
                .raw.instance
                .get_physical_device_queue_family_properties2(
                    graphics_context.raw.physical_device,
                    &mut queue_family_props,
                );

            queue_family_props
        };

        let video_session = {
            let mut video_session = vk::VideoSessionKHR::null();

            let video_profile = vk::VideoProfileKHR::builder()
                .video_codec_operation(codec_operation)
                .chroma_subsampling(vk::VideoChromaSubsamplingFlagsKHR::TYPE_420)
                .luma_bit_depth(luma_bits)
                .chroma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_8);
            let video_session_create_info = vk::VideoSessionCreateInfoKHR::builder()
                .queue_family_index(0) // todo create queue
                .video_profile(&video_profile)
                .picture_format(INPUT_FORMAT)
                .max_coded_extent(coded_extent)
                .reference_pictures_format(INPUT_FORMAT) // IDR. Only decoder?
                .max_reference_pictures_slots_count(1) // Only decoder?
                .max_reference_pictures_active_count(1) // Only decoder?
                .build();
            video_queue_fns
                .create_video_session_khr(
                    device.handle(),
                    &video_session_create_info,
                    ptr::null(),
                    &mut video_session,
                )
                .result()
                .map_err(err!())?;

            video_session
        };

        todo!()
    }

    pub unsafe fn encode(&mut self, texture: Texture, semaphore: vk::Semaphore) -> &[u8] {
        let device = &self.graphics_context.raw.device;

        let command_buffer = self.command_buffers[0];

        let video_begin_coding_info = vk::VideoBeginCodingInfoKHR::builder()
            .codec_quality_preset(vk::VideoCodingQualityPresetFlagsKHR::NORMAL)
            .video_session(self.video_session)
            .video_session_parameters(self.video_session_parameters)
            // .reference_slots(reference_slots)
            .build();
        self.video_queue_fns
            .cmd_begin_video_coding_khr(command_buffer, &video_begin_coding_info);

        let video_encode_info = vk::VideoEncodeInfoKHR::builder()
            .quality_level(self.quality_level)
            .coded_extent(self.coded_extent)
            .dst_bitstream_buffer(self.bitstream_buffer)
            .dst_bitstream_buffer_max_range(self.bitstream_buffer_size)
            .src_picture_resource(
                vk::VideoPictureResourceKHR::builder()
                    .coded_extent(self.coded_extent)
                    .base_array_layer(0)
                    .image_view_binding(self.image_view)
                    .build(),
            )
            // .setup_reference_slot()
            // .reference_slots()
            // .preceding_externally_encoded_bytes() // set when enabled rate control
            .build();
        self.video_encode_queue_fns
            .cmd_encode_video_khr(command_buffer, &video_encode_info);

        let video_coding_control_info = vk::VideoCodingControlInfoKHR::builder()
            .flags(vk::VideoCodingControlFlagsKHR::DEFAULT) // todo: set this to RESET to get the config data
            .build();
        self.video_queue_fns
            .cmd_control_video_coding_khr(command_buffer, &video_coding_control_info);

        let video_end_coding_info = vk::VideoEndCodingInfoKHR::default();
        self.video_queue_fns
            .cmd_end_video_coding_khr(command_buffer, &video_end_coding_info);

        todo!()
    }
}

impl Drop for VulkanEncoder {
    fn drop(&mut self) {
        todo!()
    }
}
