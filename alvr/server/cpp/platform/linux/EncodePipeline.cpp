#include "EncodePipeline.h"

#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"
#include "EncodePipelineSW.h"
#include "EncodePipelineVAAPI.h"
#include "EncodePipelineNvEnc.h"
#include "ffmpeg_helper.h"

extern "C" {
#include <libavcodec/avcodec.h>
}

void alvr::EncodePipeline::SetParams(FfiDynamicEncoderParams params) {
  if (params.updated) {
    encoder_ctx->bit_rate = params.bitrate_bps / params.framerate * 60.0;
    encoder_ctx->framerate = AVRational{60, 1};
    encoder_ctx->rc_buffer_size = encoder_ctx->bit_rate / 60.0 * 1.1;
    encoder_ctx->rc_max_rate = encoder_ctx->bit_rate;
    encoder_ctx->rc_initial_buffer_occupancy = encoder_ctx->rc_buffer_size / 4 * 3;
  }
}

std::unique_ptr<alvr::EncodePipeline> alvr::EncodePipeline::Create(Renderer *render, VkContext &vk_ctx, VkFrame &input_frame, VkFrameCtx &vk_frame_ctx, uint32_t width, uint32_t height)
{
  if(Settings::Instance().m_force_sw_encoding == false) {
    if (vk_ctx.nvidia) {
      try {
        auto nvenc = std::make_unique<alvr::EncodePipelineNvEnc>(render, vk_ctx, input_frame, vk_frame_ctx, width, height);
        Info("using NvEnc encoder");
        return nvenc;
      } catch (std::exception &e)
      {
        Info("failed to create NvEnc encoder: %s", e.what());
      }
    } else {
      try {
        auto vaapi = std::make_unique<alvr::EncodePipelineVAAPI>(render, vk_ctx, input_frame, width, height);
        Info("using VAAPI encoder");
        return vaapi;
      } catch (std::exception &e)
      {
        Info("failed to create VAAPI encoder: %s", e.what());
      }
    }
  }
  auto sw = std::make_unique<alvr::EncodePipelineSW>(render, width, height);
  Info("using SW encoder");
  return sw;
}

alvr::EncodePipeline::~EncodePipeline()
{
  avcodec_free_context(&encoder_ctx);
}

bool alvr::EncodePipeline::GetEncoded(FramePacket &packet)
{
  av_packet_free(&encoder_packet);
  encoder_packet = av_packet_alloc();
  int err = avcodec_receive_packet(encoder_ctx, encoder_packet);
  if (err != 0) {
    av_packet_free(&encoder_packet);
    if (err == AVERROR(EAGAIN)) {
      return false;
    }
    throw alvr::AvException("failed to encode", err);
  }
  packet.data = encoder_packet->data;
  packet.size = encoder_packet->size;
  packet.pts = encoder_packet->pts;
  packet.isIDR = (encoder_packet->flags & AV_PKT_FLAG_KEY) != 0;
  return true;
}

int alvr::EncodePipeline::GetCodec()
{
  return Settings::Instance().m_codec;
}
