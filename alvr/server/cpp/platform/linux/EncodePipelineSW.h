#pragma once

#include "EncodePipeline.h"

extern "C" struct AVFrame;
extern "C" struct SwsContext;

namespace alvr
{

class EncodePipelineSW: public EncodePipeline
{
public:
  ~EncodePipelineSW();
  EncodePipelineSW(VkFrame &input_frame, VkFrameCtx& vk_frame_ctx, uint32_t width, uint32_t height);

  void PushFrame(uint64_t targetTimestampNs, bool idr) override;

private:
  AVFrame *vk_frame;
  AVFrame * transferred_frame = nullptr;
  AVFrame * encoder_frame = nullptr;
  SwsContext *scaler_ctx = nullptr;
};
}
