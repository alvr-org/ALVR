#pragma once
#include <cstdint>
#include <memory>
#include <vector>
#include "alvr_server/bindings.h"

extern "C" struct AVCodecContext;
extern "C" struct AVPacket;

class Renderer;

namespace alvr
{

class VkFrame;
class VkFrameCtx;
class VkContext;

struct FramePacket {
  uint8_t *data;
  int size;
  uint64_t pts;
  bool isIDR;
};

class EncodePipeline
{
public:
  struct Timestamp {
    uint64_t gpu = 0;
    uint64_t cpu = 0;
  };

  virtual ~EncodePipeline();

  virtual void PushFrame(uint64_t targetTimestampNs, bool idr) = 0;
  virtual bool GetEncoded(FramePacket &data);
  virtual Timestamp GetTimestamp() { return timestamp; }
  virtual int GetCodec();

  virtual void SetParams(FfiDynamicEncoderParams params);
  static std::unique_ptr<EncodePipeline> Create(Renderer *render, VkContext &vk_ctx, VkFrame &input_frame, VkFrameCtx &vk_frame_ctx, uint32_t width, uint32_t height);
protected:
  AVCodecContext *encoder_ctx = nullptr; //shall be initialized by child class
  AVPacket *encoder_packet = NULL;
  Timestamp timestamp = {};
};

}
