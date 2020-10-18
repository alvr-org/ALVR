#ifndef ALVRCLIENT_UDP_H
#define ALVRCLIENT_UDP_H

#include <functional>
#include <list>
#include <string>
#include <memory>
#include <jni.h>
#include <sys/socket.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <new>
#include <stack>
#include <mutex>

#include "packet_types.h"
#include "nal.h"
#include "sound.h"
#include "UdpSocket.h"

class ServerConnectionNative {
public:
    ServerConnectionNative();
    ~ServerConnectionNative();
    void initialize(JNIEnv *env, jobject instance, jint helloPort, jint port, jstring deviceName_,
                        jobjectArray broadcastAddrList_, jintArray refreshRates_, jint renderWidth,
                        jint renderHeight, jfloatArray fov, jint deviceType, jint deviceSubType,
                        jint deviceCapabilityFlags, jint controllerCapabilityFlags, jfloat ipd);

    void send(const void *packet, int length);

    void runLoop(JNIEnv *env, jobject instance, jstring serverAddress, int serverPort);
    void interrupt();
    void setSinkPrepared(bool prepared);

    bool isConnected();

    jstring getServerAddress(JNIEnv *env);
    int getServerPort();

private:

    void initializeJNICallbacks(JNIEnv *env, jobject instance);

    void sendStreamStartPacket();

    void sendPacketLossReport(ALVR_LOST_FRAME_TYPE frameType, uint32_t fromPacketCounter,
                              uint32_t toPacketCounter);
    void processVideoSequence(uint32_t sequence);
    void processSoundSequence(uint32_t sequence);

    void processReadPipe(int pipefd);

    void sendTimeSyncLocked();
    void sendBroadcastLocked();
    void doPeriodicWork();

    void recoverConnection(std::string serverAddress, int serverPort);

    void checkConnection();
    void updateTimeout();

    void onConnect(const ConnectionMessage &connectionMessage);
    void onBroadcastRequest();
    void onPacketRecv(const char *packet, size_t packetSize);

    void loadFov(JNIEnv *env, jfloatArray fov_);

private:
// Connection has lost when elapsed 3 seconds from last packet.
    static const uint64_t CONNECTION_TIMEOUT = 3 * 1000 * 1000;

    bool m_stopped = false;

    // Turned true when decoder thread is prepared.
    bool mSinkPrepared = false;

    UdpSocket m_socket;
    time_t m_prevSentSync = 0;
    time_t m_prevSentBroadcast = 0;
    int64_t m_timeDiff = 0;
    uint64_t timeSyncSequence = (uint64_t) -1;
    uint64_t m_lastReceived = 0;
    uint64_t m_lastFrameIndex = 0;
    ConnectionMessage m_connectionMessage = {};

    uint32_t m_prevVideoSequence = 0;
    uint32_t m_prevSoundSequence = 0;
    std::shared_ptr<SoundPlayer> m_soundPlayer;
    std::shared_ptr<NALParser> m_nalParser;

    HelloMessage mHelloMessage;

    JNIEnv *m_env;
    jobject m_instance;
    jmethodID mOnConnectMethodID;
    jmethodID mOnChangeSettingsMethodID;
    jmethodID mOnDisconnectedMethodID;
    jmethodID mOnHapticsFeedbackID;
    jmethodID mSetWebGuiUrlID;
    jmethodID mOnGuardianSyncAckID;
    jmethodID mOnGuardianSegmentAckID;

    //
    // Send buffer
    //
    struct SendBuffer {
        char buf[MAX_PACKET_UDP_PACKET_SIZE];
        int len;
    };

    int m_notifyPipe[2] = {-1, -1};
    std::mutex pipeMutex;
    std::list<SendBuffer> m_sendQueue;

};

#endif //ALVRCLIENT_UDP_H
