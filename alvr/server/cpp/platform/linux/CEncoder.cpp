#include "CEncoder.h"

#include <algorithm>
#include <chrono>
#include <exception>
#include <memory>
#include <openvr_driver.h>
#include <stdexcept>
#include <string>
#include <sys/socket.h>
#include <sys/un.h>

#include "ALVR-common/packet_types.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/Logger.h"
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Statistics.h"
#include "subprocess.hpp"

CEncoder::CEncoder(std::shared_ptr<ClientConnection> listener,
                   std::shared_ptr<PoseHistory> poseHistory)
    : m_listener(listener), m_poseHistory(poseHistory) {}

CEncoder::~CEncoder() { Stop(); }

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
    // we don't really care about what happends with unlink, it's just incase we crashed before this run
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
    int client = accept(m_socket, NULL, NULL);
    Info("CEncoder client connected");
	int received_fds[3];
	GetFds(client, &received_fds);

    printf("\n\nCEncoder: got fds: %d,%d,%d\n", received_fds[0], received_fds[1], received_fds[2]);

    close(client);
    this->Stop();
}

void CEncoder::Stop() {
    m_exiting = true;
    close(m_socket);
    unlink(m_socketPath.c_str());
}

void CEncoder::OnPacketLoss() { m_scheduler.OnPacketLoss(); }

void CEncoder::InsertIDR() { m_scheduler.InsertIDR(); }
