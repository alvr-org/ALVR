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
#include "UdpSocket.h"
#include <stdlib.h>
#include <pthread.h>
#include <endian.h>
#include <algorithm>
#include <errno.h>
#include <sys/ioctl.h>
#include "utils.h"
#include "latency_collector.h"
#include "exception.h"

const uint64_t CONNECTION_TIMEOUT = 3 * 1000 * 1000;

struct SendBuffer {
    char buf[MAX_PACKET_UDP_PACKET_SIZE];
    int len;
};

class ServerConnectionNative {
public:
// Connection has lost when elapsed 3 seconds from last packet.

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

    int m_notifyPipe[2] = {-1, -1};
    std::mutex pipeMutex;
    std::list<SendBuffer> m_sendQueue;
};

namespace {
    ServerConnectionNative g_socket;
}


void closeSocket() {
    if (g_socket.m_notifyPipe[0] >= 0) {
        close(g_socket.m_notifyPipe[0]);
        close(g_socket.m_notifyPipe[1]);
    }

    g_socket.m_nalParser.reset();
    g_socket.m_sendQueue.clear();
}

void initializeJNICallbacks(JNIEnv *env, jobject instance) {
    jclass clazz = env->GetObjectClass(instance);

    g_socket.mOnConnectMethodID = env->GetMethodID(clazz, "onConnected", "(IIIIIZIFFF)V");
    g_socket.mOnChangeSettingsMethodID = env->GetMethodID(clazz, "onChangeSettings", "(JII)V");
    g_socket.mOnDisconnectedMethodID = env->GetMethodID(clazz, "onDisconnected", "()V");
    g_socket.mOnHapticsFeedbackID = env->GetMethodID(clazz, "onHapticsFeedback", "(JFFFZ)V");
    g_socket.mSetWebGuiUrlID = env->GetMethodID(clazz, "setWebViewURL", "(Ljava/lang/String;)V");
    g_socket.mOnGuardianSyncAckID = env->GetMethodID(clazz, "onGuardianSyncAck", "(J)V");
    g_socket.mOnGuardianSegmentAckID = env->GetMethodID(clazz, "onGuardianSegmentAck", "(JI)V");

    env->DeleteLocalRef(clazz);
}

void updateTimeout() {
    g_socket.m_lastReceived = getTimestampUs();
}

void sendStreamStartPacket() {
    LOGSOCKETI("Sending stream start packet.");
    // Start stream.
    StreamControlMessage message = {};
    message.type = ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE;
    message.mode = 1;
    g_socket.m_socket.send(&message, sizeof(message));
}

void onConnect(const ConnectionMessage &connectionMessage) {
    // Save video width and height
    g_socket.m_connectionMessage = connectionMessage;

    updateTimeout();
    g_socket.m_prevVideoSequence = 0;
    g_socket.m_prevSoundSequence = 0;
    g_socket.m_timeDiff = 0;
    LatencyCollector::Instance().resetAll();
    g_socket.m_nalParser->setCodec(g_socket.m_connectionMessage.codec);

    g_socket.m_env->CallVoidMethod(g_socket.m_instance, g_socket.mOnConnectMethodID,
                                   g_socket.m_connectionMessage.videoWidth,
                                   g_socket.m_connectionMessage.videoHeight,
                                   g_socket.m_connectionMessage.codec,
                                   g_socket.m_connectionMessage.frameQueueSize,
                                   g_socket.m_connectionMessage.refreshRate,
                                   g_socket.m_connectionMessage.streamMic,
                                   g_socket.m_connectionMessage.foveationMode,
                                   g_socket.m_connectionMessage.foveationStrength,
                                   g_socket.m_connectionMessage.foveationShape,
                                   g_socket.m_connectionMessage.foveationVerticalOffset);

    jstring jstr = g_socket.m_env->NewStringUTF(g_socket.m_connectionMessage.webGuiUrl);
    g_socket.m_env->CallVoidMethod(g_socket.m_instance, g_socket.mSetWebGuiUrlID, jstr);

    if (g_socket.mSinkPrepared) {
        LOGSOCKETI("onConnect: Send stream start packet.");
        sendStreamStartPacket();
    }
}

void onBroadcastRequest() {
    // Respond with hello message.
    g_socket.m_socket.send(&g_socket.mHelloMessage, sizeof(g_socket.mHelloMessage));
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

void sendPacketLossReport(ALVR_LOST_FRAME_TYPE frameType,
                          uint32_t fromPacketCounter,
                          uint32_t toPacketCounter) {
    PacketErrorReport report;
    report.type = ALVR_PACKET_TYPE_PACKET_ERROR_REPORT;
    report.lostFrameType = frameType;
    report.fromPacketCounter = fromPacketCounter;
    report.toPacketCounter = toPacketCounter;
    int ret = g_socket.m_socket.send(&report, sizeof(report));
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

        sendPacketLossReport(ALVR_LOST_FRAME_TYPE_AUDIO, g_socket.m_prevSoundSequence + 1, sequence - 1);

        LOGE("SoundPacket loss %d (%d -> %d)", lost, g_socket.m_prevSoundSequence + 1,
             sequence - 1);
    }
    g_socket.m_prevSoundSequence = sequence;
}

