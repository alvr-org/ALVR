/// UdpReceiverThread jni functions using UDP socket
// Send tracking information and lost packet feedback to server.
// And receive screen video stream.
////////////////////////////////////////////////////////////////////

#include "bindings.h"

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
#include <cstdlib>
#include <pthread.h>
#include <endian.h>
#include <algorithm>
#include <cerrno>
#include <sys/ioctl.h>
#include "utils.h"
#include "latency_collector.h"
#include "exception.h"
#include <utility>
#include <bits/fcntl.h>

const uint64_t CONNECTION_TIMEOUT = 3 * 1000 * 1000;

struct SendBuffer {
    char buf[MAX_PACKET_UDP_PACKET_SIZE];
    int len;
};

class ServerConnectionNative {
public:
    int m_sock = -1;
    bool m_connected = false;

    bool m_hasServerAddress = false;
    sockaddr_in m_serverAddr = {};

    std::list<sockaddr_in> m_broadcastAddrList;
// Connection has lost when elapsed 3 seconds from last packet.

    bool m_stopped = false;

    // Turned true when decoder thread is prepared.
    bool mSinkPrepared = false;

    time_t m_prevSentSync = 0;
    time_t m_prevSentBroadcast = 0;
    int64_t m_timeDiff = 0;
    uint64_t timeSyncSequence = (uint64_t) -1;
    uint64_t m_lastFrameIndex = 0;
    ConnectionMessage m_connectionMessage = {};

    uint32_t m_prevVideoSequence = 0;
    uint32_t m_prevSoundSequence = 0;
    std::shared_ptr<SoundPlayer> m_soundPlayer;
    std::shared_ptr<NALParser> m_nalParser;

    JNIEnv *m_env;
    jobject m_instance;
    jmethodID mOnConnectMethodID;
    jmethodID mOnDisconnectedMethodID;
    jmethodID mOnHapticsFeedbackID;
    jmethodID mSetWebGuiUrlID;
    jmethodID mOnGuardianSyncAckID;
    jmethodID mOnGuardianSegmentAckID;

    int m_notifyPipe[2] = {-1, -1};
    std::mutex pipeMutex;
    std::list<SendBuffer> m_sendQueue;
};

namespace {
    ServerConnectionNative g_socket;
}

int send(const void *buf, size_t len) {
    LOGSOCKET("Sending %zu bytes", len);
    return (int) sendto(g_socket.m_sock, buf, len, 0, (sockaddr *) &g_socket.m_serverAddr,
                        sizeof(g_socket.m_serverAddr));
}

void connectSocket(void *v_env, ConnectionMessage connectionMessage) {
    auto *env = (JNIEnv *) v_env;

    // Save video width and height
    g_socket.m_connectionMessage = connectionMessage;

    inet_pton(AF_INET, connectionMessage.ip, &g_socket.m_serverAddr.sin_addr);
    g_socket.m_serverAddr.sin_port = htons(9944);
    g_socket.m_connected = true;
    g_socket.m_hasServerAddress = true;

    LOGI("Try setting recv buffer size = %d bytes", g_socket.m_connectionMessage.bufferSize);
    int val = g_socket.m_connectionMessage.bufferSize;
    setsockopt(g_socket.m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, sizeof(val));
    socklen_t socklen = sizeof(val);
    getsockopt(g_socket.m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, &socklen);
    LOGI("Current socket recv buffer is %d bytes", val);

    g_socket.m_prevVideoSequence = 0;
    g_socket.m_prevSoundSequence = 0;
    g_socket.m_timeDiff = 0;
    LatencyCollector::Instance().resetAll();
    g_socket.m_nalParser->setCodec(g_socket.m_connectionMessage.codec);

    env->CallVoidMethod(g_socket.m_instance, g_socket.mOnConnectMethodID,
                                   g_socket.m_connectionMessage.videoWidth,
                                   g_socket.m_connectionMessage.videoHeight,
                                   g_socket.m_connectionMessage.codec,
                                   g_socket.m_connectionMessage.realtimeDecoder,
                                   g_socket.m_connectionMessage.frameQueueSize,
                                   g_socket.m_connectionMessage.refreshRate,
                                   g_socket.m_connectionMessage.streamMic,
                                   g_socket.m_connectionMessage.foveationMode,
                                   g_socket.m_connectionMessage.foveationStrength,
                                   g_socket.m_connectionMessage.foveationShape,
                                   g_socket.m_connectionMessage.foveationVerticalOffset,
                                   g_socket.m_connectionMessage.trackingSpace);

    jstring jstr = env->NewStringUTF(g_socket.m_connectionMessage.webGuiUrl);
    env->CallVoidMethod(g_socket.m_instance, g_socket.mSetWebGuiUrlID, jstr);
}

