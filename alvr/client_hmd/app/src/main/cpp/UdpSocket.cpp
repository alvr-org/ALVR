
#include <sys/socket.h>
#include <exception.h>

#include <endian.h>
#include <cstdlib>
#include <algorithm>
#include <cerrno>
#include <bits/fcntl.h>
#include <unistd.h>

#include "UdpSocket.h"
#include "utils.h"

UdpSocket::~UdpSocket()
{
    if (m_sock >= 0)
    {
        close(m_sock);
    }
}

void UdpSocket::initialize(JNIEnv *env, int helloPort, int port, jobjectArray broadcastAddrList_)
{
    int val;
    socklen_t len;

    m_sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (m_sock < 0)
    {
        throw FormatException("socket error : %d %s", errno, strerror(errno));
    }
    val = 1;
    int flags = fcntl(m_sock, F_GETFL, 0);
    fcntl(m_sock, F_SETFL, flags | O_NONBLOCK);

    val = 1;
    setsockopt(m_sock, SOL_SOCKET, SO_BROADCAST, (char *) &val, sizeof(val));

    // To avoid EADDRINUSE when previous process (or thread) remains live.
    val = 1;
    setsockopt(m_sock, SOL_SOCKET, SO_REUSEADDR, &val, sizeof(val));

    //
    // UdpSocket recv buffer
    //

    //setMaxSocketBuffer();
    // 30Mbps 50ms buffer
    getsockopt(m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, &len);
    LOGI("Default socket recv buffer is %d bytes", val);

    val = 30 * 1000 * 500 / 8;
    setsockopt(m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, sizeof(val));
    len = sizeof(val);
    getsockopt(m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, &len);
    LOGI("Current socket recv buffer is %d bytes", val);

    sockaddr_in addr;
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = INADDR_ANY;
    if (bind(m_sock, (sockaddr *) &addr, sizeof(addr)) < 0)
    {
        throw FormatException("bind error : %d %s", errno, strerror(errno));
    }

    //
    // Parse broadcast address list.
    //

    setBroadcastAddrList(env, helloPort, port, broadcastAddrList_);
}

void
UdpSocket::setBroadcastAddrList(JNIEnv *env, int helloPort, int port, jobjectArray broadcastAddrList_)
{
    int broadcastCount = env->GetArrayLength(broadcastAddrList_);

    for (int i = 0; i < broadcastCount; i++)
    {
        jstring address = (jstring) env->GetObjectArrayElement(broadcastAddrList_, i);
        auto addressStr = GetStringFromJNIString(env, address);
        env->DeleteLocalRef(address);

        sockaddr_in addr;
        memset(&addr, 0, sizeof(addr));
        addr.sin_family = AF_INET;
        addr.sin_port = htons(helloPort);
        inet_pton(addr.sin_family, addressStr.c_str(), &addr.sin_addr);

        m_broadcastAddrList.push_back(addr);

        memset(&addr, 0, sizeof(addr));
        addr.sin_family = AF_INET;
        addr.sin_port = htons(port);
        inet_pton(addr.sin_family, addressStr.c_str(), &addr.sin_addr);

        m_broadcastAddrList.push_back(addr);
    }
}

int UdpSocket::send(const void *buf, size_t len)
{
    LOGSOCKET("Sending %zu bytes", len);
    return (int) sendto(m_sock, buf, len, 0, (sockaddr *) &m_serverAddr, sizeof(m_serverAddr));
}

void UdpSocket::recv()
{
    char packet[MAX_PACKET_UDP_PACKET_SIZE];
    sockaddr_in addr;
    socklen_t socklen = sizeof(addr);

    while (true)
    {
        int packetSize = static_cast<int>(recvfrom(m_sock, packet, MAX_PACKET_UDP_PACKET_SIZE, 0,
                                                   (sockaddr *) &addr,
                                                   &socklen));
        if (packetSize <= 0)
        {
            LOGSOCKET("Error on recvfrom. ret=%d", packetSize);
            return;
        }
        LOGSOCKET("recvfrom Ok. calling parse(). ret=%d", packetSize);
        parse(packet, packetSize, addr);
        LOGSOCKET("parse() end. ret=%d", packetSize);
    }
}

