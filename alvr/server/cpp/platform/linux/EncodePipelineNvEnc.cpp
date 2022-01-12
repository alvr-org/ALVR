#include "EncodePipelineNvEnc.h"
#include "ALVR-common/packet_types.h"
#include "alvr_server/Settings.h"
#include "ffmpeg_helper.h"
#include <chrono>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavutil/opt.h>
#include <libswscale/swscale.h>
}

namespace {

const char *encoder(ALVR_CODEC codec) {
    switch (codec) {
    case ALVR_CODEC_H264:
        return "h264_nvenc";
    case ALVR_CODEC_H265:
        return "hevc_nvenc";
    }
    throw std::runtime_error("invalid codec " + std::to_string(codec));
}

} // namespace
alvr::EncodePipelineNvEnc::EncodePipelineNvEnc(std::vector<VkFrame> &input_frames,
                                               VkFrameCtx &vk_frame_ctx) {
    auto input_frame_ctx = (AVHWFramesContext *)vk_frame_ctx.ctx->data;
    assert(input_frame_ctx->sw_format == AV_PIX_FMT_BGRA);

    int err;
    for (auto &input_frame : input_frames) {
        vk_frames.push_back(std::move(input_frame.make_av_frame(vk_frame_ctx)));
    }

    const auto &settings = Settings::Instance();

    auto codec_id = ALVR_CODEC(settings.m_codec);
    const char *encoder_name = encoder(codec_id);
    AVCodec *codec = AVCODEC.avcodec_find_encoder_by_name(encoder_name);
    if (codec == nullptr) {
        throw std::runtime_error(std::string("Failed to find encoder ") + encoder_name);
    }

    encoder_ctx = AVCODEC.avcodec_alloc_context3(codec);
    if (not encoder_ctx) {
        throw std::runtime_error("failed to allocate NvEnc encoder");
    }

    switch (codec_id) {
    case ALVR_CODEC_H264:
        AVUTIL.av_opt_set(encoder_ctx, "preset", "llhq", 0);
        AVUTIL.av_opt_set(encoder_ctx, "zerolatency", "1", 0);
        break;
    case ALVR_CODEC_H265:
        AVUTIL.av_opt_set(encoder_ctx, "preset", "llhq", 0);
        AVUTIL.av_opt_set(encoder_ctx, "zerolatency", "1", 0);
        break;
    }

    /**
     * We will recieve a frame from HW as AV_PIX_FMT_VULKAN which will converted to AV_PIX_FMT_BGRA
     * as SW format when we get it from HW.
     * But NVEnc support only BGR0 format and we easy can just to force it
     * Because:
     * AV_PIX_FMT_BGRA - 28  ///< packed BGRA 8:8:8:8, 32bpp, BGRABGRA...
     * AV_PIX_FMT_BGR0 - 123 ///< packed BGR 8:8:8,    32bpp, BGRXBGRX...   X=unused/undefined
     *
     * We just to ignore the alpha channel and it's done
     */
    encoder_ctx->pix_fmt = AV_PIX_FMT_BGR0;
    encoder_ctx->width = settings.m_renderWidth;
    encoder_ctx->height = settings.m_renderHeight;
    encoder_ctx->time_base = {std::chrono::steady_clock::period::num,
                              std::chrono::steady_clock::period::den};
    encoder_ctx->framerate = AVRational{settings.m_refreshRate, 1};
    encoder_ctx->sample_aspect_ratio = AVRational{1, 1};
    encoder_ctx->max_b_frames = 0;
    encoder_ctx->gop_size = 30;
    encoder_ctx->bit_rate = settings.mEncodeBitrateMBs * 1000 * 1000;

    err = AVCODEC.avcodec_open2(encoder_ctx, codec, NULL);
    if (err < 0) {
        throw alvr::AvException("Cannot open video encoder codec:", err);
    }

    hw_frame = AVUTIL.av_frame_alloc();
}

alvr::EncodePipelineNvEnc::~EncodePipelineNvEnc() {
    AVUTIL.av_buffer_unref(&hw_ctx);
    AVUTIL.av_frame_free(&hw_frame);
}

void alvr::EncodePipelineNvEnc::PushFrame(uint32_t frame_index, bool idr) {
    assert(frame_index < vk_frames.size());

    int err = AVUTIL.av_hwframe_transfer_data(hw_frame, vk_frames[frame_index].get(), 0);
    if (err) {
        throw alvr::AvException("av_hwframe_transfer_data", err);
    }

    hw_frame->pict_type = idr ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
    hw_frame->pts = std::chrono::steady_clock::now().time_since_epoch().count();

    if ((err = AVCODEC.avcodec_send_frame(encoder_ctx, hw_frame)) < 0) {
        throw alvr::AvException("avcodec_send_frame failed:", err);
    }
}