void sendPacketLossReport(ALVR_LOST_FRAME_TYPE frameType,
                          uint32_t fromPacketCounter,
                          uint32_t toPacketCounter) {
    PacketErrorReport report{};
    report.type = ALVR_PACKET_TYPE_PACKET_ERROR_REPORT;
    report.lostFrameType = frameType;
    report.fromPacketCounter = fromPacketCounter;
    report.toPacketCounter = toPacketCounter;
    int ret = send(&report, sizeof(report));
    LOGI("Sent packet loss report. ret=%d", ret);
}

void processSoundSequence(uint32_t sequence) {
    if (g_socket.m_prevSoundSequence != 0 && g_socket.m_prevSoundSequence + 1 != sequence) {
        int32_t lost = sequence - (g_socket.m_prevSoundSequence + 1);
        if (lost < 0) {
            // lost become negative on out-of-order packet.
            // TODO: This is not accurate statistics.
            lost = -lost;
        }
        LatencyCollector::Instance().packetLoss(lost);

        sendPacketLossReport(ALVR_LOST_FRAME_TYPE_AUDIO, g_socket.m_prevSoundSequence + 1,
                             sequence - 1);

        LOGE("SoundPacket loss %d (%d -> %d)", lost, g_socket.m_prevSoundSequence + 1,
             sequence - 1);
    }
    g_socket.m_prevSoundSequence = sequence;
}

void processVideoSequence(uint32_t sequence) {
    if (g_socket.m_prevVideoSequence != 0 && g_socket.m_prevVideoSequence + 1 != sequence) {
        int32_t lost = sequence - (g_socket.m_prevVideoSequence + 1);
        if (lost < 0) {
            // lost become negative on out-of-order packet.
            // TODO: This is not accurate statistics.
            lost = -lost;
        }
        LatencyCollector::Instance().packetLoss(lost);

        LOGE("VideoPacket loss %d (%d -> %d)", lost, g_socket.m_prevVideoSequence + 1,
             sequence - 1);
    }
    g_socket.m_prevVideoSequence = sequence;
}

void onPacketRecv(const char *packet, size_t packetSize) {
    uint32_t type = *(uint32_t *) packet;
    if (type == ALVR_PACKET_TYPE_VIDEO_FRAME) {
        auto *header = (VideoFrame *) packet;

        if (g_socket.m_lastFrameIndex != header->trackingFrameIndex) {
            LatencyCollector::Instance().receivedFirst(header->trackingFrameIndex);
            if ((int64_t) header->sentTime - g_socket.m_timeDiff > getTimestampUs()) {
                LatencyCollector::Instance().estimatedSent(header->trackingFrameIndex, 0);
            } else {
                LatencyCollector::Instance().estimatedSent(header->trackingFrameIndex,
                                                           (int64_t) header->sentTime -
                                                           g_socket.m_timeDiff - getTimestampUs());
            }
            g_socket.m_lastFrameIndex = header->trackingFrameIndex;
        }

        processVideoSequence(header->packetCounter);

        // Following packets of a video frame
        bool fecFailure = false;
        bool ret2 = g_socket.m_nalParser->processPacket(header, packetSize, fecFailure);
        if (ret2) {
            LatencyCollector::Instance().receivedLast(header->trackingFrameIndex);
        }
        if (fecFailure) {
            LatencyCollector::Instance().fecFailure();
            sendPacketLossReport(ALVR_LOST_FRAME_TYPE_VIDEO, 0, 0);
        }
    } else if (type == ALVR_PACKET_TYPE_TIME_SYNC) {
        // Time sync packet
        if (packetSize < sizeof(TimeSync)) {
            return;
        }
        auto *timeSync = (TimeSync *) packet;
        uint64_t Current = getTimestampUs();
        if (timeSync->mode == 1) {
            uint64_t RTT = Current - timeSync->clientTime;
            g_socket.m_timeDiff =
                    ((int64_t) timeSync->serverTime + (int64_t) RTT / 2) - (int64_t) Current;
            LOGI("TimeSync: server - client = %ld us RTT = %lu us", g_socket.m_timeDiff, RTT);

            TimeSync sendBuf = *timeSync;
            sendBuf.mode = 2;
            sendBuf.clientTime = Current;
            send(&sendBuf, sizeof(sendBuf));
        }
    } else if (type == ALVR_PACKET_TYPE_AUDIO_FRAME_START) {
        // Change settings
        if (packetSize < sizeof(AudioFrameStart)) {
            return;
        }
        auto header = (AudioFrameStart *) packet;

        processSoundSequence(header->packetCounter);

        if (g_socket.m_soundPlayer) {
            g_socket.m_soundPlayer->putData((uint8_t *) packet + sizeof(*header),
                                            packetSize - sizeof(*header));
        }

        //LOG("Received audio frame start: Counter=%d Size=%d PresentationTime=%lu",
        //    header->packetCounter, header->frameByteSize, header->presentationTime);
    } else if (type == ALVR_PACKET_TYPE_AUDIO_FRAME) {
        // Change settings
        if (packetSize < sizeof(AudioFrame)) {
            return;
        }
        auto header = (AudioFrame *) packet;

        processSoundSequence(header->packetCounter);

        if (g_socket.m_soundPlayer) {
            g_socket.m_soundPlayer->putData((uint8_t *) packet + sizeof(*header),
                                            packetSize - sizeof(*header));
        }

        //LOG("Received audio frame: Counter=%d", header->packetCounter);
    } else if (type == ALVR_PACKET_TYPE_HAPTICS) {
        if (packetSize < sizeof(HapticsFeedback)) {
            return;
        }
        auto header = (HapticsFeedback *) packet;

        g_socket.m_env->CallVoidMethod(g_socket.m_instance, g_socket.mOnHapticsFeedbackID,
                                       static_cast<jlong>(header->startTime),
                                       header->amplitude, header->duration, header->frequency,
                                       static_cast<jboolean>(header->hand));

    } else if (type == ALVR_PACKET_TYPE_GUARDIAN_SYNC_ACK) {
        if (packetSize < sizeof(GuardianSyncStartAck)) {
            return;
        }

        auto ack = (GuardianSyncStartAck *) packet;

        g_socket.m_env->CallVoidMethod(g_socket.m_instance, g_socket.mOnGuardianSyncAckID,
                                       static_cast<jlong>(ack->timestamp));
    } else if (type == ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_ACK) {
        if (packetSize < sizeof(GuardianSegmentAck)) {
            return;
        }

        auto ack = (GuardianSegmentAck *) packet;

        g_socket.m_env->CallVoidMethod(g_socket.m_instance, g_socket.mOnGuardianSegmentAckID,
                                       static_cast<jlong>(ack->timestamp),
                                       static_cast<jint>(ack->segmentIndex));
    }
}

