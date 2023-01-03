#include "EncodePipelineVAAPI.h"
#include "ALVR-common/packet_types.h"
#include "ffmpeg_helper.h"
#include "alvr_server/Settings.h"
#include <chrono>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavutil/hwcontext.h>
#include <libavutil/opt.h>
}

namespace
{

const char * encoder(ALVR_CODEC codec)
{
  switch (codec)
  {
    case ALVR_CODEC_H264:
      return "h264_vaapi";
    case ALVR_CODEC_H265:
      return "hevc_vaapi";
  }
  throw std::runtime_error("invalid codec " + std::to_string(codec));
}

void set_hwframe_ctx(AVCodecContext *ctx, AVBufferRef *hw_device_ctx)
{
  AVBufferRef *hw_frames_ref;
  AVHWFramesContext *frames_ctx = NULL;
  int err = 0;

  if (!(hw_frames_ref = av_hwframe_ctx_alloc(hw_device_ctx))) {
    throw std::runtime_error("Failed to create VAAPI frame context.");
  }
  frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
  frames_ctx->format = AV_PIX_FMT_VAAPI;
  frames_ctx->sw_format = Settings::Instance().m_codec == ALVR_CODEC_H265 && Settings::Instance().m_use10bitEncoder ? AV_PIX_FMT_P010 : AV_PIX_FMT_NV12;
  frames_ctx->width = ctx->width;
  frames_ctx->height = ctx->height;
  frames_ctx->initial_pool_size = 3;
  if ((err = av_hwframe_ctx_init(hw_frames_ref)) < 0) {
    av_buffer_unref(&hw_frames_ref);
    throw alvr::AvException("Failed to initialize VAAPI frame context:", err);
  }
  ctx->hw_frames_ctx = av_buffer_ref(hw_frames_ref);
  if (!ctx->hw_frames_ctx)
    err = AVERROR(ENOMEM);

  av_buffer_unref(&hw_frames_ref);
}

// Map the vulkan frames to corresponding vaapi frames
AVFrame *map_frame(AVBufferRef *hw_device_ctx, alvr::VkFrame &input_frame, alvr::VkFrameCtx& vk_frame_ctx)
{
  AVBufferRef *hw_frames_ref;
  int err = 0;

  auto input_frame_ctx = (AVHWFramesContext*)vk_frame_ctx.ctx->data;

  if (!(hw_frames_ref = av_hwframe_ctx_alloc(hw_device_ctx))) {
    throw std::runtime_error("Failed to create VAAPI frame context.");
  }
  auto frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
  frames_ctx->format = AV_PIX_FMT_VAAPI;
  frames_ctx->sw_format = input_frame_ctx->sw_format;
  frames_ctx->width = input_frame_ctx->width;
  frames_ctx->height = input_frame_ctx->height;
  frames_ctx->initial_pool_size = 1;
  if ((err = av_hwframe_ctx_init(hw_frames_ref)) < 0) {
    av_buffer_unref(&hw_frames_ref);
    throw alvr::AvException("Failed to initialize VAAPI frame context:", err);
  }

  AVFrame * mapped_frame = av_frame_alloc();
  av_hwframe_get_buffer(hw_frames_ref, mapped_frame, 0);
  auto vk_frame = input_frame.make_av_frame(vk_frame_ctx);
  av_hwframe_map(mapped_frame, vk_frame.get(), AV_HWFRAME_MAP_READ);

  av_buffer_unref(&hw_frames_ref);

  return mapped_frame;
}

}