void UdpSocket::disconnect()
{
    m_connected = false;
    memset(&m_serverAddr, 0, sizeof(m_serverAddr));
}

jstring UdpSocket::getServerAddress(JNIEnv *env)
{
    if (m_hasServerAddress)
    {
        char serverAddress[100];
        inet_ntop(m_serverAddr.sin_family, &m_serverAddr.sin_addr, serverAddress,
                  sizeof(serverAddress));
        return env->NewStringUTF(serverAddress);
    }
    return NULL;
}

int UdpSocket::getServerPort()
{
    if (m_hasServerAddress)
        return htons(m_serverAddr.sin_port);
    return 0;
}

int UdpSocket::getSocket()
{
    return m_sock;
}

void UdpSocket::sendBroadcast(const void *buf, size_t len)
{
    for (const sockaddr_in &address : m_broadcastAddrList)
    {
        sendto(m_sock, buf, len, 0, (sockaddr *) &address, sizeof(address));
    }
}

void UdpSocket::parse(char *packet, int packetSize, const sockaddr_in &addr)
{
    if (m_connected)
    {
        if (addr.sin_port != m_serverAddr.sin_port ||
            addr.sin_addr.s_addr != m_serverAddr.sin_addr.s_addr)
        {
            char str[1000];
            // Invalid source address. Ignore.
            inet_ntop(addr.sin_family, &addr.sin_addr, str, sizeof(str));
            LOGE("Received packet from invalid source address. Address=%s:%d", str,
                 htons(addr.sin_port));
            return;
        }
        m_onPacketRecv(packet, packetSize);
    } else
    {
        uint32_t type = *(uint32_t *) packet;
        if (type == ALVR_PACKET_TYPE_BROADCAST_REQUEST_MESSAGE)
        {
            m_onBroadcastRequest();
        } else if (type == ALVR_PACKET_TYPE_CONNECTION_MESSAGE)
        {
            if (packetSize < sizeof(ConnectionMessage))
                return;

            m_serverAddr = addr;
            m_connected = true;
            m_hasServerAddress = true;

            ConnectionMessage *connectionMessage = (ConnectionMessage *) packet;

            LOGI("Try setting recv buffer size = %d bytes", connectionMessage->bufferSize);
            int val = connectionMessage->bufferSize;
            setsockopt(m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, sizeof(val));
            socklen_t socklen = sizeof(val);
            getsockopt(m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, &socklen);
            LOGI("Current socket recv buffer is %d bytes", val);

            m_onConnect(*connectionMessage);

            return;
        }
    }
}

void UdpSocket::recoverConnection(std::string serverAddress, int serverPort)
{
    LOGI("Sending recover connection request. server=%s:%d", serverAddress.c_str(), serverPort);
    sockaddr_in addr;
    addr.sin_family = AF_INET;
    addr.sin_port = htons(serverPort);
    inet_pton(AF_INET, serverAddress.c_str(), &addr.sin_addr);

    RecoverConnection message = {};
    message.type = ALVR_PACKET_TYPE_RECOVER_CONNECTION;

    sendto(m_sock, &message, sizeof(message), 0, (sockaddr *) &addr, sizeof(addr));
}

void UdpSocket::setOnConnect(std::function<void(const ConnectionMessage &connectionMessage)> onConnect)
{
    m_onConnect = onConnect;
}

void UdpSocket::setOnBroadcastRequest(std::function<void()> onBroadcastRequest)
{
    m_onBroadcastRequest = onBroadcastRequest;
}

void UdpSocket::setOnPacketRecv(std::function<void(const char *buf, size_t len)> onPacketRecv)
{
    m_onPacketRecv = onPacketRecv;
}

bool UdpSocket::isConnected() const
{
    return m_connected;
}