void parse(char *packet, int packetSize, const sockaddr_in &addr) {
    if (g_socket.m_connected) {
        if (addr.sin_port != g_socket.m_serverAddr.sin_port ||
            addr.sin_addr.s_addr != g_socket.m_serverAddr.sin_addr.s_addr) {
            char str[1000];
            // Invalid source address. Ignore.
            inet_ntop(addr.sin_family, &addr.sin_addr, str, sizeof(str));
            LOGE("Received packet from invalid source address. Address=%s:%d", str,
                 htons(addr.sin_port));
            return;
        }
        onPacketRecv(packet, packetSize);
    }
}

void recv() {
    char packet[MAX_PACKET_UDP_PACKET_SIZE];
    sockaddr_in addr{};
    socklen_t socklen = sizeof(addr);

    while (true) {
        int packetSize = static_cast<int>(recvfrom(g_socket.m_sock, packet,
                                                   MAX_PACKET_UDP_PACKET_SIZE, 0,
                                                   (sockaddr *) &addr,
                                                   &socklen));
        if (packetSize <= 0) {
            LOGSOCKET("Error on recvfrom. ret=%d", packetSize);
            return;
        }
        LOGSOCKET("recvfrom Ok. calling parse(). ret=%d", packetSize);
        parse(packet, packetSize, addr);
        LOGSOCKET("parse() end. ret=%d", packetSize);
    }
}

void closeSocket() {
    if (g_socket.m_notifyPipe[0] >= 0) {
        close(g_socket.m_notifyPipe[0]);
        close(g_socket.m_notifyPipe[1]);
    }

    g_socket.m_nalParser.reset();
    g_socket.m_sendQueue.clear();
}

