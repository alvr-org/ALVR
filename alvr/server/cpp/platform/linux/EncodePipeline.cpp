#include "EncodePipeline.h"

#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"
#include "EncodePipelineSW.h"
#include "EncodePipelineVAAPI.h"
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
}

std::unique_ptr<alvr::EncodePipeline> alvr::EncodePipeline::Create(std::vector<VkFrame> &input_frames, VkFrameCtx &vk_frame_ctx)
{
  try {
    return std::make_unique<alvr::EncodePipelineVAAPI>(input_frames, vk_frame_ctx);
  } catch (...)
  {
    Info("failed to create VAAPI encoder");
  }
  return std::make_unique<alvr::EncodePipelineSW>(input_frames, vk_frame_ctx);
}

alvr::EncodePipeline::~EncodePipeline()
{
  AVCODEC.avcodec_free_context(&encoder_ctx);
}

bool alvr::EncodePipeline::GetEncoded(std::vector<uint8_t> &out)
{
  AVPacket * enc_pkt = AVCODEC.av_packet_alloc();
  int err = AVCODEC.avcodec_receive_packet(encoder_ctx, enc_pkt);
  if (err == AVERROR(EAGAIN)) {
    return false;
  } else if (err) {
    throw alvr::AvException("failed to encode", err);
  }
  filter_NAL(enc_pkt->data, enc_pkt->size, out);
  AVCODEC.av_packet_free(&enc_pkt);
  return true;
}