void onPacketRecv(const char *packet, size_t packetSize) {
    updateTimeout();

    uint32_t type = *(uint32_t *) packet;
    if (type == ALVR_PACKET_TYPE_VIDEO_FRAME) {
        VideoFrame *header = (VideoFrame *) packet;

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
        TimeSync *timeSync = (TimeSync *) packet;
        uint64_t Current = getTimestampUs();
        if (timeSync->mode == 1) {
            uint64_t RTT = Current - timeSync->clientTime;
            g_socket.m_timeDiff =
                    ((int64_t) timeSync->serverTime + (int64_t) RTT / 2) - (int64_t) Current;
            LOGI("TimeSync: server - client = %ld us RTT = %lu us", g_socket.m_timeDiff, RTT);

            TimeSync sendBuf = *timeSync;
            sendBuf.mode = 2;
            sendBuf.clientTime = Current;
            g_socket.m_socket.send(&sendBuf, sizeof(sendBuf));
        }
    } else if (type == ALVR_PACKET_TYPE_CHANGE_SETTINGS) {
        // Change settings
        if (packetSize < sizeof(ChangeSettings)) {
            return;
        }
        ChangeSettings *settings = (ChangeSettings *) packet;

        g_socket.m_env->CallVoidMethod(g_socket.m_instance, g_socket.mOnChangeSettingsMethodID, settings->debugFlags,
                              settings->suspend, settings->frameQueueSize);
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

        g_socket.m_env->CallVoidMethod(g_socket.m_instance, g_socket.mOnGuardianSyncAckID, static_cast<jlong>(ack->timestamp));
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

void initializeSocket(void *v_env, void *v_instance,
                int helloPort, int port, void *v_deviceName, void *v_broadcastAddrList,
                void *v_refreshRates, int renderWidth, int renderHeight, void *v_fov,
                int deviceType, int deviceSubType, int deviceCapabilityFlags,
                int controllerCapabilityFlags, float ipd) {
    auto *env = (JNIEnv *) v_env;
    auto *instance = (jobject) v_instance;
    auto *deviceName_ = (jstring) v_deviceName;
    auto *broadcastAddrList_ = (jobjectArray) v_broadcastAddrList;
    auto *refreshRates_ = (jintArray) v_refreshRates;
    auto *fov = (jfloatArray) v_fov;

    //
    // Initialize variables
    //

    g_socket.m_stopped = false;
    g_socket.m_lastReceived = 0;
    g_socket.m_prevSentSync = 0;
    g_socket.m_prevSentBroadcast = 0;
    g_socket.m_prevVideoSequence = 0;
    g_socket.m_prevSoundSequence = 0;
    g_socket.m_timeDiff = 0;

    initializeJNICallbacks(env, instance);

    g_socket.m_nalParser = std::make_shared<NALParser>(env, instance);

    //
    // Fill hello message
    //

    memset(&g_socket.mHelloMessage, 0, sizeof(g_socket.mHelloMessage));

    g_socket.mHelloMessage.type = ALVR_PACKET_TYPE_HELLO_MESSAGE;
    memcpy(g_socket.mHelloMessage.signature, ALVR_HELLO_PACKET_SIGNATURE,
           sizeof(g_socket.mHelloMessage.signature));
    strcpy(g_socket.mHelloMessage.version, ALVR_VERSION);

    auto deviceName = GetStringFromJNIString(env, deviceName_);

    memcpy(g_socket.mHelloMessage.deviceName, deviceName.c_str(),
           std::min(deviceName.length(), sizeof(g_socket.mHelloMessage.deviceName)));

    jint *refreshRates = env->GetIntArrayElements(refreshRates_, nullptr);
    g_socket.mHelloMessage.refreshRate = refreshRates[0];
    env->ReleaseIntArrayElements(refreshRates_, refreshRates, 0);

    g_socket.mHelloMessage.renderWidth = static_cast<uint32_t>(renderWidth);
    g_socket.mHelloMessage.renderHeight = static_cast<uint32_t>(renderHeight);

    //
    // UdpSocket
    //

    g_socket.m_socket.setOnConnect(onConnect);
    g_socket.m_socket.setOnBroadcastRequest(onBroadcastRequest);
    g_socket.m_socket.setOnPacketRecv(onPacketRecv);
    g_socket.m_socket.initialize(env, helloPort, port, broadcastAddrList_);

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

    SendBuffer sendBuffer;
    while (1) {
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
        g_socket.m_socket.send(sendBuffer.buf, sendBuffer.len);
    }

    return;
}

void sendTimeSyncLocked() {
    time_t current = time(nullptr);
    if (g_socket.m_prevSentSync != current && g_socket.m_socket.isConnected()) {
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

        g_socket.m_socket.send(&timeSync, sizeof(timeSync));
    }
    g_socket.m_prevSentSync = current;
}

void sendBroadcastLocked() {
    if (g_socket.m_socket.isConnected()) {
        return;
    }

    time_t current = time(nullptr);
    if (g_socket.m_prevSentBroadcast != current) {
        LOGI("Sending broadcast hello.");
        g_socket.m_socket.sendBroadcast(&g_socket.mHelloMessage, sizeof(g_socket.mHelloMessage));
    }
    g_socket.m_prevSentBroadcast = current;
}

void checkConnection() {
    if (g_socket.m_socket.isConnected()) {
        if (g_socket.m_lastReceived + CONNECTION_TIMEOUT < getTimestampUs()) {
            // Timeout
            LOGE("Connection timeout.");
            g_socket.m_socket.disconnect();

            g_socket.m_env->CallVoidMethod(g_socket.m_instance, g_socket.mOnDisconnectedMethodID);

            if (g_socket.m_soundPlayer) {
                g_socket.m_soundPlayer->Stop();
            }
        }
    }
}

void doPeriodicWork() {
    sendTimeSyncLocked();
    sendBroadcastLocked();
    checkConnection();
}

void recoverConnection(std::string serverAddress, int serverPort) {
    g_socket.m_socket.recoverConnection(serverAddress, serverPort);
}

void sendNative(long long nativeBuffer, int length) {
    auto *packet = reinterpret_cast<char *>(nativeBuffer);

    if (g_socket.m_stopped) {
        return;
    }
    SendBuffer sendBuffer;

    memcpy(sendBuffer.buf, packet, length);
    sendBuffer.len = length;

    {
        std::lock_guard<decltype(g_socket.pipeMutex)> lock(g_socket.pipeMutex);
        g_socket.m_sendQueue.push_back(sendBuffer);
    }
    // Notify enqueue to loop thread
    write(g_socket.m_notifyPipe[1], "", 1);
}

void runLoop(void *v_env, void *v_instance, void *v_serverAddress, int serverPort) {
    auto *env = (JNIEnv *) v_env;
    auto *instance = (jobject) v_instance;
    auto *serverAddress = (jstring) v_serverAddress;

    fd_set fds, fds_org;

    FD_ZERO(&fds_org);
    FD_SET(g_socket.m_socket.getSocket(), &fds_org);
    FD_SET(g_socket.m_notifyPipe[0], &fds_org);
    int nfds = std::max(g_socket.m_socket.getSocket(), g_socket.m_notifyPipe[0]) + 1;

    g_socket.m_env = env;
    g_socket.m_instance = instance;

    if (serverAddress != NULL) {
        recoverConnection(GetStringFromJNIString(env, serverAddress), serverPort);
    }

    while (!g_socket.m_stopped) {
        timeval timeout;
        timeout.tv_sec = 0;
        timeout.tv_usec = 10 * 1000;
        memcpy(&fds, &fds_org, sizeof(fds));
        int ret = select(nfds, &fds, NULL, NULL, &timeout);

        if (ret == 0) {
            doPeriodicWork();

            // timeout
            continue;
        }

        if (FD_ISSET(g_socket.m_notifyPipe[0], &fds)) {
            //LOG("select pipe");
            processReadPipe(g_socket.m_notifyPipe[0]);
        }

        if (FD_ISSET(g_socket.m_socket.getSocket(), &fds)) {
            g_socket.m_socket.recv();
        }
        doPeriodicWork();
    }

    LOGI("Exited select loop.");

    if (g_socket.m_socket.isConnected()) {
        // Stop stream.
        StreamControlMessage message = {};
        message.type = ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE;
        message.mode = 2;
        g_socket.m_socket.send(&message, sizeof(message));
    }

    g_socket.m_soundPlayer.reset();

    LOGI("Exiting UdpReceiverThread runLoop");

    return;
}

void interruptNative() {
    g_socket.m_stopped = true;

    // Notify stop to loop thread.
    write(g_socket.m_notifyPipe[1], "", 1);
}

unsigned char isConnectedNative() {
    return g_socket.m_socket.isConnected();
}


void setSinkPreparedNative(unsigned char prepared) {
    if (g_socket.m_stopped) {
        return;
    }
    g_socket.mSinkPrepared = prepared;
    LOGSOCKETI("setSinkPrepared: Decoder prepared=%d", g_socket.mSinkPrepared);
    if (prepared && isConnectedNative()) {
        LOGSOCKETI("setSinkPrepared: Send stream start packet.");
        sendStreamStartPacket();
    }
}

void *getServerAddress(void *env) {
    return g_socket.m_socket.getServerAddress((JNIEnv *) env);
}

int getServerPort() {
    return g_socket.m_socket.getServerPort();
}