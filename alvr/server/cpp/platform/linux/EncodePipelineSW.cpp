#include "EncodePipelineSW.h"

#include <algorithm>
#include <chrono>

#include "alvr_server/Settings.h"
#include "alvr_server/Logger.h"
#include "ffmpeg_helper.h"
#include "FormatConverter.h"

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavutil/opt.h>
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

alvr::EncodePipelineSW::EncodePipelineSW(Renderer *render, uint32_t width, uint32_t height)
{
  const auto& settings = Settings::Instance();

  if (settings.m_codec == ALVR_CODEC_H265)
  {
    // TODO: Make it work?
    throw std::runtime_error("HEVC is not supported by SW encoder");
  }

  auto codec_id = ALVR_CODEC(settings.m_codec);
  const char * encoder_name = encoder(codec_id);
  const AVCodec *codec = avcodec_find_encoder_by_name(encoder_name);
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
      break;
    case ALVR_CODEC_H265:
      encoder_ctx->profile = settings.m_use10bitEncoder ? FF_PROFILE_HEVC_MAIN_10 : FF_PROFILE_HEVC_MAIN;
      break;
  }

  switch (Settings::Instance().m_rateControlMode)
  {
    case ALVR_CBR:
      av_dict_set(&opt, "nal-hrd", "cbr", 0);
      break;
    case ALVR_VBR:
      av_dict_set(&opt, "nal-hrd", "vbr", 0);
      break;
  }

  av_dict_set(&opt, "preset", "ultrafast", 0);
  av_dict_set(&opt, "tune", "zerolatency", 0);
  av_dict_set(&opt, "forced-idr", "true", 0);

  encoder_ctx->width = width;
  encoder_ctx->height = height;
  encoder_ctx->time_base = {1, (int)1e9};
  encoder_ctx->framerate = AVRational{settings.m_refreshRate, 1};
  encoder_ctx->sample_aspect_ratio = AVRational{1, 1};
  encoder_ctx->pix_fmt = settings.m_use10bitEncoder && codec_id == ALVR_CODEC_H265 ? AV_PIX_FMT_YUV420P10 : AV_PIX_FMT_YUV420P;
  encoder_ctx->max_b_frames = 0;
  encoder_ctx->gop_size = 0;
  SetBitrate(settings.mEncodeBitrateMBs * 1'000'000L);
  encoder_ctx->thread_type = FF_THREAD_SLICE;
  encoder_ctx->thread_count = settings.m_swThreadCount;

  int err = avcodec_open2(encoder_ctx, codec, &opt);
  if (err < 0) {
    throw alvr::AvException("Cannot open video encoder codec:", err);
  }

  encoder_frame = av_frame_alloc();
  encoder_frame->width = width;
  encoder_frame->height = height;
  encoder_frame->format = encoder_ctx->pix_fmt;
  av_frame_get_buffer(encoder_frame, 0);
  rgbtoyuv = new RgbToYuv420(render, render->GetOutput().image, render->GetOutput().imageInfo, render->GetOutput().semaphore);
}

alvr::EncodePipelineSW::~EncodePipelineSW()
{
  if (rgbtoyuv) {
    delete rgbtoyuv;
  }
  av_frame_free(&encoder_frame);
}

void alvr::EncodePipelineSW::PushFrame(uint64_t targetTimestampNs, bool idr)
{
  rgbtoyuv->Convert(encoder_frame->data, encoder_frame->linesize);
  rgbtoyuv->Sync();
  timestamp.cpu = std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count();

  encoder_frame->pict_type = idr ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
  encoder_frame->pts = targetTimestampNs;

  int err;
  if ((err = avcodec_send_frame(encoder_ctx, encoder_frame)) < 0) {
    throw alvr::AvException("avcodec_send_frame failed:", err);
  }
}
