#ifndef ALVRCLIENT_UDPSOCKET_H
#define ALVRCLIENT_UDPSOCKET_H

#include <sys/socket.h>
#include <arpa/inet.h>
#include <jni.h>

#include <list>
#include <string>


#include "packet_types.h"

class UdpSocket
{
public:
    UdpSocket() = default;
    ~UdpSocket();

    void initialize(JNIEnv *env, int helloPort, int port, jobjectArray broadcastAddrList_);
    void sendBroadcast(const void *buf, size_t len);
    int send(const void *buf, size_t len);
    void recv();
    void disconnect();

    //
    // Callback
    //

    void setOnConnect(std::function<void(const ConnectionMessage &connectionMessage)> onConnect);
    void setOnBroadcastRequest(std::function<void()> onBroadcastRequest);
    void setOnPacketRecv(std::function<void(const char *buf, size_t len)> onPacketRecv);

    //
    // Getter
    //

    bool isConnected() const;

    jstring getServerAddress(JNIEnv *env);

    int getServerPort() const;

    int getSocket() const;

private:
    void parse(char *packet, int packetSize, const sockaddr_in &addr);
    void setBroadcastAddrList(JNIEnv *env, int helloPort, int port, jobjectArray broadcastAddrList_);

private:
    int m_sock = -1;
    bool m_connected = false;

    bool m_hasServerAddress = false;
    sockaddr_in m_serverAddr = {};

    std::list<sockaddr_in> m_broadcastAddrList;

    std::function<void(const ConnectionMessage &connectionMessage)> m_onConnect;
    std::function<void()> m_onBroadcastRequest;
    std::function<void(const char *buf, size_t len)> m_onPacketRecv;
};

#endif //ALVRCLIENT_UDPSOCKET_H
