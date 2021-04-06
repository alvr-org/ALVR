#include "CEncoder.h"

#include <algorithm>
#include <chrono>
#include <exception>
#include <memory>
#include <openvr_driver.h>
#include <stdexcept>
#include <stdlib.h>
#include <string>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <vulkan/vulkan.hpp>

#include "ALVR-common/packet_types.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/Logger.h"
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Statistics.h"
#include "protocol.h"

extern "C" {
#include <libavutil/hwcontext.h>
#include <libavutil/hwcontext_vulkan.h>
#include <libavdevice/avdevice.h>
#include <libavcodec/avcodec.h>
#include <libavutil/avutil.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavutil/opt.h>
#include <libavutil/pixdesc.h>
}

#define VK_LOAD_PFN(inst, name) (PFN_##name) vkGetInstanceProcAddr(inst, #name)

CEncoder::CEncoder(std::shared_ptr<ClientConnection> listener,
                   std::shared_ptr<PoseHistory> poseHistory)
    : m_listener(listener), m_poseHistory(poseHistory) {}

CEncoder::~CEncoder() { Stop(); }

namespace {
void read_exactly(int fd, char *out, size_t size, std::atomic_bool &exiting) {
    while (not exiting and size != 0) {
        timeval timeout{.tv_sec = 0, .tv_usec = 15000};
        fd_set read_fd, write_fd, except_fd;
        FD_ZERO(&read_fd);
        FD_SET(fd, &read_fd);
        FD_ZERO(&write_fd);
        FD_ZERO(&except_fd);
        int count = select(fd + 1, &read_fd, &write_fd, &except_fd, &timeout);
        if (count < 0) {
            throw MakeException("select failed: %s", strerror(errno));
        } else if (count == 1) {
            int s = read(fd, out, size);
            if (s == -1) {
                throw MakeException("read failed: %s", strerror(errno));
            }
            out += s;
            size -= s;
        }
    }
}

int accept_timeout(int socket, std::atomic_bool &exiting) {
    while (not exiting) {
        timeval timeout{.tv_sec = 0, .tv_usec = 15000};
        fd_set read_fd, write_fd, except_fd;
        FD_ZERO(&read_fd);
        FD_SET(socket, &read_fd);
        FD_ZERO(&write_fd);
        FD_ZERO(&except_fd);
        int count = select(socket + 1, &read_fd, &write_fd, &except_fd, &timeout);
        if (count < 0) {
            throw MakeException("select failed: %s", strerror(errno));
        } else if (count == 1) {
          return accept(socket, NULL, NULL);
        }
    }
    return -1;
}

void skipAUD_h265(uint8_t **buffer, int *length) {
  // H.265 encoder always produces AUD NAL even if AMF_VIDEO_ENCODER_HEVC_INSERT_AUD is set. But it is not needed.
  static const int AUD_NAL_SIZE = 7;

  if (*length < AUD_NAL_SIZE + 4) {
    return;
  }

  // Check if start with AUD NAL.
  if (memcmp(*buffer, "\x00\x00\x00\x01\x46", 5) != 0) {
    return;
  }
  // Check if AUD NAL size is AUD_NAL_SIZE bytes.
  if (memcmp(*buffer + AUD_NAL_SIZE, "\x00\x00\x00\x01", 4) != 0) {
    return;
  }
  *buffer += AUD_NAL_SIZE;
  *length -= AUD_NAL_SIZE;
}

class AvException: public std::runtime_error
{
public:
  AvException(std::string msg, int averror): std::runtime_error{makemsg(msg, averror)} {}
private:
  static std::string makemsg(const std::string & msg, int averror)
  {
    char av_msg[AV_ERROR_MAX_STRING_SIZE];
    av_strerror(averror, av_msg, sizeof(av_msg));
    return msg + " " + av_msg;
  }
};

const char * encoder(int codec)
{
  switch (codec)
  {
    case ALVR_CODEC_H264:
      return "h264_vaapi";
    case ALVR_CODEC_H265:
      return "hevc_vaapi";
  }
  throw std::runtime_error("invalid codec " + std::to_string(codec));
}

void set_hwframe_ctx(AVCodecContext *ctx, AVBufferRef *hw_device_ctx, int width, int height)
{
  AVBufferRef *hw_frames_ref;
  AVHWFramesContext *frames_ctx = NULL;
  int err = 0;

  if (!(hw_frames_ref = av_hwframe_ctx_alloc(hw_device_ctx))) {
    throw std::runtime_error("Failed to create VAAPI frame context.");
  }
  frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
  frames_ctx->format = AV_PIX_FMT_VAAPI;
  frames_ctx->sw_format = AV_PIX_FMT_NV12;
  frames_ctx->width = width;
  frames_ctx->height = height;
  frames_ctx->initial_pool_size = 10;
  if ((err = av_hwframe_ctx_init(hw_frames_ref)) < 0) {
    av_buffer_unref(&hw_frames_ref);
    throw AvException("Failed to initialize VAAPI frame context:", err);
  }
  ctx->hw_frames_ctx = av_buffer_ref(hw_frames_ref);
  if (!ctx->hw_frames_ctx)
    err = AVERROR(ENOMEM);

  av_buffer_unref(&hw_frames_ref);
}

// it seems that ffmpeg does not provide this mapping
AVPixelFormat vk_format_to_av_format(VkFormat vk_fmt)
{
  for (int f = AV_PIX_FMT_NONE; f < AV_PIX_FMT_NB; ++f)
  {
    auto current_fmt = av_vkfmt_from_pixfmt(AVPixelFormat(f));
    if (current_fmt and *current_fmt == vk_fmt)
      return AVPixelFormat(f);
  }
  throw MakeException("unsupported pixel format %i", vk_fmt);
}

AVBufferRef* make_vk_hwframe_ctx(AVBufferRef *hw_device_ctx, VkImageCreateInfo info)
{
  AVBufferRef *hw_frames_ref;
  AVHWFramesContext *frames_ctx = NULL;
  int err = 0;

  if (!(hw_frames_ref = av_hwframe_ctx_alloc(hw_device_ctx))) {
    throw std::runtime_error("Failed to create vulkan frame context.");
  }
  frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
  frames_ctx->format = AV_PIX_FMT_VULKAN;
  frames_ctx->sw_format = vk_format_to_av_format(info.format);
  frames_ctx->width = info.extent.width;
  frames_ctx->height = info.extent.height;
  frames_ctx->initial_pool_size = 0;
  if ((err = av_hwframe_ctx_init(hw_frames_ref)) < 0) {
    av_buffer_unref(&hw_frames_ref);
    throw AvException("Failed to initialize vulkan frame context:", err);
  }
  return hw_frames_ref;
}

#ifdef DEBUG
void logfn(void*, int level, const char* data, va_list va)
{
  vfprintf(stderr, data, va);
}
#endif

} // namespace

