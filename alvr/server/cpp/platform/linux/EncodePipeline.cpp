#include "EncodePipeline.h"

#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"
#include "EncodePipelineSW.h"
#include "EncodePipelineVAAPI.h"
#include "EncodePipelineNvEnc.h"
#include "EncodePipelineAMF.h"
#include "ffmpeg_helper.h"

extern "C" {
#include <libavcodec/avcodec.h>
}

namespace {

bool should_keep_nal_h264(const uint8_t * header_start)
{
  uint8_t nal_type = (header_start[2] == 0 ? header_start[4] : header_start[3]) & 0x1F;
    switch (nal_type)
    {
      case 6: // supplemental enhancement information
      case 9: // access unit delimiter
        return false;
      default:
        return true;
    }
}

bool should_keep_nal_h265(const uint8_t * header_start)
{
  uint8_t nal_type = ((header_start[2] == 0 ? header_start[4] : header_start[3]) >> 1) & 0x3F;
  switch (nal_type)
  {
    case 35: // access unit delimiter
    case 39: // supplemental enhancement information
      return false;
    default:
      return true;
  }
}

void filter_NAL(const uint8_t* input, size_t input_size, std::vector<uint8_t> &out)
{
  if (input_size < 4)
    return;
  auto codec = Settings::Instance().m_codec;
  std::array<uint8_t, 3> header = {{0, 0, 1}};
  auto end = input + input_size;
  auto header_start = input;
  while (header_start != end)
  {
    auto next_header = std::search(header_start + 3, end, header.begin(), header.end());
    if (next_header != end and next_header[-1] == 0)
    {
      next_header--;
    }
    if (codec == ALVR_CODEC_H264 and should_keep_nal_h264(header_start))
      out.insert(out.end(), header_start, next_header);
    if (codec == ALVR_CODEC_H265 and should_keep_nal_h265(header_start))
      out.insert(out.end(), header_start, next_header);
    header_start = next_header;
  }
}

}

void alvr::EncodePipeline::SetBitrate(int64_t bitrate) {
  encoder_ctx->bit_rate = bitrate;
  encoder_ctx->rc_buffer_size = bitrate / Settings::Instance().m_refreshRate * 1.1;
  encoder_ctx->rc_max_rate = encoder_ctx->bit_rate;
}

std::unique_ptr<alvr::EncodePipeline> alvr::EncodePipeline::Create(Renderer *render, VkContext &vk_ctx, VkFrame &input_frame, VkFrameCtx &vk_frame_ctx, uint32_t width, uint32_t height)
{
  if(Settings::Instance().m_force_sw_encoding == false) {
    if (vk_ctx.nvidia) {
      try {
        auto nvenc = std::make_unique<alvr::EncodePipelineNvEnc>(render, input_frame, vk_frame_ctx, width, height);
        Info("using NvEnc encoder");
        return nvenc;
      } catch (std::exception &e)
      {
        Info("failed to create NvEnc encoder: %s", e.what());
      }
    } else {
      try {
        auto amf = std::make_unique<alvr::EncodePipelineAMF>(render, width, height);
        Info("using AMF encoder");
        return amf;
      } catch (std::exception &e)
      {
        Info("failed to create AMF encoder: %s", e.what());
      }
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

bool alvr::EncodePipeline::GetEncoded(std::vector<uint8_t> &out, uint64_t *pts)
{
  AVPacket * enc_pkt = av_packet_alloc();
  int err = avcodec_receive_packet(encoder_ctx, enc_pkt);
  if (err == AVERROR(EAGAIN)) {
    return false;
  } else if (err) {
    throw alvr::AvException("failed to encode", err);
  }
  filter_NAL(enc_pkt->data, enc_pkt->size, out);
  *pts = enc_pkt->pts;
  av_packet_free(&enc_pkt);
  return true;
}
