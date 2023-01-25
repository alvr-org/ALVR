#pragma once

#include "EncodePipeline.h"

extern "C" struct AVFrame;

class FormatConverter;

namespace alvr
{

class EncodePipelineSW: public EncodePipeline
{
public:
  ~EncodePipelineSW();
  EncodePipelineSW(Renderer *render, uint32_t width, uint32_t height);

  void PushFrame(uint64_t targetTimestampNs, bool idr) override;

private:
  AVFrame *encoder_frame = nullptr;
  FormatConverter *rgbtoyuv = nullptr;
};
}
