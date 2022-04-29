/// UdpReceiverThread jni functions using UDP socket
// Send tracking information and lost packet feedback to server.
// And receive screen video stream.
////////////////////////////////////////////////////////////////////

#include "bindings.h"
#include <jni.h>
#include "packet_types.h"
#include "nal.h"
#include "latency_collector.h"

class ServerConnectionNative {
public:
    bool m_connected = false;

    int64_t m_timeDiff = 0;
    uint64_t timeSyncSequence = (uint64_t) -1;
    uint64_t m_lastFrameIndex = 0;

    uint32_t m_prevVideoSequence = 0;
    std::shared_ptr<NALParser> m_nalParser;

    JNIEnv *m_env;
    jobject m_instance;
    jmethodID mOnDisconnectedMethodID;
};

namespace {
    ServerConnectionNative g_socket;
}

void initializeSocket(void *v_env, void *v_instance, void *v_nalClass, unsigned int codec,
                      bool enableFEC) {
    auto *env = (JNIEnv *) v_env;
    auto *instance = (jobject) v_instance;
    auto *nalClass = (jclass) v_nalClass;

    g_socket.m_env = env;
    g_socket.m_instance = env->NewGlobalRef(instance);

    g_socket.m_prevVideoSequence = 0;
    g_socket.m_timeDiff = 0;

    jclass clazz = env->GetObjectClass(instance);
    g_socket.mOnDisconnectedMethodID = env->GetMethodID(clazz, "onDisconnected", "()V");
    env->DeleteLocalRef(clazz);

    g_socket.m_nalParser = std::make_shared<NALParser>(enableFEC);
    g_socket.m_nalParser->setCodec(codec);

    LatencyCollector::Instance().resetAll();
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

void legacyReceive(const unsigned char *packet, unsigned int packetSize) {
    g_socket.m_connected = true;

    uint32_t type = *(uint32_t *) packet;
    if (type == ALVR_PACKET_TYPE_VIDEO_FRAME) {
        auto *header = (VideoFrame *) packet;

        if (g_socket.m_lastFrameIndex != header->trackingFrameIndex) {
            LatencyCollector::Instance().receivedFirst(header->trackingFrameIndex);
            if ((int64_t) header->sentTime - g_socket.m_timeDiff > (int64_t) getTimestampUs()) {
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
            videoErrorReportSend();
        }
    } else if (type == ALVR_PACKET_TYPE_TIME_SYNC) {
        // Time sync packet
        if (packetSize < sizeof(TimeSync)) {
            return;
        }
        auto *timeSync = (TimeSync *) packet;
        uint64_t Current = getTimestampUs();
        if (timeSync->mode == 1) {
            LatencyCollector::Instance().setTotalLatency(timeSync->serverTotalLatency);
            uint64_t RTT = Current - timeSync->clientTime;
            g_socket.m_timeDiff =
                    ((int64_t) timeSync->serverTime + (int64_t) RTT / 2) - (int64_t) Current;
            LOG("TimeSync: server - client = %ld us RTT = %lu us", g_socket.m_timeDiff, RTT);

            TimeSync sendBuf = *timeSync;
            sendBuf.mode = 2;
            sendBuf.clientTime = Current;
            timeSyncSend(sendBuf);
        }
        if (timeSync->mode == 3) {
            LatencyCollector::Instance().received(timeSync->trackingRecvFrameIndex);
        }
    }
}

void sendTimeSync() {
    LOG("Sending timesync.");

    TimeSync timeSync = {};
    timeSync.type = ALVR_PACKET_TYPE_TIME_SYNC;
    timeSync.mode = 0;
    timeSync.clientTime = getTimestampUs();
    timeSync.sequence = ++g_socket.timeSyncSequence;

    timeSync.packetsLostTotal = LatencyCollector::Instance().getPacketsLostTotal();
    timeSync.packetsLostInSecond = LatencyCollector::Instance().getPacketsLostInSecond();

    timeSync.averageTotalLatency = (uint32_t) LatencyCollector::Instance().getLatency(0);

    timeSync.averageSendLatency = (uint32_t) LatencyCollector::Instance().getLatency(3);

    timeSync.averageTransportLatency = (uint32_t) LatencyCollector::Instance().getLatency(1);

    timeSync.averageDecodeLatency = (uint64_t) LatencyCollector::Instance().getLatency(2);

    timeSync.idleTime = (uint32_t) LatencyCollector::Instance().getLatency(4);

    timeSync.fecFailure = g_socket.m_nalParser->fecFailure() ? 1 : 0;
    timeSync.fecFailureTotal = LatencyCollector::Instance().getFecFailureTotal();
    timeSync.fecFailureInSecond = LatencyCollector::Instance().getFecFailureInSecond();

    timeSync.fps = LatencyCollector::Instance().getFramesInSecond();

    timeSyncSend(timeSync);
}

unsigned char isConnectedNative() {
    return g_socket.m_connected;
}

void closeSocket(void *v_env) {
    auto *env = (JNIEnv *) v_env;

    g_socket.m_connected = false;

    env->CallVoidMethod(g_socket.m_instance, g_socket.mOnDisconnectedMethodID);
    env->DeleteGlobalRef(g_socket.m_instance);

    g_socket.m_nalParser.reset();
}