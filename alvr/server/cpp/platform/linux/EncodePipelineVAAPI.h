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
  EncodePipelineVAAPI(VkFrame &input_frame, VkFrameCtx& vk_frame_ctx, uint32_t width, uint32_t height);

  void PushFrame(uint64_t targetTimestampNs, bool idr) override;

private:
  AVBufferRef *hw_ctx = nullptr;
  AVFrame *mapped_frame;
  AVFilterGraph *filter_graph = nullptr;
  AVFilterContext *filter_in = nullptr;
  AVFilterContext *filter_out = nullptr;
};
}
