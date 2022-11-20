#include "CEncoder.h"

#include <chrono>
#include <exception>
#include <memory>
#include <poll.h>
#include <sstream>
#include <stdexcept>
#include <stdlib.h>
#include <string>
#include <sys/mman.h>
#include <sys/poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <iostream>

#include "ALVR-common/packet_types.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/Logger.h"
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Statistics.h"
#include "alvr_server/bindings.h"
#include "protocol.h"
#include "ffmpeg_helper.h"
#include "EncodePipeline.h"
#include "FrameRender.h"

extern "C" {
#include <libavutil/avutil.h>
}

CEncoder::CEncoder(std::shared_ptr<ClientConnection> listener,
                   std::shared_ptr<PoseHistory> poseHistory)
    : m_listener(listener), m_poseHistory(poseHistory) {}

CEncoder::~CEncoder() { Stop(); }

namespace {
void read_exactly(pollfd pollfds, char *out, size_t size, std::atomic_bool &exiting) {
    while (not exiting and size != 0) {
        int timeout = 1; // poll api doesn't fit perfectly(100 mircoseconds) poll uses milliseconds we do the best we can(1000 mircoseconds)
        pollfds.events = POLLIN;
        int count = poll(&pollfds, 1, timeout);
        if (count < 0) {
            throw MakeException("poll failed: %s", strerror(errno));
        } else if (count == 1) {
            int s = read(pollfds.fd, out, size);
            if (s == -1) {
                throw MakeException("read failed: %s", strerror(errno));
            }
            out += s;
            size -= s;
        }
    }
}

void read_latest(pollfd pollfds, char *out, size_t size, std::atomic_bool &exiting) {
    read_exactly(pollfds, out, size, exiting);
    while (not exiting)
    {
        int timeout = 0; // poll api fixes the original perfectly(0 microseconds)
        pollfds.events = POLLIN;
        int count = poll(&pollfds, 1 , timeout);
        if (count == 0)
            return;
        read_exactly(pollfds, out, size, exiting);
    }
}

int accept_timeout(pollfd socket, std::atomic_bool &exiting) {
    while (not exiting) {
        int timeout = 15; // poll api also fits the original perfectly(15000 microseconds)
        socket.events = POLLIN;
        int count = poll(&socket, 1, timeout);
        if (count < 0) {
            throw MakeException("poll failed: %s", strerror(errno));
        } else if (count == 1) {
          return accept(socket.fd, NULL, NULL);
        }
    }
    return -1;
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

    m_socket.fd = socket(AF_UNIX, SOCK_STREAM, 0);
    struct sockaddr_un name;
    if (m_socket.fd == -1) {
        perror("socket");
        exit(1);
    }

    memset(&name, 0, sizeof(name));
    name.sun_family = AF_UNIX;
    strncpy(name.sun_path, m_socketPath.c_str(), sizeof(name.sun_path) - 1);

    ret = bind(m_socket.fd, (const struct sockaddr *)&name, sizeof(name));
    if (ret == -1) {
        perror("bind");
        exit(1);
    }

    ret = listen(m_socket.fd, 1024);
    if (ret == -1) {
        perror("listen");
        exit(1);
    }

    Info("CEncoder Listening\n");
    struct pollfd client;
    client.fd = accept_timeout(m_socket, m_exiting);
    if (m_exiting)
      return;
    init_packet init;
    client.events = POLLIN;
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
        GetFds(client.fd, &m_fds);

      fprintf(stderr, "\n\nWe are initalizing Vulkan in CEncoder thread\n\n\n");

#ifdef DEBUG
      AVUTIL.av_log_set_level(AV_LOG_DEBUG);
      AVUTIL.av_log_set_callback(logfn);
#endif

      AVDictionary *d = NULL; // "create" an empty dictionary
      //av_dict_set(&d, "debug", "1", 0); // add an entry
      alvr::VkContext vk_ctx(init.device_name.data(), d);

      FrameRender render(vk_ctx.get_vk_instance(), vk_ctx.get_vk_device(), vk_ctx.get_vk_phys_device(), vk_ctx.get_vk_device_extensions());

      render.Startup(init.image_create_info.extent.width, init.image_create_info.extent.height, init.image_create_info.format, vk_ctx.get_vk_queue_families());
      for (size_t i = 0; i < 3; ++i) {
          render.AddImage(init.image_create_info, init.mem_index, m_fds[2*i], m_fds[2*i+1]);
      }

      RenderPipeline quad(&render);
      quad.SetShader(RenderPipeline::VertexShader, QUAD_SHADER_VERT_SPV_PTR, QUAD_SHADER_VERT_SPV_LEN);
      quad.SetShader(RenderPipeline::FragmentShader, QUAD_SHADER_FRAG_SPV_PTR, QUAD_SHADER_FRAG_SPV_LEN);
      render.AddPipeline(&quad);

      RenderPipeline color(&render);
      if (Settings::Instance().m_enableColorCorrection) {
          struct ColorCorrection {
              float renderWidth;
              float renderHeight;
              float brightness;
              float contrast;
              float saturation;
              float gamma;
              float sharpening;
              float _align;
          };
          ColorCorrection *cc = new ColorCorrection;
          cc->renderWidth = Settings::Instance().m_renderWidth;
          cc->renderHeight = Settings::Instance().m_renderHeight;
          cc->brightness = Settings::Instance().m_brightness;
          cc->contrast = Settings::Instance().m_contrast + 1.f;
          cc->saturation = Settings::Instance().m_saturation + 1.f;
          cc->gamma = Settings::Instance().m_gamma;
          cc->sharpening = Settings::Instance().m_sharpening;

          color.SetShader(RenderPipeline::VertexShader, QUAD_SHADER_VERT_SPV_PTR, QUAD_SHADER_VERT_SPV_LEN);
          color.SetShader(RenderPipeline::FragmentShader, COLOR_SHADER_FRAG_SPV_PTR, COLOR_SHADER_FRAG_SPV_LEN);
          color.SetPushConstant(RenderPipeline::FragmentShader, cc, sizeof(ColorCorrection));
          render.AddPipeline(&color);
      }

      auto output = render.CreateOutput(init.image_create_info.extent.width, init.image_create_info.extent.height);

      alvr::VkFrameCtx vk_frame_ctx(vk_ctx, output.imageInfo);

      std::vector<alvr::VkFrame> images;
      images.reserve(1);
      images.emplace_back(vk_ctx, output.image, output.imageInfo, output.size, output.memory);

      auto encode_pipeline = alvr::EncodePipeline::Create(images, vk_frame_ctx);

      fprintf(stderr, "CEncoder starting to read present packets");
      present_packet frame_info;
      std::vector<uint8_t> encoded_data;
      while (not m_exiting) {
        read_latest(client, (char *)&frame_info, sizeof(frame_info), m_exiting);

        if (m_listener->GetStatistics()->CheckBitrateUpdated()) {
          encode_pipeline->SetBitrate(m_listener->GetStatistics()->GetBitrate() * 1000000L); // in bits;
        }

        auto pose = m_poseHistory->GetBestPoseMatch((const vr::HmdMatrix34_t&)frame_info.pose);
        if (!pose)
        {
          continue;
        }

        // Linux does not really have a present event. This place is the closest one.
        ReportPresent(pose->targetTimestampNs);

        render.Render(frame_info.image, frame_info.semaphore_value);

        // Linux has currently no compositor. Report frame has been composed right away
        ReportComposed(pose->targetTimestampNs);

        auto encode_start = std::chrono::steady_clock::now();
        encode_pipeline->PushFrame(0, pose->targetTimestampNs, m_scheduler.CheckIDRInsertion());

        static_assert(sizeof(frame_info.pose) == sizeof(vr::HmdMatrix34_t&));

        encoded_data.clear();
        uint64_t pts;
        // Encoders can req more then once frame, need to accumulate more data before sending it to the client
        if (!encode_pipeline->GetEncoded(encoded_data, &pts)) {
          continue;
        }

        m_listener->SendVideo(encoded_data.data(), encoded_data.size(), pts);

        auto encode_end = std::chrono::steady_clock::now();

        m_listener->GetStatistics()->EncodeOutput();

      }
    }
    catch (std::exception &e) {
      std::stringstream err;
      err << "error in encoder thread: " << e.what();
      Error(err.str().c_str());
    }

    client.events = POLLHUP;
    close(client.fd);
}

void CEncoder::Stop() {
    m_exiting = true;
    m_socket.events = POLLHUP;
    close(m_socket.fd);
    unlink(m_socketPath.c_str());
}

void CEncoder::OnPacketLoss() { m_scheduler.OnPacketLoss(); }

void CEncoder::InsertIDR() { m_scheduler.InsertIDR(); }
