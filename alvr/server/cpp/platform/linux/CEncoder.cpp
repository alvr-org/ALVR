#include "CEncoder.h"

#include <algorithm>
#include <chrono>
#include <exception>
#include <memory>
#include <sstream>
#include <openvr_driver.h>
#include <stdexcept>
#include <stdlib.h>
#include <string>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include "ALVR-common/packet_types.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/Logger.h"
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Statistics.h"
#include "protocol.h"
#include "ffmpeg_helper.h"

extern "C" {
#include <libavutil/hwcontext.h>
#include <libavcodec/avcodec.h>
#include <libavutil/avutil.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavutil/opt.h>
}

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
    throw alvr::AvException("Failed to initialize VAAPI frame context:", err);
  }
  ctx->hw_frames_ctx = av_buffer_ref(hw_frames_ref);
  if (!ctx->hw_frames_ctx)
    err = AVERROR(ENOMEM);

  av_buffer_unref(&hw_frames_ref);
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

    // check that pointer types are null, other values would not make sense over a socket
    assert(init.image_create_info.queueFamilyIndexCount == 0);
    assert(init.image_create_info.pNext == NULL);

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
      AVDictionary *d = NULL; // "create" an empty dictionary
      //av_dict_set(&d, "debug", "1", 0); // add an entry
      alvr::VkContext vk_ctx(init.device_name.data(), d);
      alvr::VkFrameCtx vk_frame_ctx(vk_ctx, init.image_create_info);

      std::vector<alvr::VkFrame> images;
      images.reserve(3);
      for (size_t i = 0; i < 3; ++i) {
        images.emplace_back(vk_ctx, init.image_create_info, init.mem_index, m_fds[2*i], m_fds[2*i+1]);
      }

      AVBufferRef *encoder_ctx;
      //int err = av_hwdevice_ctx_create_derived(&encoder_ctx, AV_HWDEVICE_TYPE_VAAPI, vulkan_ctx, 0);
      int err = av_hwdevice_ctx_create(&encoder_ctx, AV_HWDEVICE_TYPE_VAAPI, NULL, NULL, 0);
      if (err < 0) {
        throw alvr::AvException("Failed to create a VAAPI device:", err);
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
          avctx->profile = FF_PROFILE_H264_MAIN;
          av_opt_set(avctx.get(), "rc_mode", "2", 0); //CBR
          break;
        case ALVR_CODEC_H265:
          avctx->profile = FF_PROFILE_HEVC_MAIN;
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
        throw alvr::AvException("Cannot open video encoder codec:", err);
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
      par->format = AV_PIX_FMT_VAAPI;
      par->hw_frames_ctx = av_buffer_ref(avctx->hw_frames_ctx);
      av_buffersrc_parameters_set(filter_in_ctx, par);
      av_free(par);

      AVFilterContext *filter_out_ctx;
      if ((err = avfilter_graph_create_filter(&filter_out_ctx, filter_out, "out", NULL, NULL, graph.get())))
      {
        throw alvr::AvException("filter_out creation failed:", err);
      }

      outputs->name = av_strdup("in");
      outputs->filter_ctx = filter_in_ctx;
      outputs->pad_idx = 0;
      outputs->next = NULL;

      inputs->name = av_strdup("out");
      inputs->filter_ctx = filter_out_ctx;
      inputs->pad_idx = 0;
      inputs->next = NULL;

      if ((err = avfilter_graph_parse_ptr(graph.get(), "scale_vaapi=format=nv12",
              &inputs, &outputs, NULL)) < 0)
      {
        throw alvr::AvException("avfilter_graph_parse_ptr failed:", err);
      }

      avfilter_inout_free(&outputs);
      avfilter_inout_free(&inputs);

      for (unsigned i = 0 ; i < graph->nb_filters; ++i)
      {
        graph->filters[i]->hw_device_ctx= av_buffer_ref(encoder_ctx);
      }

      if ((err = avfilter_graph_config(graph.get(), NULL)))
      {
        throw alvr::AvException("avfilter_graph_config failed:", err);
      }

      AVFrame *encoder_frame = av_frame_alloc();

      std::vector<AVFrame *> mapped_frames;

      for (size_t i = 0 ; i < images.size(); ++i)
      {
        AVFrame * mapped_frame = av_frame_alloc();
        av_hwframe_get_buffer(avctx->hw_frames_ctx, mapped_frame, 0);
        auto vk_frame = images[i].make_av_frame(vk_frame_ctx);
        av_hwframe_map(mapped_frame, vk_frame.get(), AV_HWFRAME_MAP_READ);
        mapped_frames.push_back(mapped_frame);
      }

      fprintf(stderr, "CEncoder starting to read present packets");
      present_packet frame_info;
      std::vector<uint8_t> encoded_frame;
      while (not m_exiting) {
        read_exactly(client, (char *)&frame_info, sizeof(frame_info), m_exiting);

        auto encode_start = std::chrono::steady_clock::now();

        static_assert(sizeof(frame_info.pose) == sizeof(vr::HmdMatrix34_t&));
        auto pose = m_poseHistory->GetBestPoseMatch((const vr::HmdMatrix34_t&)frame_info.pose);
        if (not pose)
          continue;
        m_poseSubmitIndex = pose->info.FrameIndex;

        err = av_buffersrc_add_frame_flags(filter_in_ctx, mapped_frames[frame_info.image], AV_BUFFERSRC_FLAG_PUSH | AV_BUFFERSRC_FLAG_KEEP_REF);
        if (err != 0)
        {
          throw alvr::AvException("av_buffersrc_add_frame failed", err);
        }
        err = av_buffersink_get_frame(filter_out_ctx, encoder_frame);
        if (err != 0)
        {
          throw alvr::AvException("av_buffersink_get_frame failed", err);
        }

        if (m_scheduler.CheckIDRInsertion())
        {
          encoder_frame->pict_type = AV_PICTURE_TYPE_I;
        } else {
          encoder_frame->pict_type = AV_PICTURE_TYPE_NONE;
        }
        encoder_frame->pts = encode_start.time_since_epoch().count();

        if ((err = avcodec_send_frame(avctx.get(), encoder_frame)) < 0) {
          throw alvr::AvException("avcodec_send_frame failed: ", err);
        }
        av_frame_unref(encoder_frame);

        encoded_frame.clear();
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
          encoded_frame.insert(encoded_frame.end(), enc_pkt.data, enc_pkt.data + enc_pkt.size);
          enc_pkt.stream_index = 0;
          av_packet_unref(&enc_pkt);
        }

        uint8_t *frame_data = encoded_frame.data();
        int frame_size = encoded_frame.size();
        if (codec_id == ALVR_CODEC_H265)
        {
          skipAUD_h265(&frame_data, &frame_size);
        }
        m_listener->SendVideo(frame_data, frame_size, m_poseSubmitIndex + Settings::Instance().m_trackingFrameOffset);

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
