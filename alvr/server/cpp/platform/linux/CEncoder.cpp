#include "CEncoder.h"

#include <algorithm>
#include <array>
#include <chrono>
#include <exception>
#include <memory>
#include <openvr_driver.h>
#include <stdexcept>
#include <string>
#include <sys/socket.h>
#include <sys/un.h>
#include <vulkan/vulkan.hpp>

#include "ALVR-common/packet_types.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/Logger.h"
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Statistics.h"
#include "protocol.h"
#include "subprocess.hpp"

extern "C" {
#include <libavutil/hwcontext.h>
#include <libavutil/hwcontext_vulkan.h>
}

CEncoder::CEncoder(std::shared_ptr<ClientConnection> listener,
                   std::shared_ptr<PoseHistory> poseHistory)
    : m_listener(listener), m_poseHistory(poseHistory) {}

CEncoder::~CEncoder() { Stop(); }

namespace
{
void read_exactly(int fd, char* out, size_t size, std::atomic_bool& exiting)
{
  while (not exiting and size != 0)
  {
    timeval timeout{
      .tv_sec = 0,
      .tv_usec = 15000
    };
    fd_set read_fd, write_fd, except_fd;
    FD_ZERO(&read_fd);
    FD_SET(fd, &read_fd);
    FD_ZERO(&write_fd);
    FD_ZERO(&except_fd);
    int count = select(fd + 1, &read_fd, &write_fd, &except_fd, &timeout);
    if (count < 0) {
      throw MakeException("select failed: %s", strerror(errno));
    } else if (count == 1)
    {
      int s = read(fd, out, size);
      if (s == -1)
      {
        throw MakeException("read failed: %s", strerror(errno));
      }
      out+= s;
      size -= s;
    }
  }
}

}

void CEncoder::GetFds(int client, int (*received_fds)[3]) {
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
        perror("recvmsg");
        exit(1);
    }

    for (cmsg = CMSG_FIRSTHDR(&msg); cmsg != NULL; cmsg = CMSG_NXTHDR(&msg, cmsg)) {
        if (cmsg->cmsg_level == SOL_SOCKET && cmsg->cmsg_type == SCM_RIGHTS) {
            memcpy(received_fds, CMSG_DATA(cmsg), sizeof(*received_fds));
            break;
        }
    }

    if (cmsg == NULL) {
        fprintf(stderr, "CEncoder: cmsg is NULL\n");
        exit(1);
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

    ret = listen(m_socket, 0);
    if (ret == -1) {
        perror("listen");
        exit(1);
    }

    Info("CEncoder Listening\n");
    int client = accept(m_socket, NULL, NULL);
    Info("CEncoder client connected\n");
    init_packet init;
    read_exactly(client, (char*)&init, sizeof(init), m_exiting);
    GetFds(client, &m_fds);

    Debug("CEncoder: got fds: %d,%d,%d\n", m_fds[0], m_fds[1], m_fds[2]);
    init_packet init_packet;
    ret = read(client, &init_packet, sizeof(init_packet));
    if (ret == -1) {
        perror("read");
        exit(1);
    }
    std::array<uint8_t, 8> targetPpcUuid;
    std::copy(std::begin(init_packet.devicePpcUuid), std::end(init_packet.devicePpcUuid),
              std::begin(targetPpcUuid));
    //
    // We have everything we need, it is time to initalize Vulkan.
    //
    // putenv("VK_APIDUMP_LOG_FILENAME=\"/home/ron/alvr_vrdump.txt\"");
    fprintf(stderr, "\n\nWe are initalizing Vulkan in CEncoder thread\n\n\n");

    AVBufferRef *vulkan_ctx;
    av_hwdevice_ctx_create(&vulkan_ctx, AV_HWDEVICE_TYPE_VULKAN, init.device_name.data(), NULL, 0);

    AVHWDeviceContext *hwctx = (AVHWDeviceContext*)vulkan_ctx->data;
    AVVulkanDeviceContext *vkctx = (AVVulkanDeviceContext*)hwctx->internal;
    vk::Device device = vkctx->act_dev;

    AVVkFrame* images[3];

    init.image_create_info.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED; // VUID-VkImageCreateInfo-pNext-01443
    for (size_t i = 0; i < 3 ; ++i)
    {
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
      importMemInfo.fd = m_fds[i];

      vk::MemoryAllocateInfo memAllocInfo;
      memAllocInfo.pNext = &importMemInfo;
      memAllocInfo.allocationSize = req.size;
      memAllocInfo.memoryTypeIndex = init.mem_index;

      vk::DeviceMemory mem = device.allocateMemory(memAllocInfo);
      device.bindImageMemory(image, mem, 0);

      vk::SemaphoreCreateInfo semInfo;
      vk::Semaphore semaphore = device.createSemaphore(semInfo);

      images[i] = av_vk_frame_alloc();
      images[i]->img[0] = image;
      images[i]->tiling = init.image_create_info.tiling;
      images[i]->mem[0] = mem;
      images[i]->size[0] = req.size;
      //FIXME: images[i]->flags
      //FIXME: images[i]->access
      images[i]->layout[0] = VK_IMAGE_LAYOUT_UNDEFINED;
      images[i]->sem[0] = semaphore;
    }

    present_packet packet;
    while (not m_exiting) {
        read_exactly(client, (char*)&packet, sizeof(packet), m_exiting);
    }

    close(client);
}

void CEncoder::Stop() {
    m_exiting = true;
    close(m_socket);
    unlink(m_socketPath.c_str());
    m_vkDevice.destroy();
    m_vkInstance.destroy();
}

void CEncoder::OnPacketLoss() { m_scheduler.OnPacketLoss(); }

void CEncoder::InsertIDR() { m_scheduler.InsertIDR(); }
