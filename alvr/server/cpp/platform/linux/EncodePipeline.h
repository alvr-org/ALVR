#pragma once
#include <cstdint>
#include <memory>
#include <vector>

namespace alvr
{

class VkFrame;
class VkFrameCtx;

class EncodePipeline
{
public:
  virtual ~EncodePipeline() = default;

  virtual void EncodeFrame(uint32_t frame_index, bool idr, std::vector<uint8_t> & out) = 0;

  static std::unique_ptr<EncodePipeline> Create(std::vector<VkFrame> &input_frames, VkFrameCtx &vk_frame_ctx);
};

}
