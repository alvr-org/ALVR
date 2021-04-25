#include "EncodePipeline.h"

#include "EncodePipelineVAAPI.h"

std::unique_ptr<alvr::EncodePipeline> alvr::EncodePipeline::Create(std::vector<VkFrame> &input_frames, VkFrameCtx &vk_frame_ctx)
{
  return std::make_unique<alvr::EncodePipelineVAAPI>(input_frames, vk_frame_ctx);
}
