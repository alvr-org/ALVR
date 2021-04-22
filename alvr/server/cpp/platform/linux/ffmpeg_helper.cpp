#include "ffmpeg_helper.h"

#include <chrono>

extern "C" {
#include <libavutil/avutil.h>
#include <libavutil/hwcontext.h>
#include <libavutil/hwcontext_vulkan.h>
}

namespace {
// it seems that ffmpeg does not provide this mapping
AVPixelFormat vk_format_to_av_format(vk::Format vk_fmt)
{
  for (int f = AV_PIX_FMT_NONE; f < AV_PIX_FMT_NB; ++f)
  {
    auto current_fmt = av_vkfmt_from_pixfmt(AVPixelFormat(f));
    if (current_fmt and *current_fmt == (VkFormat)vk_fmt)
      return AVPixelFormat(f);
  }
  throw std::runtime_error("unsupported vulkan pixel format " + std::to_string((VkFormat)vk_fmt));
}
}

std::string alvr::AvException::makemsg(const std::string & msg, int averror)
{
  char av_msg[AV_ERROR_MAX_STRING_SIZE];
  av_strerror(averror, av_msg, sizeof(av_msg));
  return msg + " " + av_msg;
}

alvr::VkContext::VkContext(const char* device, AVDictionary * opt)
{
  int ret = av_hwdevice_ctx_create(&ctx, AV_HWDEVICE_TYPE_VULKAN, device, opt, 0);
  if (ret)
    throw AvException("failed to initialize vulkan", ret);

  AVHWDeviceContext *hwctx = (AVHWDeviceContext *)ctx->data;
  AVVulkanDeviceContext *vkctx = (AVVulkanDeviceContext *)hwctx->hwctx;

#define VK_LOAD_PFN(inst, name) (PFN_##name) vkGetInstanceProcAddr(inst, #name)
  d.vkImportSemaphoreFdKHR = VK_LOAD_PFN(vkctx->inst, vkImportSemaphoreFdKHR);
}

vk::Device alvr::VkContext::get_vk_device() const
{
  AVHWDeviceContext *hwctx = (AVHWDeviceContext *)ctx->data;
  AVVulkanDeviceContext *vkctx = (AVVulkanDeviceContext *)hwctx->hwctx;
  return vkctx->act_dev;
}

alvr::VkContext::~VkContext()
{
  av_buffer_unref(&ctx);
}

alvr::VkFrameCtx::VkFrameCtx(VkContext & vkContext, vk::ImageCreateInfo image_create_info)
{
  AVHWFramesContext *frames_ctx = NULL;
  int err = 0;

  if (!(ctx = av_hwframe_ctx_alloc(vkContext.ctx))) {
    throw std::runtime_error("Failed to create vulkan frame context.");
  }
  frames_ctx = (AVHWFramesContext *)(ctx->data);
  frames_ctx->format = AV_PIX_FMT_VULKAN;
  frames_ctx->sw_format = vk_format_to_av_format(image_create_info.format);
  frames_ctx->width = image_create_info.extent.width;
  frames_ctx->height = image_create_info.extent.height;
  frames_ctx->initial_pool_size = 0;
  if ((err = av_hwframe_ctx_init(ctx)) < 0) {
    av_buffer_unref(&ctx);
    throw alvr::AvException("Failed to initialize vulkan frame context:", err);
  }
}

alvr::VkFrameCtx::~VkFrameCtx()
{
  av_buffer_unref(&ctx);
}

alvr::VkFrame::VkFrame(
    const VkContext& vk_ctx,
    vk::ImageCreateInfo image_create_info,
    size_t memory_index,
    int image_fd, int semaphore_fd):
  width(image_create_info.extent.width),
  height(image_create_info.extent.height)
{
  device = vk_ctx.get_vk_device();

  vk::ExternalMemoryImageCreateInfo extMemImageInfo;
  extMemImageInfo.handleTypes = vk::ExternalMemoryHandleTypeFlagBits::eOpaqueFd;
  image_create_info.pNext = &extMemImageInfo;
  image_create_info.initialLayout = vk::ImageLayout::eUndefined;// VUID-VkImageCreateInfo-pNext-01443
  vk::Image image = device.createImage(image_create_info);

  auto req = device.getImageMemoryRequirements(image);

  vk::MemoryDedicatedAllocateInfo dedicatedMemInfo;
  dedicatedMemInfo.image = image;

  vk::ImportMemoryFdInfoKHR importMemInfo;
  importMemInfo.pNext = &dedicatedMemInfo;
  importMemInfo.handleType = vk::ExternalMemoryHandleTypeFlagBits::eOpaqueFd;
  importMemInfo.fd = image_fd;

  vk::MemoryAllocateInfo memAllocInfo;
  memAllocInfo.pNext = &importMemInfo;
  memAllocInfo.allocationSize = req.size;
  memAllocInfo.memoryTypeIndex = memory_index;

  vk::DeviceMemory mem = device.allocateMemory(memAllocInfo);
  device.bindImageMemory(image, mem, 0);

  vk::SemaphoreCreateInfo semInfo;
  vk::Semaphore semaphore = device.createSemaphore(semInfo);

  vk::ImportSemaphoreFdInfoKHR impSemInfo;
  impSemInfo.semaphore = semaphore;
  impSemInfo.handleType = vk::ExternalSemaphoreHandleTypeFlagBits::eOpaqueFd;
  impSemInfo.fd = semaphore_fd;

  device.importSemaphoreFdKHR(impSemInfo, vk_ctx.d);

  av_vkframe = av_vk_frame_alloc();
  av_vkframe->img[0] = image;
  av_vkframe->tiling = (VkImageTiling)image_create_info.tiling;
  av_vkframe->mem[0] = mem;
  av_vkframe->size[0] = req.size;
  av_vkframe->layout[0] = VK_IMAGE_LAYOUT_UNDEFINED;
  av_vkframe->sem[0] = semaphore;
}

alvr::VkFrame::~VkFrame()
{
  device.destroySemaphore(av_vkframe->sem[0]);
  device.destroyImage(av_vkframe->img[0]);
  device.freeMemory(av_vkframe->mem[0]);
  av_free(av_vkframe);
}

std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> alvr::VkFrame::make_av_frame(VkFrameCtx &frame_ctx)
{
  std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> frame{
    av_frame_alloc(),
      [](AVFrame *p) {av_frame_free(&p);}
  };
  frame->width = width;
  frame->height = height;
  frame->hw_frames_ctx = av_buffer_ref(frame_ctx.ctx);
  frame->data[0] = (uint8_t*)av_vkframe;
  frame->format = AV_PIX_FMT_VULKAN;
  frame->buf[0] = av_buffer_alloc(1);
  frame->pts = std::chrono::steady_clock::now().time_since_epoch().count();

  return frame;
}