void initializeSocket(void *v_env, void *v_instance) {
    auto *env = (JNIEnv *) v_env;
    auto *instance = (jobject) v_instance;

    //
    // Initialize variables
    //

    g_socket.m_stopped = false;
    g_socket.m_prevSentSync = 0;
    g_socket.m_prevSentBroadcast = 0;
    g_socket.m_prevVideoSequence = 0;
    g_socket.m_prevSoundSequence = 0;
    g_socket.m_timeDiff = 0;

    jclass clazz = env->GetObjectClass(instance);
    g_socket.mOnConnectMethodID = env->GetMethodID(clazz, "onConnected", "(IIIZIIZIFFFI)V");
    g_socket.mOnDisconnectedMethodID = env->GetMethodID(clazz, "onDisconnected", "()V");
    g_socket.mOnHapticsFeedbackID = env->GetMethodID(clazz, "onHapticsFeedback", "(JFFFZ)V");
    g_socket.mSetWebGuiUrlID = env->GetMethodID(clazz, "setWebViewURL", "(Ljava/lang/String;)V");
    g_socket.mOnGuardianSyncAckID = env->GetMethodID(clazz, "onGuardianSyncAck", "(J)V");
    g_socket.mOnGuardianSegmentAckID = env->GetMethodID(clazz, "onGuardianSegmentAck", "(JI)V");
    env->DeleteLocalRef(clazz);

    g_socket.m_nalParser = std::make_shared<NALParser>(env, instance);


    //
    // UdpSocket
    //
    {
        int val;
        socklen_t len;

        g_socket.m_sock = socket(AF_INET, SOCK_DGRAM, 0);
        if (g_socket.m_sock < 0) {
            throw FormatException("socket error : %d %s", errno, strerror(errno));
        }
        val = 1;
        int flags = fcntl(g_socket.m_sock, F_GETFL, 0);
        fcntl(g_socket.m_sock, F_SETFL, flags | O_NONBLOCK);

//        val = 1;
//        setsockopt(g_socket.m_sock, SOL_SOCKET, SO_BROADCAST, (char *) &val, sizeof(val));

        // To avoid EADDRINUSE when previous process (or thread) remains live.
        val = 1;
        setsockopt(g_socket.m_sock, SOL_SOCKET, SO_REUSEADDR, &val, sizeof(val));

        //
        // UdpSocket recv buffer
        //

        //setMaxSocketBuffer();
        // 30Mbps 50ms buffer
        getsockopt(g_socket.m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, &len);
        LOGI("Default socket recv buffer is %d bytes", val);

        val = 30 * 1000 * 500 / 8;
        setsockopt(g_socket.m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, sizeof(val));
        len = sizeof(val);
        getsockopt(g_socket.m_sock, SOL_SOCKET, SO_RCVBUF, (char *) &val, &len);
        LOGI("Current socket recv buffer is %d bytes", val);

        sockaddr_in addr{};
        addr.sin_family = AF_INET;
        addr.sin_port = htons(9944);
        addr.sin_addr.s_addr = INADDR_ANY;
        if (bind(g_socket.m_sock, (sockaddr *) &addr, sizeof(addr)) < 0) {
            throw FormatException("bind error : %d %s", errno, strerror(errno));
        }
    }


    //
    // Sound
    //

    g_socket.m_soundPlayer = std::make_shared<SoundPlayer>();
    if (g_socket.m_soundPlayer->initialize() != 0) {
        LOGE("Failed on SoundPlayer initialize.");
        g_socket.m_soundPlayer.reset();
    }
    LOGI("SoundPlayer successfully initialize.");

    //
    // Pipe used for send buffer notification.
    //

    if (pipe2(g_socket.m_notifyPipe, O_NONBLOCK) < 0) {
        throw FormatException("pipe2 error : %d %s", errno, strerror(errno));
    }

    LOGI("ServerConnectionNative initialized.");
}

void processReadPipe(int pipefd) {
    char buf[2000];
    int len = 1;

    int ret = static_cast<int>(read(pipefd, buf, len));
    if (ret <= 0) {
        return;
    }

    SendBuffer sendBuffer{};
    while (true) {
        {
            std::lock_guard<std::mutex> lock(g_socket.pipeMutex);

            if (g_socket.m_sendQueue.empty()) {
                break;
            } else {
                sendBuffer = g_socket.m_sendQueue.front();
                g_socket.m_sendQueue.pop_front();
            }
        }
        if (g_socket.m_stopped) {
            return;
        }

        //LOG("Sending tracking packet %d", sendBuffer.len);
        send(sendBuffer.buf, sendBuffer.len);
    }
}