void CEncoder::GetFds(int client, int (*received_fds)[6]) {
    struct msghdr msg;
    struct cmsghdr *cmsg;
    union {
        struct cmsghdr cm;
        u_int8_t pktinfo_sizer[sizeof(struct cmsghdr) + 1024];
    } control_un;
    struct iovec iov[1];
    char data[1];
    int ret;

    msg.msg_control = &control_un;
    msg.msg_controllen = sizeof(control_un);
    msg.msg_flags = 0;
    msg.msg_name = NULL;
    msg.msg_namelen = 0;
    iov[0].iov_base = data;
    iov[0].iov_len = 1;
    msg.msg_iov = iov;
    msg.msg_iovlen = 1;

    ret = recvmsg(client, &msg, 0);
    if (ret == -1) {
      throw MakeException("recvmsg failed: %s", strerror(errno));
    }

    for (cmsg = CMSG_FIRSTHDR(&msg); cmsg != NULL; cmsg = CMSG_NXTHDR(&msg, cmsg)) {
        if (cmsg->cmsg_level == SOL_SOCKET && cmsg->cmsg_type == SCM_RIGHTS) {
            memcpy(received_fds, CMSG_DATA(cmsg), sizeof(*received_fds));
            break;
        }
    }

    if (cmsg == NULL) {
      throw MakeException("cmsg is NULL");
    }
}

void CEncoder::UpdatePoseIndex()
{
  std::array<vr::Compositor_FrameTiming, 5> frames;
  frames[0].m_nSize = sizeof(vr::Compositor_FrameTiming);
  if (vr::VRServerDriverHost()->IsExiting())
  {
    return;
  }
  // find the latest frame that has been presented
  uint32_t s = vr::VRServerDriverHost()->GetFrameTimings(frames.data(), frames.size());
  for (int i = s-1 ; i >= 0 ; --i)
  {
    if (frames[i].m_nNumFramePresents > 0) {
      auto pose = m_poseHistory->GetBestPoseMatch(frames[i].m_HmdPose.mDeviceToAbsoluteTracking);
      if (pose)
        m_poseSubmitIndex = pose->info.FrameIndex;
      return;
    }
  }

}


