#pragma once

#include "EncodePipeline.h"

#include <x264.h>
#include <x265.h>

class FormatConverter;

namespace alvr
{

class EncodePipelineSW: public EncodePipeline
{
public:
  ~EncodePipelineSW();
  EncodePipelineSW(Renderer *render, uint32_t width, uint32_t height);

  void PushFrame(uint64_t targetTimestampNs, bool idr) override;
  bool GetEncoded(FramePacket &packet) override;
  void SetParams(FfiDynamicEncoderParams params) override;

private:
  struct {
    x264_t *enc = nullptr;
    x264_param_t param;
    x264_picture_t picture;
    x264_picture_t picture_out;
    x264_nal_t *nal = nullptr;
  } avc;
  struct {
    const x265_api *api = nullptr;
    x265_encoder *enc = nullptr;
    x265_param *param = nullptr;
    x265_picture picture;
    x265_picture picture_out;
    x265_nal *nal = nullptr;
  } hevc;
  bool use_x264 = false;
  int nal_size = 0;
  int64_t pts = 0;
  FormatConverter *rgbtoyuv = nullptr;
};
}
