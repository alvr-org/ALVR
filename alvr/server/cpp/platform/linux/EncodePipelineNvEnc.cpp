#include "EncodePipelineNvEnc.h"
#include "ALVR-common/packet_types.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Logger.h"
#include "ffmpeg_helper.h"
#include <chrono>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavutil/opt.h>
}

namespace {

const char *encoder(ALVR_CODEC codec) {
    switch (codec) {
    case ALVR_CODEC_H264:
        return "h264_nvenc";
    case ALVR_CODEC_HEVC:
        return "hevc_nvenc";
    case ALVR_CODEC_AV1:
        return "av1_nvenc";
    }
    throw std::runtime_error("invalid codec " + std::to_string(codec));
}

void set_hwframe_ctx(AVCodecContext *ctx, AVBufferRef *hw_device_ctx)
{
  AVBufferRef *hw_frames_ref;
  AVHWFramesContext *frames_ctx = NULL;
  int err = 0;

  if (!(hw_frames_ref = av_hwframe_ctx_alloc(hw_device_ctx))) {
    throw std::runtime_error("Failed to create CUDA frame context.");
  }
  frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
  frames_ctx->format = AV_PIX_FMT_CUDA;
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
  frames_ctx->sw_format = AV_PIX_FMT_BGR0;
  frames_ctx->width = ctx->width;
  frames_ctx->height = ctx->height;
  if ((err = av_hwframe_ctx_init(hw_frames_ref)) < 0) {
    av_buffer_unref(&hw_frames_ref);
    throw alvr::AvException("Failed to initialize CUDA frame context:", err);
  }
  ctx->hw_frames_ctx = av_buffer_ref(hw_frames_ref);
  if (!ctx->hw_frames_ctx)
    err = AVERROR(ENOMEM);

  av_buffer_unref(&hw_frames_ref);
}

} // namespace
alvr::EncodePipelineNvEnc::EncodePipelineNvEnc(Renderer *render,
                                               VkContext &vk_ctx,
                                               VkFrame &input_frame,
                                               VkFrameCtx &vk_frame_ctx,
                                               uint32_t width,
                                               uint32_t height) {
    r = render;
    auto input_frame_ctx = (AVHWFramesContext *)vk_frame_ctx.ctx->data;
    assert(input_frame_ctx->sw_format == AV_PIX_FMT_BGRA);

    int err;
    vk_frame = input_frame.make_av_frame(vk_frame_ctx);

    err = av_hwdevice_ctx_create_derived(&hw_ctx, AV_HWDEVICE_TYPE_CUDA, vk_ctx.ctx, 0);
    if (err < 0) {
        throw alvr::AvException("Failed to create a CUDA device:", err);
    }

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
    case ALVR_CODEC_HEVC:
        break;
    case ALVR_CODEC_AV1:
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

    switch (settings.m_h264Profile) {
    case ALVR_H264_PROFILE_BASELINE:
        av_opt_set(encoder_ctx->priv_data, "profile", "baseline", 0);
        break;
    case ALVR_H264_PROFILE_MAIN:
        av_opt_set(encoder_ctx->priv_data, "profile", "main", 0);
        break;
    default:
    case ALVR_H264_PROFILE_HIGH:
        av_opt_set(encoder_ctx->priv_data, "profile", "high", 0);
        break;
    }

    char preset[] = "p0";
    // replace 0 with preset number
    preset[1] += settings.m_nvencQualityPreset;
    av_opt_set(encoder_ctx->priv_data, "preset", preset, 0);

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
    // Delay isn't actually a delay instead its how many surfaces to encode at a time
    av_opt_set_int(encoder_ctx->priv_data, "delay", 1, 0);
    av_opt_set_int(encoder_ctx->priv_data, "forced-idr", 1, 0);

    encoder_ctx->pix_fmt = AV_PIX_FMT_CUDA;
    encoder_ctx->width = width;
    encoder_ctx->height = height;
    encoder_ctx->time_base = {1, (int)1e9};
    encoder_ctx->framerate = AVRational{settings.m_refreshRate, 1};
    encoder_ctx->sample_aspect_ratio = AVRational{1, 1};
    encoder_ctx->max_b_frames = 0;
    encoder_ctx->gop_size = INT16_MAX;
    encoder_ctx->color_range = Settings::Instance().m_useFullRangeEncoding ? AVCOL_RANGE_JPEG : AVCOL_RANGE_MPEG;
    auto params = FfiDynamicEncoderParams {};
    params.updated = true;
    params.bitrate_bps = 30'000'000;
    params.framerate = 60.0;
    SetParams(params);

    set_hwframe_ctx(encoder_ctx, hw_ctx);

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
    AVVkFrame *vkf = reinterpret_cast<AVVkFrame*>(vk_frame->data[0]);
    vkf->sem_value[0]++;

    VkTimelineSemaphoreSubmitInfo timelineInfo = {};
    timelineInfo.sType = VK_STRUCTURE_TYPE_TIMELINE_SEMAPHORE_SUBMIT_INFO;
    timelineInfo.signalSemaphoreValueCount = 1;
    timelineInfo.pSignalSemaphoreValues = &vkf->sem_value[0];

    VkPipelineStageFlags waitStage = VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT;

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.pNext = &timelineInfo;
    submitInfo.waitSemaphoreCount = 1;
    submitInfo.pWaitSemaphores = &r->GetOutput().semaphore;
    submitInfo.pWaitDstStageMask = &waitStage;
    submitInfo.signalSemaphoreCount = 1;
    submitInfo.pSignalSemaphores = &vkf->sem[0];
    VK_CHECK(vkQueueSubmit(r->m_queue, 1, &submitInfo, nullptr));

    int err = av_hwframe_get_buffer(encoder_ctx->hw_frames_ctx, hw_frame, 0);
    if (err < 0) {
        throw alvr::AvException("Failed to allocate CUDA frame", err);
    }
    err = av_hwframe_transfer_data(hw_frame, vk_frame.get(), 0);
    if (err < 0) {
        throw alvr::AvException("Failed to transfer Vulkan image to CUDA frame", err);
    }

    hw_frame->pict_type = idr ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
    hw_frame->pts = targetTimestampNs;

    if ((err = avcodec_send_frame(encoder_ctx, hw_frame)) < 0) {
        throw alvr::AvException("avcodec_send_frame failed:", err);
    }

    av_frame_unref(hw_frame);
}