void CEncoder::Run() {
    Info("CEncoder::Run\n");
    m_socketPath = getenv("XDG_RUNTIME_DIR");
    m_socketPath += "/alvr-ipc";

    int ret;
    // we don't really care about what happends with unlink, it's just incase we crashed before this
    // run
    ret = unlink(m_socketPath.c_str());

    m_socket = socket(AF_UNIX, SOCK_STREAM, 0);
    struct sockaddr_un name;
    if (m_socket == -1) {
        perror("socket");
        exit(1);
    }

    memset(&name, 0, sizeof(name));
    name.sun_family = AF_UNIX;
    strncpy(name.sun_path, m_socketPath.c_str(), sizeof(name.sun_path) - 1);

    ret = bind(m_socket, (const struct sockaddr *)&name, sizeof(name));
    if (ret == -1) {
        perror("bind");
        exit(1);
    }

    ret = listen(m_socket, 1024);
    if (ret == -1) {
        perror("listen");
        exit(1);
    }

    Info("CEncoder Listening\n");
    int client = accept_timeout(m_socket, m_exiting);
    if (m_exiting)
      return;
    init_packet init;
    read_exactly(client, (char *)&init, sizeof(init), m_exiting);
    if (m_exiting)
      return;
    char ifbuf[256];
    char ifbuf2[256];
    sprintf(ifbuf, "/proc/%d/cmdline", (int)init.source_pid);
    std::ifstream ifscmdl(ifbuf);
    ifscmdl >> ifbuf2;
    Info("CEncoder client connected, pid %d, cmdline %s\n", (int)init.source_pid, ifbuf2);

    try {
      GetFds(client, &m_fds);
      //
      // We have everything we need, it is time to initalize Vulkan.
      //
      // putenv("VK_APIDUMP_LOG_FILENAME=\"/home/ron/alvr_vrdump.txt\"");
      fprintf(stderr, "\n\nWe are initalizing Vulkan in CEncoder thread\n\n\n");

#ifdef DEBUG
      av_log_set_level(AV_LOG_DEBUG);
      av_log_set_callback(logfn);
#endif

      static char e1[] = "VK_INSTANCE_LAYERS";//=VK_LAYER_KHRONOS_validation";
      static char e2[] = "DISABLE_ALVR_DISPLAY=1";
      putenv(e1);
      putenv(e2);
      AVBufferRef *vulkan_ctx;
      AVDictionary *d = NULL; // "create" an empty dictionary
      //av_dict_set(&d, "debug", "1", 0); // add an entry
      ret = av_hwdevice_ctx_create(&vulkan_ctx, AV_HWDEVICE_TYPE_VULKAN, init.device_name.data(), d, 0);
      if (ret) {
        throw AvException("failed to initialize vulkan", ret);
      }

      AVHWDeviceContext *hwctx = (AVHWDeviceContext *)vulkan_ctx->data;
      AVVulkanDeviceContext *vkctx = (AVVulkanDeviceContext *)hwctx->hwctx;
      vk::Device device = vkctx->act_dev;
      auto vk_frame_ctx = make_vk_hwframe_ctx(vulkan_ctx, init.image_create_info);

      AVVkFrame *images[3];

      init.image_create_info.initialLayout =
        VK_IMAGE_LAYOUT_UNDEFINED; // VUID-VkImageCreateInfo-pNext-01443
      assert(init.image_create_info.queueFamilyIndexCount == 0);
      assert(init.image_create_info.pNext == NULL);
      for (size_t i = 0; i < 3; ++i) {
        vk::ExternalMemoryImageCreateInfo extMemImageInfo;
        extMemImageInfo.handleTypes = vk::ExternalMemoryHandleTypeFlagBits::eOpaqueFd;
        init.image_create_info.pNext = &extMemImageInfo;
        vk::Image image = device.createImage(init.image_create_info);

        auto req = device.getImageMemoryRequirements(image);

        vk::MemoryDedicatedAllocateInfo dedicatedMemInfo;
        dedicatedMemInfo.image = image;

        vk::ImportMemoryFdInfoKHR importMemInfo;
        importMemInfo.pNext = &dedicatedMemInfo;
        importMemInfo.handleType = vk::ExternalMemoryHandleTypeFlagBits::eOpaqueFd;
        importMemInfo.fd = m_fds[2*i];

        vk::MemoryAllocateInfo memAllocInfo;
        memAllocInfo.pNext = &importMemInfo;
        memAllocInfo.allocationSize = req.size;
        memAllocInfo.memoryTypeIndex = init.mem_index;

        vk::DeviceMemory mem = device.allocateMemory(memAllocInfo);
        device.bindImageMemory(image, mem, 0);

        vk::SemaphoreCreateInfo semInfo;
        vk::Semaphore semaphore = device.createSemaphore(semInfo);

        vk::ImportSemaphoreFdInfoKHR impSemInfo;
        impSemInfo.semaphore = semaphore;
        impSemInfo.handleType = vk::ExternalSemaphoreHandleTypeFlagBits::eOpaqueFd;
        impSemInfo.fd = m_fds[2*i+1];

        struct {
          PFN_vkImportSemaphoreFdKHR vkImportSemaphoreFdKHR;
        } d;
        d.vkImportSemaphoreFdKHR = VK_LOAD_PFN(vkctx->inst, vkImportSemaphoreFdKHR);

        device.importSemaphoreFdKHR(impSemInfo, d);

        images[i] = av_vk_frame_alloc();
        images[i]->img[0] = image;
        images[i]->tiling = init.image_create_info.tiling;
        images[i]->mem[0] = mem;
        images[i]->size[0] = req.size;
        images[i]->layout[0] = VK_IMAGE_LAYOUT_UNDEFINED;
        images[i]->sem[0] = semaphore;
      }

      AVBufferRef *encoder_ctx;
      //int err = av_hwdevice_ctx_create_derived(&encoder_ctx, AV_HWDEVICE_TYPE_VAAPI, vulkan_ctx, 0);
      int err = av_hwdevice_ctx_create(&encoder_ctx, AV_HWDEVICE_TYPE_VAAPI, NULL, NULL, 0);
      if (err < 0) {
        throw AvException("Failed to create a VAAPI device:", err);
      }

      const int codec_id = Settings::Instance().m_codec;
      const char * encoder_name = encoder(codec_id);
      AVCodec *codec = avcodec_find_encoder_by_name(encoder_name);
      if (codec == nullptr)
      {
        throw std::runtime_error(std::string("Failed to find encoder ") + encoder_name);
      }


      std::unique_ptr<AVCodecContext, std::function<void(AVCodecContext*)>> avctx{
        avcodec_alloc_context3(codec),
          [](AVCodecContext *p) {avcodec_free_context(&p);}
      };

      switch (codec_id)
      {
        case ALVR_CODEC_H264:
          av_opt_set(avctx.get(), "profile", "100", 0);//high
          av_opt_set(avctx.get(), "rc_mode", "2", 0); //CBR
          break;
        case ALVR_CODEC_H265:
          av_opt_set(avctx.get(), "profile", "1", 0);//main
          av_opt_set(avctx.get(), "rc_mode", "2", 0);
          break;
      }

      avctx->width = init.image_create_info.extent.width;
      avctx->height = init.image_create_info.extent.height;
      avctx->time_base = {std::chrono::steady_clock::period::num, std::chrono::steady_clock::period::den};
      avctx->framerate = AVRational{Settings::Instance().m_refreshRate, 1}; // framerate will be forced by vsync
      avctx->sample_aspect_ratio = AVRational{1, 1};
      avctx->pix_fmt = AV_PIX_FMT_VAAPI;
      avctx->max_b_frames = 0;

      avctx->bit_rate = Settings::Instance().mEncodeBitrateMBs * 1024 * 1024;

      /* set hw_frames_ctx for encoder's AVCodecContext */
      set_hwframe_ctx(avctx.get(), encoder_ctx, avctx->width, avctx->height);

      if ((err = avcodec_open2(avctx.get(), codec, NULL)) < 0) {
        throw AvException("Cannot open video encoder codec:", err);
      }

      auto filter_in = avfilter_get_by_name("buffer");
      auto filter_out = avfilter_get_by_name("buffersink");

      std::unique_ptr<AVFilterGraph, std::function<void(AVFilterGraph*)>> graph{
        avfilter_graph_alloc(),
          [](AVFilterGraph* p) {avfilter_graph_free(&p);}
      };

      AVFilterInOut *outputs = avfilter_inout_alloc();
      AVFilterInOut *inputs = avfilter_inout_alloc();

      AVFilterContext *filter_in_ctx = avfilter_graph_alloc_filter(graph.get(), filter_in, "in");

      AVBufferSrcParameters *par = av_buffersrc_parameters_alloc();
      memset(par, 0, sizeof(*par));
      par->width = init.image_create_info.extent.width;
      par->height = init.image_create_info.extent.height;
      par->time_base = {std::chrono::steady_clock::period::num, std::chrono::steady_clock::period::den};
      par->format = AV_PIX_FMT_VULKAN;
      par->hw_frames_ctx = av_buffer_ref(vk_frame_ctx);
      av_buffersrc_parameters_set(filter_in_ctx, par);
      av_free(par);

      AVFilterContext *filter_out_ctx;
      if ((err = avfilter_graph_create_filter(&filter_out_ctx, filter_out, "out", NULL, NULL, graph.get())))
      {
        throw AvException("filter_out creation failed:", err);
      }

      outputs->name = av_strdup("in");
      outputs->filter_ctx = filter_in_ctx;
      outputs->pad_idx = 0;
      outputs->next = NULL;

      inputs->name = av_strdup("out");
      inputs->filter_ctx = filter_out_ctx;
      inputs->pad_idx = 0;
      inputs->next = NULL;

      if ((err = avfilter_graph_parse_ptr(graph.get(), "hwmap, scale_vaapi=format=nv12",
              &inputs, &outputs, NULL)) < 0)
      {
        throw AvException("avfilter_graph_parse_ptr failed:", err);
      }

      avfilter_inout_free(&outputs);
      avfilter_inout_free(&inputs);

      for (unsigned i = 0 ; i < graph->nb_filters; ++i)
      {
        graph->filters[i]->hw_device_ctx= av_buffer_ref(encoder_ctx);
      }

      if ((err = avfilter_graph_config(graph.get(), NULL)))
      {
        throw AvException("avfilter_graph_config failed:", err);
      }

      AVFrame *encoder_frame = av_frame_alloc();

      fprintf(stderr, "CEncoder starting to read present packets");
      present_packet frame_info;
      auto epoch = std::chrono::steady_clock::now();
      while (not m_exiting) {
        read_exactly(client, (char *)&frame_info, sizeof(frame_info), m_exiting);

        auto encode_start = std::chrono::steady_clock::now();
        UpdatePoseIndex();

        AVFrame *in_frame = av_frame_alloc();
        in_frame->width = init.image_create_info.extent.width;
        in_frame->height = init.image_create_info.extent.height;
        in_frame->hw_frames_ctx = av_buffer_ref(vk_frame_ctx);
        in_frame->data[0] = (uint8_t*)images[frame_info.image];
        in_frame->format = AV_PIX_FMT_VULKAN;
        in_frame->buf[0] = av_buffer_alloc(1);
        in_frame->pts = (encode_start - epoch).count();
        static_assert(std::is_same_v<std::chrono::steady_clock::duration::rep, int64_t>);

        err = av_buffersrc_add_frame_flags(filter_in_ctx, in_frame, AV_BUFFERSRC_FLAG_PUSH);
        if (err != 0)
        {
          throw AvException("av_buffersrc_add_frame failed", err);
        }
        err = av_buffersink_get_frame(filter_out_ctx, encoder_frame);
        if (err != 0)
        {
          throw AvException("av_buffersink_get_frame failed", err);
        }
        av_frame_free(&in_frame);

        if ((err = avcodec_send_frame(avctx.get(), encoder_frame)) < 0) {
          throw AvException("avcodec_send_frame failed: ", err);
        }
        av_frame_unref(encoder_frame);
        write(client, &frame_info.image, sizeof(frame_info.image));

        bool first_packet = true;
        while (1) {
          AVPacket enc_pkt;
          av_init_packet(&enc_pkt);
          enc_pkt.data = NULL;
          enc_pkt.size = 0;

          err = avcodec_receive_packet(avctx.get(), &enc_pkt);
          if (err == AVERROR(EAGAIN)) {
            break;
          } else if (err) {
            throw std::runtime_error("failed to encode");
          }
          uint8_t *frame_data = (uint8_t*)enc_pkt.data;
          int frame_size = enc_pkt.size;
          if (first_packet and codec_id == ALVR_CODEC_H265)
          {
            skipAUD_h265(&frame_data, &frame_size);
            first_packet = false;
          }
          m_listener->FECSend(frame_data, frame_size, m_poseSubmitIndex, frame_info.frame);
          enc_pkt.stream_index = 0;
          av_packet_unref(&enc_pkt);
        }

        auto encode_end = std::chrono::steady_clock::now();

        m_listener->GetStatistics()->EncodeOutput(std::chrono::duration_cast<std::chrono::microseconds>(encode_end - encode_start).count());

      }
      av_buffer_unref(&encoder_ctx);
      av_frame_free(&encoder_frame);
    }
    catch (std::exception &e) {
      std::stringstream err;
      err << "error in encoder thread: " << e.what();
      Error(err.str().c_str());
    }

    close(client);
}

void CEncoder::Stop() {
    m_exiting = true;
    close(m_socket);
    unlink(m_socketPath.c_str());
}

void CEncoder::OnPacketLoss() { m_scheduler.OnPacketLoss(); }

void CEncoder::InsertIDR() { m_scheduler.InsertIDR(); }
