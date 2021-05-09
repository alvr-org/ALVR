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

  if (!(hw_frames_ref = AVUTIL.av_hwframe_ctx_alloc(hw_device_ctx))) {
    throw std::runtime_error("Failed to create VAAPI frame context.");
  }
  frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
  frames_ctx->format = AV_PIX_FMT_VAAPI;
  frames_ctx->sw_format = AV_PIX_FMT_NV12;
  frames_ctx->width = ctx->width;
  frames_ctx->height = ctx->height;
  frames_ctx->initial_pool_size = 3;
  if ((err = AVUTIL.av_hwframe_ctx_init(hw_frames_ref)) < 0) {
    AVUTIL.av_buffer_unref(&hw_frames_ref);
    throw alvr::AvException("Failed to initialize VAAPI frame context:", err);
  }
  ctx->hw_frames_ctx = AVUTIL.av_buffer_ref(hw_frames_ref);
  if (!ctx->hw_frames_ctx)
    err = AVERROR(ENOMEM);

  AVUTIL.av_buffer_unref(&hw_frames_ref);
}

// Map the vulkan frames to corresponding vaapi frames
std::vector<AVFrame*> map_frames(AVBufferRef *hw_device_ctx, std::vector<alvr::VkFrame>& input_frames, alvr::VkFrameCtx& vk_frame_ctx)
{
  AVBufferRef *hw_frames_ref;
  int err = 0;

  auto input_frame_ctx = (AVHWFramesContext*)vk_frame_ctx.ctx->data;

  if (!(hw_frames_ref = AVUTIL.av_hwframe_ctx_alloc(hw_device_ctx))) {
    throw std::runtime_error("Failed to create VAAPI frame context.");
  }
  auto frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
  frames_ctx->format = AV_PIX_FMT_VAAPI;
  frames_ctx->sw_format = input_frame_ctx->sw_format;
  frames_ctx->width = input_frame_ctx->width;
  frames_ctx->height = input_frame_ctx->height;
  frames_ctx->initial_pool_size = input_frames.size();
  if ((err = AVUTIL.av_hwframe_ctx_init(hw_frames_ref)) < 0) {
    AVUTIL.av_buffer_unref(&hw_frames_ref);
    throw alvr::AvException("Failed to initialize VAAPI frame context:", err);
  }

  std::vector<AVFrame*> result(0);
  for (auto& input_frame: input_frames)
  {
    AVFrame * mapped_frame = AVUTIL.av_frame_alloc();
    AVUTIL.av_hwframe_get_buffer(hw_frames_ref, mapped_frame, 0);
    auto vk_frame = input_frame.make_av_frame(vk_frame_ctx);
    AVUTIL.av_hwframe_map(mapped_frame, vk_frame.get(), AV_HWFRAME_MAP_READ);
    result.push_back(mapped_frame);
  }

  AVUTIL.av_buffer_unref(&hw_frames_ref);

  return result;
}

}

