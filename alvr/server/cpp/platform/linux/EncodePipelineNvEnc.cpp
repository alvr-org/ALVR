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
alvr::EncodePipelineNvEnc::EncodePipelineNvEnc(Renderer *render,
                                               VkFrame &input_frame,
                                               VkFrameCtx &vk_frame_ctx,
                                               uint32_t width,
                                               uint32_t height) {
    r = render;
    auto input_frame_ctx = (AVHWFramesContext *)vk_frame_ctx.ctx->data;
    assert(input_frame_ctx->sw_format == AV_PIX_FMT_BGRA);

    int err;
    vk_frame = std::move(input_frame.make_av_frame(vk_frame_ctx));

    const auto &settings = Settings::Instance();

    auto codec_id = ALVR_CODEC(settings.m_codec);
    const char *encoder_name = encoder(codec_id);
    const AVCodec *codec = avcodec_find_encoder_by_name(encoder_name);
    if (codec == nullptr) {
        throw std::runtime_error(std::string("Failed to find encoder ") + encoder_name);
    }

    encoder_ctx = avcodec_alloc_context3(codec);
    if (not encoder_ctx) {
        throw std::runtime_error("failed to allocate NvEnc encoder");
    }

    switch (codec_id) {
    case ALVR_CODEC_H264:
        switch (settings.m_entropyCoding) {
        case ALVR_CABAC:
            av_opt_set(encoder_ctx->priv_data, "coder", "ac", 0);
            break;
        case ALVR_CAVLC:
            av_opt_set(encoder_ctx->priv_data, "coder", "vlc", 0);
            break;
        }
        break;
    case ALVR_CODEC_H265:
        break;
    }

    switch (settings.m_rateControlMode) {
    case ALVR_CBR:
        av_opt_set(encoder_ctx->priv_data, "rc", "cbr", 0);
        break;
    case ALVR_VBR:
        av_opt_set(encoder_ctx->priv_data, "rc", "vbr", 0);
        break;
    }

    switch (settings.m_encoderQualityPreset) {
    case ALVR_QUALITY:
        av_opt_set(encoder_ctx->priv_data, "preset", "p7", 0);
        break;
    case ALVR_BALANCED:
        av_opt_set(encoder_ctx->priv_data, "preset", "p4", 0);
        break;
    case ALVR_SPEED:
    default:
        av_opt_set(encoder_ctx->priv_data, "preset", "p1", 0);
        break;
    }

    if (settings.m_nvencAdaptiveQuantizationMode == 1) {
        av_opt_set_int(encoder_ctx->priv_data, "spatial_aq", 1, 0);
    } else if (settings.m_nvencAdaptiveQuantizationMode == 2) {
        av_opt_set_int(encoder_ctx->priv_data, "temporal_aq", 1, 0);
    }

    if (settings.m_nvencEnableWeightedPrediction) {
        av_opt_set_int(encoder_ctx->priv_data, "weighted_pred", 1, 0);
    }

    av_opt_set_int(encoder_ctx->priv_data, "tune", settings.m_nvencTuningPreset, 0);
    av_opt_set_int(encoder_ctx->priv_data, "zerolatency", 1, 0);
    av_opt_set_int(encoder_ctx->priv_data, "delay", 0, 0);

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
    encoder_ctx->width = width;
    encoder_ctx->height = height;
    encoder_ctx->time_base = {1, (int)1e9};
    encoder_ctx->framerate = AVRational{settings.m_refreshRate, 1};
    encoder_ctx->sample_aspect_ratio = AVRational{1, 1};
    encoder_ctx->max_b_frames = 0;
    encoder_ctx->gop_size = INT16_MAX;
    SetBitrate(settings.mEncodeBitrateMBs * 1'000'000L);

    err = avcodec_open2(encoder_ctx, codec, NULL);
    if (err < 0) {
        throw alvr::AvException("Cannot open video encoder codec:", err);
    }

    hw_frame = av_frame_alloc();
}

alvr::EncodePipelineNvEnc::~EncodePipelineNvEnc() {
    av_buffer_unref(&hw_ctx);
    av_frame_free(&hw_frame);
}

void alvr::EncodePipelineNvEnc::PushFrame(uint64_t targetTimestampNs, bool idr) {
    r->Sync();
    timestamp.cpu = std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count();
    int err = av_hwframe_transfer_data(hw_frame, vk_frame.get(), 0);
    if (err) {
        throw alvr::AvException("av_hwframe_transfer_data", err);
    }

    hw_frame->pict_type = idr ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
    hw_frame->pts = targetTimestampNs;

    if ((err = avcodec_send_frame(encoder_ctx, hw_frame)) < 0) {
        throw alvr::AvException("avcodec_send_frame failed:", err);
    }
}
