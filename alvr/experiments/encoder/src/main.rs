mod video_encoder;

use alvr_common::{
    glam::{UVec2, Vec2},
    prelude::*,
};
use alvr_graphics::GraphicsContext;
use alvr_session::{AccelerationType, CodecType, FoveatedRenderingDesc, FrameSize};
use std::{cmp, sync::Arc};
use video_encoder::VideoEncoder;

pub enum SlicesCount {
    // Tries one slice per hardware/software queue.
    Automatic,

    // If there are more slices than vulkan queues, some slices will be encoded sequentially
    Custom(usize),
}

pub struct VideoConstraints {
    pub preferred_frame_size: FrameSize, // NB: this is the resolution of both eye vews combined
    pub recommended_view_size: UVec2,
    pub max_view_size: UVec2,
    pub codec_type: CodecType,
    pub encoder_acceleration_type: AccelerationType,
    pub preferred_slices_count: SlicesCount,
    pub hardware_h264_encoder_queues_count: usize,
    pub hardware_h265_encoder_queues_count: usize,
    pub software_h264_encoder_queues_count: usize,
    pub h264_decoder_queues_count: usize,
    pub h265_decoder_queues_count: usize,
    pub foveated_encoding_config: Option<FoveatedRenderingDesc>,
}

pub struct FoveationVars {
    pub compressed_view_size: UVec2,
    pub center_size: Vec2,
    pub center_shift: Vec2,
    pub edge_ratio: Vec2,
}

pub struct VideoConfig {
    pub target_view_size: UVec2,
    pub foveation_vars: Option<FoveationVars>,
    pub codec_type: CodecType,
    pub encoder_acceleration_type: AccelerationType,
    pub slice_count: usize,
    pub slice_rows: usize,
    pub slice_size: UVec2,
}

// Compute all video-related parameters
// Preferred -> user constraint; recommended -> hardware constraint
// Slices are implicitly ordered as: hardware->software, row-major
pub fn resolve_video_config(constraints: VideoConstraints) -> VideoConfig {
    let preferred_view_size = match constraints.preferred_frame_size {
        FrameSize::Scale(scale) => (constraints.recommended_view_size.as_vec2() * scale).as_uvec2(),
        FrameSize::Absolute { width, height } => UVec2::new(width / 2, height),
    };

    let target_view_size = UVec2::min(preferred_view_size, constraints.max_view_size);

    if target_view_size != preferred_view_size {
        warn!("Requested resolution ({preferred_view_size}) is too high. Lowering to {target_view_size}")
    }

    let foveation_vars = constraints.foveated_encoding_config.map(|config| {
        let target_view_size = target_view_size.as_vec2();

        let center_size = Vec2::new(config.center_size_x, config.center_size_y);
        let center_shift = Vec2::new(config.center_shift_x, config.center_shift_y);
        let edge_ratio = Vec2::new(config.edge_ratio_x, config.edge_ratio_y);

        let edge_size = target_view_size - center_size * target_view_size;
        let center_size_aligned = 1_f32
            - (edge_size / (edge_ratio * 2_f32)).ceil() * edge_ratio * 2_f32 / target_view_size;

        let edge_size_aligned = target_view_size - center_size * target_view_size;
        let center_shift_aligned =
            (center_shift * edge_size_aligned / (edge_ratio * 2_f32)).ceil() * edge_ratio * 2_f32
                / edge_size_aligned;

        let foveation_scale = center_size_aligned + (1_f32 - center_size_aligned) / edge_ratio;

        let compressed_view_size = (foveation_scale * target_view_size).as_uvec2();

        FoveationVars {
            compressed_view_size,
            center_size,
            center_shift: center_shift_aligned,
            edge_ratio,
        }
    });

    let compressed_view_size = foveation_vars
        .as_ref()
        .map(|vars| vars.compressed_view_size)
        .unwrap_or(target_view_size);

    let combined_size = UVec2::new(compressed_view_size.x * 2, compressed_view_size.y);

    let mut max_slices_count = None;
    let mut codec_type = constraints.codec_type;
    let mut encoder_acceleration_type = constraints.encoder_acceleration_type;

    if matches!(
        constraints.encoder_acceleration_type,
        AccelerationType::Hardware
    ) && matches!(constraints.codec_type, CodecType::HEVC)
    {
        max_slices_count = Some(cmp::max(
            constraints.hardware_h265_encoder_queues_count,
            constraints.h265_decoder_queues_count,
        ));
    }

    if (matches!(
        constraints.encoder_acceleration_type,
        AccelerationType::Hardware
    ) && matches!(constraints.codec_type, CodecType::H264))
        || matches!(max_slices_count, Some(0))
    {
        codec_type = CodecType::H264;

        max_slices_count = Some(cmp::max(
            constraints.hardware_h264_encoder_queues_count,
            constraints.h264_decoder_queues_count,
        ));
    }

    if (matches!(
        constraints.encoder_acceleration_type,
        AccelerationType::Hardware
    ) && matches!(constraints.codec_type, CodecType::H264))
        || matches!(max_slices_count, Some(0))
    {
        encoder_acceleration_type = AccelerationType::Hardware;
        codec_type = CodecType::H264;

        max_slices_count = Some(cmp::max(
            constraints.software_h264_encoder_queues_count,
            constraints.h265_decoder_queues_count,
        ));
    }

    let max_slices_count = max_slices_count.unwrap_or_else(|| {
        warn!("Unexpected codec configuration");
        1
    });

    let slice_count = match constraints.preferred_slices_count {
        SlicesCount::Automatic => max_slices_count,
        SlicesCount::Custom(count) => cmp::min(count, max_slices_count),
    };

    // todo: proper slice size/layout calculation

    let slice_rows = 1;
    // align for encoder
    let slice_size = (combined_size.as_vec2() / 32_f32).ceil().as_uvec2() * 32;

    VideoConfig {
        target_view_size,
        foveation_vars,
        codec_type,
        encoder_acceleration_type,
        slice_count,
        slice_rows,
        slice_size,
    }
}

fn main() {
    // let graphics_context = Arc::new(GraphicsContext::new(None).unwrap());

    // let encoder = unsafe {
    //     VideoEncoder::new(
    //         Arc::clone(&graphics_context),
    //         UVec2::new(10, 10),
    //         VideoEncoderConfig {
    //             hardware: todo!(),
    //             software: todo!(),
    //             slices: todo!(),
    //         },
    //     )
    // };
}
