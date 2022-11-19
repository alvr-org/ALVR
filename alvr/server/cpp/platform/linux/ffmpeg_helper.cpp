#include "ffmpeg_helper.h"

#include <chrono>

#include "alvr_server/bindings.h"

#define str(s) #s
#define LOAD_LIB(LIBNAME, VERSION) \
	if (not m_##LIBNAME.Load(g_driverRootDir + std::string("/lib"#LIBNAME".so." str(VERSION)))) {\
		if (not m_##LIBNAME.Load("lib"#LIBNAME".so." str(VERSION))) {\
			throw std::runtime_error("failed to load lib"#LIBNAME".so." str(VERSION));\
		}\
	}\

alvr::libav::libav()
{
	LOAD_LIB(avutil, AVUTIL_MAJOR)
	LOAD_LIB(avcodec, AVCODEC_MAJOR)
	LOAD_LIB(swscale, SWSCALE_MAJOR)
	LOAD_LIB(avfilter, AVFILTER_MAJOR)
}
#undef str

alvr::libav& alvr::libav::instance()
{
	static libav instance;
	return instance;
}

namespace {
// it seems that ffmpeg does not provide this mapping
AVPixelFormat vk_format_to_av_format(vk::Format vk_fmt)
{
  for (int f = AV_PIX_FMT_NONE; f < AV_PIX_FMT_NB; ++f)
  {
    auto current_fmt = AVUTIL.av_vkfmt_from_pixfmt(AVPixelFormat(f));
    if (current_fmt and *current_fmt == (VkFormat)vk_fmt)
      return AVPixelFormat(f);
  }
  throw std::runtime_error("unsupported vulkan pixel format " + std::to_string((VkFormat)vk_fmt));
}
}

std::string alvr::AvException::makemsg(const std::string & msg, int averror)
{
  char av_msg[AV_ERROR_MAX_STRING_SIZE];
  AVUTIL.av_strerror(averror, av_msg, sizeof(av_msg));
  return msg + " " + av_msg;
}

alvr::VkContext::VkContext(const char* device, AVDictionary * opt)
{
  int ret = AVUTIL.av_hwdevice_ctx_create(&ctx, AV_HWDEVICE_TYPE_VULKAN, device, opt, 0);
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

vk::Instance alvr::VkContext::get_vk_instance() const
{
  AVHWDeviceContext *hwctx = (AVHWDeviceContext *)ctx->data;
  AVVulkanDeviceContext *vkctx = (AVVulkanDeviceContext *)hwctx->hwctx;
  return vkctx->inst;
}

vk::PhysicalDevice alvr::VkContext::get_vk_phys_device() const
{
  AVHWDeviceContext *hwctx = (AVHWDeviceContext *)ctx->data;
  AVVulkanDeviceContext *vkctx = (AVVulkanDeviceContext *)hwctx->hwctx;
  return vkctx->phys_dev;
}

std::vector<uint32_t> alvr::VkContext::get_vk_queue_families() const
{
  AVHWDeviceContext *hwctx = (AVHWDeviceContext *)ctx->data;
  AVVulkanDeviceContext *vkctx = (AVVulkanDeviceContext *)hwctx->hwctx;

  std::vector<uint32_t> out;
  out.push_back(vkctx->queue_family_index);
  if (std::find(out.begin(), out.end(), vkctx->queue_family_comp_index) == out.end()) {
    out.push_back(vkctx->queue_family_comp_index);
  }
  if (std::find(out.begin(), out.end(), vkctx->queue_family_tx_index) == out.end()) {
    out.push_back(vkctx->queue_family_tx_index);
  }
  return out;
}

std::vector<std::string> alvr::VkContext::get_vk_device_extensions() const
{
  AVHWDeviceContext *hwctx = (AVHWDeviceContext *)ctx->data;
  AVVulkanDeviceContext *vkctx = (AVVulkanDeviceContext *)hwctx->hwctx;

  std::vector<std::string> out;
  for (size_t i = 0; i < vkctx->nb_enabled_dev_extensions; ++i) {
    out.push_back(vkctx->enabled_dev_extensions[i]);
  }
  return out;
}

alvr::VkContext::~VkContext()
{
  AVUTIL.av_buffer_unref(&ctx);
}

alvr::VkFrameCtx::VkFrameCtx(VkContext & vkContext, vk::ImageCreateInfo image_create_info)
{
  AVHWFramesContext *frames_ctx = NULL;
  int err = 0;

  if (!(ctx = AVUTIL.av_hwframe_ctx_alloc(vkContext.ctx))) {
    throw std::runtime_error("Failed to create vulkan frame context.");
  }
  frames_ctx = (AVHWFramesContext *)(ctx->data);
  frames_ctx->format = AV_PIX_FMT_VULKAN;
  frames_ctx->sw_format = vk_format_to_av_format(image_create_info.format);
  frames_ctx->width = image_create_info.extent.width;
  frames_ctx->height = image_create_info.extent.height;
  frames_ctx->initial_pool_size = 0;
  if ((err = AVUTIL.av_hwframe_ctx_init(ctx)) < 0) {
    AVUTIL.av_buffer_unref(&ctx);
    throw alvr::AvException("Failed to initialize vulkan frame context:", err);
  }
}

alvr::VkFrameCtx::~VkFrameCtx()
{
  AVUTIL.av_buffer_unref(&ctx);
}

alvr::VkFrame::VkFrame(
    const VkContext& vk_ctx,
    VkImage image,
    VkImageCreateInfo image_info,
    VkDeviceSize size,
    VkDeviceMemory memory
    ):
  width(image_info.extent.width),
  height(image_info.extent.height)
{
  device = vk_ctx.get_vk_device();

  av_vkframe = AVUTIL.av_vk_frame_alloc();
  av_vkframe->img[0] = image;
  av_vkframe->tiling = image_info.tiling;
  av_vkframe->mem[0] = memory;
  av_vkframe->size[0] = size;
  av_vkframe->layout[0] = VK_IMAGE_LAYOUT_UNDEFINED;

  VkSemaphoreTypeCreateInfo timelineInfo = {};
  timelineInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO;
  timelineInfo.semaphoreType = VK_SEMAPHORE_TYPE_TIMELINE;

  VkSemaphoreCreateInfo semInfo = {};
  semInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
  semInfo.pNext = &timelineInfo;
  vkCreateSemaphore(device, &semInfo, nullptr, &av_vkframe->sem[0]);
}

alvr::VkFrame::~VkFrame()
{
  vkDestroySemaphore(device, av_vkframe->sem[0], nullptr);
  AVUTIL.av_free(av_vkframe);
}

std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> alvr::VkFrame::make_av_frame(VkFrameCtx &frame_ctx)
{
  std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> frame{
    AVUTIL.av_frame_alloc(),
      [](AVFrame *p) {AVUTIL.av_frame_free(&p);}
  };
  frame->width = width;
  frame->height = height;
  frame->hw_frames_ctx = AVUTIL.av_buffer_ref(frame_ctx.ctx);
  frame->data[0] = (uint8_t*)av_vkframe;
  frame->format = AV_PIX_FMT_VULKAN;
  frame->buf[0] = AVUTIL.av_buffer_alloc(1);
  frame->pts = std::chrono::steady_clock::now().time_since_epoch().count();

  return frame;
}
