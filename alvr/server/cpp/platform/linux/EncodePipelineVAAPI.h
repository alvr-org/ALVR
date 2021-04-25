#pragma once

#include "EncodePipeline.h"

extern "C" struct AVBufferRef;
extern "C" struct AVCodecContext;
extern "C" struct AVFilterContext;
extern "C" struct AVFilterGraph;
extern "C" struct AVFrame;

namespace alvr
{

class EncodePipelineVAAPI: public EncodePipeline
{
public:
  ~EncodePipelineVAAPI();
  EncodePipelineVAAPI(std::vector<VkFrame> &input_frames, VkFrameCtx& vk_frame_ctx);

  void EncodeFrame(uint32_t frame_index, bool idr, std::vector<uint8_t>& out) override;
private:
  AVBufferRef *hw_ctx = nullptr;
  AVCodecContext *encoder_ctx = nullptr;
  std::vector<AVFrame *> mapped_frames;
  AVFilterGraph *filter_graph = nullptr;
  AVFilterContext *filter_in = nullptr;
  AVFilterContext *filter_out = nullptr;
};
}