alvr::EncodePipelineVAAPI::EncodePipelineVAAPI(std::vector<VkFrame>& input_frames, VkFrameCtx& vk_frame_ctx)
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
  int err = AVUTIL.av_hwdevice_ctx_create(&hw_ctx, AV_HWDEVICE_TYPE_VAAPI, NULL, NULL, 0);
  if (err < 0) {
    throw alvr::AvException("Failed to create a VAAPI device:", err);
  }

  const auto& settings = Settings::Instance();

  auto codec_id = ALVR_CODEC(settings.m_codec);
  const char * encoder_name = encoder(codec_id);
  AVCodec *codec = AVCODEC.avcodec_find_encoder_by_name(encoder_name);
  if (codec == nullptr)
  {
    throw std::runtime_error(std::string("Failed to find encoder ") + encoder_name);
  }

  encoder_ctx = AVCODEC.avcodec_alloc_context3(codec);
  if (not encoder_ctx)
  {
    throw std::runtime_error("failed to allocate VAAPI encoder");
  }

  switch (codec_id)
  {
    case ALVR_CODEC_H264:
      encoder_ctx->profile = FF_PROFILE_H264_MAIN;
      AVUTIL.av_opt_set(encoder_ctx, "rc_mode", "2", 0); //CBR
      break;
    case ALVR_CODEC_H265:
      encoder_ctx->profile = FF_PROFILE_HEVC_MAIN;
      AVUTIL.av_opt_set(encoder_ctx, "rc_mode", "2", 0);
      break;
  }

  encoder_ctx->width = settings.m_renderWidth;
  encoder_ctx->height = settings.m_renderHeight;
  encoder_ctx->time_base = {std::chrono::steady_clock::period::num, std::chrono::steady_clock::period::den};
  encoder_ctx->framerate = AVRational{settings.m_refreshRate, 1};
  encoder_ctx->sample_aspect_ratio = AVRational{1, 1};
  encoder_ctx->pix_fmt = AV_PIX_FMT_VAAPI;
  encoder_ctx->max_b_frames = 0;
  encoder_ctx->bit_rate = settings.mEncodeBitrateMBs * 1024 * 1024;

  set_hwframe_ctx(encoder_ctx, hw_ctx);

  err = AVCODEC.avcodec_open2(encoder_ctx, codec, NULL);
  if (err < 0) {
    throw alvr::AvException("Cannot open video encoder codec:", err);
  }

  mapped_frames = map_frames(hw_ctx, input_frames, vk_frame_ctx);

  filter_graph = AVFILTER.avfilter_graph_alloc();

  AVFilterInOut *outputs = AVFILTER.avfilter_inout_alloc();
  AVFilterInOut *inputs = AVFILTER.avfilter_inout_alloc();

  filter_in = AVFILTER.avfilter_graph_alloc_filter(filter_graph, AVFILTER.avfilter_get_by_name("buffer"), "in");

  AVBufferSrcParameters *par = AVFILTER.av_buffersrc_parameters_alloc();
  memset(par, 0, sizeof(*par));
  par->width = mapped_frames[0]->width;
  par->height = mapped_frames[0]->height;
  par->time_base = encoder_ctx->time_base;
  par->format = mapped_frames[0]->format;
  par->hw_frames_ctx = AVUTIL.av_buffer_ref(mapped_frames[0]->hw_frames_ctx);
  AVFILTER.av_buffersrc_parameters_set(filter_in, par);
  AVUTIL.av_free(par);

  if ((err = AVFILTER.avfilter_graph_create_filter(&filter_out, AVFILTER.avfilter_get_by_name("buffersink"), "out", NULL, NULL, filter_graph)))
  {
    throw alvr::AvException("filter_out creation failed:", err);
  }

  outputs->name = AVUTIL.av_strdup("in");
  outputs->filter_ctx = filter_in;
  outputs->pad_idx = 0;
  outputs->next = NULL;

  inputs->name = AVUTIL.av_strdup("out");
  inputs->filter_ctx = filter_out;
  inputs->pad_idx = 0;
  inputs->next = NULL;

  if ((err = AVFILTER.avfilter_graph_parse_ptr(filter_graph, "scale_vaapi=format=nv12", &inputs, &outputs, NULL)) < 0)
  {
    throw alvr::AvException("avfilter_graph_parse_ptr failed:", err);
  }

  AVFILTER.avfilter_inout_free(&outputs);
  AVFILTER.avfilter_inout_free(&inputs);

  for (unsigned i = 0 ; i < filter_graph->nb_filters; ++i)
  {
    filter_graph->filters[i]->hw_device_ctx = AVUTIL.av_buffer_ref(hw_ctx);
  }

  if ((err = AVFILTER.avfilter_graph_config(filter_graph, NULL)))
  {
    throw alvr::AvException("avfilter_graph_config failed:", err);
  }
}

alvr::EncodePipelineVAAPI::~EncodePipelineVAAPI()
{
  AVFILTER.avfilter_graph_free(&filter_graph);
  for (auto frame: mapped_frames)
  {
    AVUTIL.av_frame_free(&frame);
  }
  AVUTIL.av_buffer_unref(&hw_ctx);
}

void alvr::EncodePipelineVAAPI::PushFrame(uint32_t frame_index, bool idr)
{
  assert(frame_index < mapped_frames.size());
  AVFrame *encoder_frame = AVUTIL.av_frame_alloc();
  int err = AVFILTER.av_buffersrc_add_frame_flags(filter_in, mapped_frames[frame_index], AV_BUFFERSRC_FLAG_PUSH | AV_BUFFERSRC_FLAG_KEEP_REF);
  if (err != 0)
  {
    throw alvr::AvException("av_buffersrc_add_frame failed", err);
  }
  err = AVFILTER.av_buffersink_get_frame(filter_out, encoder_frame);
  if (err != 0)
  {
    throw alvr::AvException("av_buffersink_get_frame failed", err);
  }

  encoder_frame->pict_type = idr ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
  encoder_frame->pts = std::chrono::steady_clock::now().time_since_epoch().count();

  if ((err = AVCODEC.avcodec_send_frame(encoder_ctx, encoder_frame)) < 0) {
    throw alvr::AvException("avcodec_send_frame failed: ", err);
  }
  AVUTIL.av_frame_unref(encoder_frame);
}