void sendTimeSyncLocked() {
    time_t current = time(nullptr);
    if (g_socket.m_prevSentSync != current && g_socket.m_connected) {
        LOGI("Sending timesync.");

        TimeSync timeSync = {};
        timeSync.type = ALVR_PACKET_TYPE_TIME_SYNC;
        timeSync.mode = 0;
        timeSync.clientTime = getTimestampUs();
        timeSync.sequence = ++g_socket.timeSyncSequence;

        timeSync.packetsLostTotal = LatencyCollector::Instance().getPacketsLostTotal();
        timeSync.packetsLostInSecond = LatencyCollector::Instance().getPacketsLostInSecond();

        timeSync.averageTotalLatency = (uint32_t) LatencyCollector::Instance().getLatency(0, 0);
        timeSync.maxTotalLatency = (uint32_t) LatencyCollector::Instance().getLatency(0, 1);
        timeSync.minTotalLatency = (uint32_t) LatencyCollector::Instance().getLatency(0, 2);

        timeSync.averageTransportLatency = (uint32_t) LatencyCollector::Instance().getLatency(1, 0);
        timeSync.maxTransportLatency = (uint32_t) LatencyCollector::Instance().getLatency(1, 1);
        timeSync.minTransportLatency = (uint32_t) LatencyCollector::Instance().getLatency(1, 2);

        timeSync.averageDecodeLatency = (uint32_t) LatencyCollector::Instance().getLatency(2, 0);
        timeSync.maxDecodeLatency = (uint32_t) LatencyCollector::Instance().getLatency(2, 1);
        timeSync.minDecodeLatency = (uint32_t) LatencyCollector::Instance().getLatency(2, 2);

        timeSync.fecFailure = g_socket.m_nalParser->fecFailure() ? 1 : 0;
        timeSync.fecFailureTotal = LatencyCollector::Instance().getFecFailureTotal();
        timeSync.fecFailureInSecond = LatencyCollector::Instance().getFecFailureInSecond();

        timeSync.fps = LatencyCollector::Instance().getFramesInSecond();

        send(&timeSync, sizeof(timeSync));
    }
    g_socket.m_prevSentSync = current;
}

void doPeriodicWork() {
    sendTimeSyncLocked();
}

void sendNative(long long nativeBuffer, int length) {
    auto *packet = reinterpret_cast<char *>(nativeBuffer);

    if (g_socket.m_stopped) {
        return;
    }
    SendBuffer sendBuffer{};

    memcpy(sendBuffer.buf, packet, length);
    sendBuffer.len = length;

    {
        std::lock_guard<decltype(g_socket.pipeMutex)> lock(g_socket.pipeMutex);
        g_socket.m_sendQueue.push_back(sendBuffer);
    }
    // Notify enqueue to loop thread
    write(g_socket.m_notifyPipe[1], "", 1);
}

void runLoop(void *v_env, void *v_instance) {
    auto *env = (JNIEnv *) v_env;
    auto *instance = (jobject) v_instance;

    fd_set fds, fds_org;

    FD_ZERO(&fds_org);
    FD_SET(g_socket.m_sock, &fds_org);
    FD_SET(g_socket.m_notifyPipe[0], &fds_org);
    int nfds = std::max(g_socket.m_sock, g_socket.m_notifyPipe[0]) + 1;

    g_socket.m_env = env;
    g_socket.m_instance = env->NewGlobalRef(instance);

    while (!g_socket.m_stopped) {
        timeval timeout{};
        timeout.tv_sec = 0;
        timeout.tv_usec = 10 * 1000;
        memcpy(&fds, &fds_org, sizeof(fds));
        int ret = select(nfds, &fds, nullptr, nullptr, &timeout);

        if (ret == 0) {
            doPeriodicWork();

            // timeout
            continue;
        }

        if (FD_ISSET(g_socket.m_notifyPipe[0], &fds)) {
            //LOG("select pipe");
            processReadPipe(g_socket.m_notifyPipe[0]);
        }

        if (FD_ISSET(g_socket.m_sock, &fds)) {
            recv();
        }
        doPeriodicWork();
    }

    LOGI("Exited select loop.");

    g_socket.m_soundPlayer.reset();

//    env->DeleteGlobalRef(g_socket.m_instance);

    LOGI("Exiting UdpReceiverThread runLoop");
}

void interruptNative() {
    g_socket.m_stopped = true;

    // Notify stop to loop thread.
    write(g_socket.m_notifyPipe[1], "", 1);
}

unsigned char isConnectedNative() {
    return g_socket.m_connected;
}


void setSinkPreparedNative(unsigned char prepared) {
    if (g_socket.m_stopped) {
        return;
    }
    g_socket.mSinkPrepared = prepared;
    LOGSOCKETI("setSinkPrepared: Decoder prepared=%d", g_socket.mSinkPrepared);
}

void disconnectSocket(void *v_env) {
    auto *env = (JNIEnv *) v_env;

    g_socket.m_connected = false;
    memset(&g_socket.m_serverAddr, 0, sizeof(g_socket.m_serverAddr));

    env->CallVoidMethod(g_socket.m_instance, g_socket.mOnDisconnectedMethodID);

    if (g_socket.m_soundPlayer) {
        g_socket.m_soundPlayer->Stop();
    }
}