alvr::EncodePipelineVAAPI::EncodePipelineVAAPI(VkFrame &input_frame, VkFrameCtx& vk_frame_ctx, uint32_t width, uint32_t height)
{
  /* VAAPI Encoding pipeline
   * The encoding pipeline has 3 frame types:
   * - input vulkan frames, only used to initialize the mapped frames
   * - mapped frames, one per input frame, same format, and point to the same memory on the device
   * - encoder frame, with a format compatible with the encoder, created by the filter
   * Each frame type has a corresponding hardware frame context, the vulkan one is provided
   *
   * The pipeline is simply made of a scale_vaapi object, that does the conversion between formats
   * and the encoder that takes the converted frame and produces packets.
   */
  int err = av_hwdevice_ctx_create(&hw_ctx, AV_HWDEVICE_TYPE_VAAPI, NULL, NULL, 0);
  if (err < 0) {
    throw alvr::AvException("Failed to create a VAAPI device:", err);
  }

  const auto& settings = Settings::Instance();

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
    throw std::runtime_error("failed to allocate VAAPI encoder");
  }

  switch (codec_id)
  {
    case ALVR_CODEC_H264:
      encoder_ctx->profile = FF_PROFILE_H264_MAIN;

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
      encoder_ctx->profile = Settings::Instance().m_use10bitEncoder ? FF_PROFILE_HEVC_MAIN_10 : FF_PROFILE_HEVC_MAIN;
      break;
  }

  encoder_ctx->width = width;
  encoder_ctx->height = height;
  encoder_ctx->time_base = {1, (int)1e9};
  encoder_ctx->framerate = AVRational{settings.m_refreshRate, 1};
  encoder_ctx->sample_aspect_ratio = AVRational{1, 1};
  encoder_ctx->pix_fmt = AV_PIX_FMT_VAAPI;
  encoder_ctx->max_b_frames = 0;
  encoder_ctx->gop_size = INT16_MAX;
  encoder_ctx->bit_rate = settings.mEncodeBitrateMBs * 1000 * 1000;
  encoder_ctx->rc_min_rate = encoder_ctx->bit_rate;
  encoder_ctx->rc_max_rate = encoder_ctx->bit_rate;
  encoder_ctx->rc_buffer_size = encoder_ctx->bit_rate / settings.m_refreshRate;
  
  vlVaQualityBits quality = {};
  quality.valid_setting = 1;
  quality.vbaq_mode = 1;  //No noticable performance difference and should improve subjective quality by allocating more bits to smooth areas
  switch (settings.m_encoderQualityPreset)
  {
    case ALVR_QUALITY:
      quality.preset_mode = PRESET_MODE_QUALITY;
      encoder_ctx->compression_level = quality.quality; // (QUALITY preset, no pre-encoding, vbaq)
    break;
    case ALVR_BALANCED: 
      quality.preset_mode = PRESET_MODE_BALANCE;
      encoder_ctx->compression_level = quality.quality; // (BALANCE preset, no pre-encoding, vbaq)
    break;
    case ALVR_SPEED:
      default:
       quality.preset_mode = PRESET_MODE_SPEED;
       encoder_ctx->compression_level = quality.quality; // (speed preset, no pre-encoding, vbaq)
    break;
  }
  
  av_opt_set_int(encoder_ctx->priv_data, "idr_interval", INT_MAX, 0);
  av_opt_set_int(encoder_ctx->priv_data, "async_depth", 1, 0);

  set_hwframe_ctx(encoder_ctx, hw_ctx);

  err = avcodec_open2(encoder_ctx, codec, NULL);
  if (err < 0) {
    throw alvr::AvException("Cannot open video encoder codec:", err);
  }

  encoder_frame = av_frame_alloc();
  mapped_frame = map_frame(hw_ctx, input_frame, vk_frame_ctx);

  filter_graph = avfilter_graph_alloc();

  AVFilterInOut *outputs = avfilter_inout_alloc();
  AVFilterInOut *inputs = avfilter_inout_alloc();

  filter_in = avfilter_graph_alloc_filter(filter_graph, avfilter_get_by_name("buffer"), "in");

  AVBufferSrcParameters *par = av_buffersrc_parameters_alloc();
  memset(par, 0, sizeof(*par));
  par->width = mapped_frame->width;
  par->height = mapped_frame->height;
  par->time_base = encoder_ctx->time_base;
  par->format = mapped_frame->format;
  par->hw_frames_ctx = av_buffer_ref(mapped_frame->hw_frames_ctx);
  av_buffersrc_parameters_set(filter_in, par);
  av_free(par);

  if ((err = avfilter_graph_create_filter(&filter_out, avfilter_get_by_name("buffersink"), "out", NULL, NULL, filter_graph)))
  {
    throw alvr::AvException("filter_out creation failed:", err);
  }

  outputs->name = av_strdup("in");
  outputs->filter_ctx = filter_in;
  outputs->pad_idx = 0;
  outputs->next = NULL;

  inputs->name = av_strdup("out");
  inputs->filter_ctx = filter_out;
  inputs->pad_idx = 0;
  inputs->next = NULL;

  std::string filters = "scale_vaapi=format=";
  if (Settings::Instance().m_codec == ALVR_CODEC_H265 && Settings::Instance().m_use10bitEncoder) {
    filters += "p010";
  } else {
    filters += "nv12";
  }
  if ((err = avfilter_graph_parse_ptr(filter_graph, filters.c_str(), &inputs, &outputs, NULL)) < 0)
  {
    throw alvr::AvException("avfilter_graph_parse_ptr failed:", err);
  }

  avfilter_inout_free(&outputs);
  avfilter_inout_free(&inputs);

  for (unsigned i = 0 ; i < filter_graph->nb_filters; ++i)
  {
    filter_graph->filters[i]->hw_device_ctx = av_buffer_ref(hw_ctx);
  }

  if ((err = avfilter_graph_config(filter_graph, NULL)))
  {
    throw alvr::AvException("avfilter_graph_config failed:", err);
  }
}

alvr::EncodePipelineVAAPI::~EncodePipelineVAAPI()
{
  avfilter_graph_free(&filter_graph);
  av_frame_free(&mapped_frame);
  av_frame_free(&encoder_frame);
  av_buffer_unref(&hw_ctx);
}

void alvr::EncodePipelineVAAPI::PushFrame(uint64_t targetTimestampNs, bool idr)
{
  int err = av_buffersrc_add_frame_flags(filter_in, mapped_frame, AV_BUFFERSRC_FLAG_PUSH | AV_BUFFERSRC_FLAG_KEEP_REF);
  if (err != 0)
  {
    throw alvr::AvException("av_buffersrc_add_frame failed", err);
  }
  err = av_buffersink_get_frame(filter_out, encoder_frame);
  if (err != 0)
  {
    throw alvr::AvException("av_buffersink_get_frame failed", err);
  }

  encoder_frame->pict_type = idr ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
  encoder_frame->pts = targetTimestampNs;

  if ((err = avcodec_send_frame(encoder_ctx, encoder_frame)) < 0) {
    throw alvr::AvException("avcodec_send_frame failed: ", err);
  }
  av_frame_unref(encoder_frame);
}