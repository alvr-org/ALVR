#include "CEncoder.h"

#include <chrono>
#include <exception>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <stdlib.h>
#include <string>
#include <sys/mman.h>
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
#include "EncodePipeline.h"

extern "C" {
#include <libavutil/avutil.h>
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

#ifdef DEBUG
void logfn(void*, int level, const char* data, va_list va)
{
  vfprintf(stderr, data, va);
}
#endif

} // namespace

std::vector<int> get_fds(int client, size_t num_fds) {
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

    std::vector<int> res(num_fds, 0);
    for (cmsg = CMSG_FIRSTHDR(&msg); cmsg != NULL; cmsg = CMSG_NXTHDR(&msg, cmsg)) {
        if (cmsg->cmsg_level == SOL_SOCKET && cmsg->cmsg_type == SCM_RIGHTS) {
            memcpy(res.data(), CMSG_DATA(cmsg), sizeof(int) * num_fds);
            break;
        }
    }

    if (cmsg == NULL) {
      throw MakeException("cmsg is NULL");
    }
    return res;
}

void CEncoder::Run() {
    Info("CEncoder::Run\n");
    std::string socketPath = getenv("XDG_RUNTIME_DIR");
    socketPath += "/alvr-ipc";

    int ret;
    // we don't really care about what happends with unlink, it's just incase we crashed before this
    // run
    ret = unlink(socketPath.c_str());

    int socket_fd = socket(AF_UNIX, SOCK_STREAM, 0);
    struct sockaddr_un name;
    if (socket_fd == -1) {
        perror("socket");
        exit(1);
    }

    memset(&name, 0, sizeof(name));
    name.sun_family = AF_UNIX;
    strncpy(name.sun_path, socketPath.c_str(), sizeof(name.sun_path) - 1);

    ret = bind(socket_fd, (const struct sockaddr *)&name, sizeof(name));
    if (ret == -1) {
        perror("bind");
        exit(1);
    }

    ret = listen(socket_fd, 1024);
    if (ret == -1) {
        perror("listen");
        exit(1);
    }

    Info("CEncoder Listening\n");
    int client = accept_timeout(socket_fd, m_exiting);
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
      std::vector<int> fds = get_fds(client, init.num_images * 2 + 1);
      close(client);
      close(socket_fd);
      unlink(socketPath.c_str());

      present_shm *shm = (present_shm *)mmap(NULL, sizeof(present_shm) + init.num_images * sizeof(present_info), PROT_READ | PROT_WRITE, MAP_SHARED, fds[0], 0);

      fprintf(stderr, "\n\nWe are initalizing Vulkan in CEncoder thread\n\n\n");

#ifdef DEBUG
      AVUTIL.av_log_set_level(AV_LOG_DEBUG);
      AVUTIL.av_log_set_callback(logfn);
#endif

      AVDictionary *d = NULL; // "create" an empty dictionary
      //av_dict_set(&d, "debug", "1", 0); // add an entry
      alvr::VkContext vk_ctx(init.device_name.data(), d);
      alvr::VkFrameCtx vk_frame_ctx(vk_ctx, init.image_create_info);

      std::vector<alvr::VkFrame> images;
      images.reserve(init.num_images);
      for (size_t i = 0; i < init.num_images; ++i) {
        images.emplace_back(vk_ctx, init.image_create_info, init.mem_index, fds[2*i+1], fds[2*i+2]);
      }

      auto encode_pipeline = alvr::EncodePipeline::Create(images, vk_frame_ctx);

      fprintf(stderr, "CEncoder starting to read present packets");
      std::vector<uint8_t> encoded_data;
      while (not m_exiting) {
        uint32_t image = present_shm::none_id;
        {
          std::unique_lock<std::mutex> lock(shm->mutex);
          while (not m_exiting)
          {
            image = shm->next;
            if (image != present_shm::none_id)
            {
              shm->owned_by_consumer = image;
              shm->next = present_shm::none_id;
              break;
            }
            shm->cv.wait_for(lock, std::chrono::milliseconds(10));
          }
        }
        if (m_exiting)
          break;
        assert(image != present_shm::none_id);
        assert(image < init.num_images);

        auto encode_start = std::chrono::steady_clock::now();
        encode_pipeline->PushFrame(image, m_scheduler.CheckIDRInsertion());

        static_assert(sizeof(shm->info[0].pose) == sizeof(vr::HmdMatrix34_t&));
        auto pose = m_poseHistory->GetBestPoseMatch((const vr::HmdMatrix34_t&)shm->info[image].pose);
        if (pose)
          m_poseSubmitIndex = pose->info.FrameIndex;

        encoded_data.clear();
        while (encode_pipeline->GetEncoded(encoded_data)) {}
        shm->owned_by_consumer = present_shm::none_id;
        m_listener->SendVideo(encoded_data.data(), encoded_data.size(), m_poseSubmitIndex + Settings::Instance().m_trackingFrameOffset);

        auto encode_end = std::chrono::steady_clock::now();

        m_listener->GetStatistics()->EncodeOutput(std::chrono::duration_cast<std::chrono::microseconds>(encode_end - encode_start).count());

      }
    }
    catch (std::exception &e) {
      std::stringstream err;
      err << "error in encoder thread: " << e.what();
      Error(err.str().c_str());
    }
}

void CEncoder::Stop() {
    m_exiting = true;
}

void CEncoder::OnPacketLoss() { m_scheduler.OnPacketLoss(); }

void CEncoder::InsertIDR() { m_scheduler.InsertIDR(); }
