#include "EncodePipelineSW.h"

#include <algorithm>
#include <chrono>

#include "alvr_server/Settings.h"
#include "ffmpeg_helper.h"

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavutil/opt.h>
#include <libswscale/swscale.h>
}

namespace
{

const char * encoder(ALVR_CODEC codec)
{
  switch (codec)
  {
    case ALVR_CODEC_H264:
      return "libx264";
    case ALVR_CODEC_H265:
      return "libx265";
  }
  throw std::runtime_error("invalid codec " + std::to_string(codec));
}


}

alvr::EncodePipelineSW::EncodePipelineSW(std::vector<VkFrame>& input_frames, VkFrameCtx& vk_frame_ctx)
{
  for (auto& input_frame: input_frames)
  {
    vk_frames.push_back(input_frame.make_av_frame(vk_frame_ctx).release());
  }

  const auto& settings = Settings::Instance();

  auto codec_id = ALVR_CODEC(settings.m_codec);
  const char * encoder_name = encoder(codec_id);
  AVCodec *codec = avcodec_find_encoder_by_name(encoder_name);
  if (codec == nullptr)
  {
    throw std::runtime_error(std::string("Failed to find encoder ") + encoder_name);
  }

  encoder_ctx = avcodec_alloc_context3(codec);
  if (not encoder_ctx)
  {
    throw std::runtime_error("failed to allocate " + std::string(encoder_name) + " encoder");
  }

  AVDictionary * opt = NULL;
  switch (codec_id)
  {
    case ALVR_CODEC_H264:
      encoder_ctx->profile = FF_PROFILE_H264_HIGH;
      av_dict_set(&opt, "preset", "ultrafast", 0);
      av_dict_set(&opt, "tune", "zerolatency", 0);
      encoder_ctx->gop_size = 72;
      break;
    case ALVR_CODEC_H265:
      encoder_ctx->profile = FF_PROFILE_HEVC_MAIN;
      av_dict_set(&opt, "preset", "ultrafast", 0);
      av_dict_set(&opt, "tune", "zerolatency", 0);
      encoder_ctx->gop_size = 72;
      break;
  }


  encoder_ctx->width = settings.m_renderWidth;
  encoder_ctx->height = settings.m_renderHeight;
  encoder_ctx->time_base = {std::chrono::steady_clock::period::num, std::chrono::steady_clock::period::den};
  encoder_ctx->framerate = AVRational{settings.m_refreshRate, 1};
  encoder_ctx->sample_aspect_ratio = AVRational{1, 1};
  encoder_ctx->pix_fmt = AV_PIX_FMT_YUV420P;
  encoder_ctx->max_b_frames = 0;
  encoder_ctx->bit_rate = settings.mEncodeBitrateMBs * 1024 * 1024;

  int err = avcodec_open2(encoder_ctx, codec, &opt);
  if (err < 0) {
    throw alvr::AvException("Cannot open video encoder codec:", err);
  }

  transferred_frame = av_frame_alloc();
  encoder_frame = av_frame_alloc();
  encoder_frame->width = settings.m_renderWidth;
  encoder_frame->height = settings.m_renderHeight;
  encoder_frame->format = encoder_ctx->pix_fmt;
  av_frame_get_buffer(encoder_frame, 0);

  scaler_ctx = sws_getContext(
          vk_frames[0]->width, vk_frames[0]->height, ((AVHWFramesContext*)vk_frames[0]->hw_frames_ctx->data)->sw_format,
          encoder_ctx->width, encoder_ctx->height, encoder_ctx->pix_fmt,
          SWS_BILINEAR,
          NULL, NULL, NULL);
}

alvr::EncodePipelineSW::~EncodePipelineSW()
{
  for (auto &vk_frame: vk_frames)
    av_frame_free(&vk_frame);
  av_frame_free(&transferred_frame);
  av_frame_free(&encoder_frame);
}

void alvr::EncodePipelineSW::PushFrame(uint32_t frame_index, bool idr)
{
  int err = av_hwframe_transfer_data(transferred_frame, vk_frames[frame_index], 0);
  if (err)
    throw alvr::AvException("av_hwframe_transfer_data", err);

  err = sws_scale(scaler_ctx, transferred_frame->data, transferred_frame->linesize, 0, transferred_frame->height,
      encoder_frame->data, encoder_frame->linesize);
  if (err == 0)
    throw alvr::AvException("sws_scale failed:", err);

  encoder_frame->pict_type = idr ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
  encoder_frame->pts = std::chrono::steady_clock::now().time_since_epoch().count();

  if ((err = avcodec_send_frame(encoder_ctx, encoder_frame)) < 0) {
    throw alvr::AvException("avcodec_send_frame failed:", err);
  }
}
