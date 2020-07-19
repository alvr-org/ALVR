/// UdpReceiverThread jni functions using UDP socket
// Send tracking information and lost packet feedback to server.
// And receive screen video stream.
////////////////////////////////////////////////////////////////////

#include <stdlib.h>
#include <pthread.h>
#include <endian.h>
#include <algorithm>
#include <errno.h>
#include <sys/ioctl.h>

#include "utils.h"
#include "latency_collector.h"
#include "ServerConnectionNative.h"
#include "exception.h"


ServerConnectionNative::ServerConnectionNative() {
}

ServerConnectionNative::~ServerConnectionNative() {
    if (m_notifyPipe[0] >= 0) {
        close(m_notifyPipe[0]);
        close(m_notifyPipe[1]);
    }

    m_nalParser.reset();
    m_sendQueue.clear();
}

void ServerConnectionNative::initialize(JNIEnv *env, jobject instance, jint helloPort, jint port, jstring deviceName_,
                            jobjectArray broadcastAddrList_, jintArray refreshRates_, jint renderWidth,
                            jint renderHeight, jfloatArray fov, jint deviceType, jint deviceSubType,
                            jint deviceCapabilityFlags, jint controllerCapabilityFlags) {
    //
    // Initialize variables
    //

    m_stopped = false;
    m_lastReceived = 0;
    m_prevSentSync = 0;
    m_prevSentBroadcast = 0;
    m_prevVideoSequence = 0;
    m_prevSoundSequence = 0;
    m_timeDiff = 0;

    initializeJNICallbacks(env, instance);

    m_nalParser = std::make_shared<NALParser>(env, instance);

    //
    // Fill hello message
    //

    memset(&mHelloMessage, 0, sizeof(mHelloMessage));

    mHelloMessage.type = ALVR_PACKET_TYPE_HELLO_MESSAGE;
    memcpy(mHelloMessage.signature, ALVR_HELLO_PACKET_SIGNATURE, sizeof(mHelloMessage.signature));
    strcpy(mHelloMessage.version, ALVR_VERSION);

    auto deviceName = GetStringFromJNIString(env, deviceName_);

    memcpy(mHelloMessage.deviceName, deviceName.c_str(),
           std::min(deviceName.length(), sizeof(mHelloMessage.deviceName)));

    mHelloMessage.refreshRate = 72;

    mHelloMessage.renderWidth = static_cast<uint32_t>(renderWidth);
    mHelloMessage.renderHeight = static_cast<uint32_t>(renderHeight);

    loadFov(env, fov);

    //
    // UdpSocket
    //

    m_socket.setOnConnect(std::bind(&ServerConnectionNative::onConnect, this, std::placeholders::_1));
    m_socket.setOnBroadcastRequest(std::bind(&ServerConnectionNative::onBroadcastRequest, this));
    m_socket.setOnPacketRecv(std::bind(&ServerConnectionNative::onPacketRecv, this, std::placeholders::_1,
                                       std::placeholders::_2));
    m_socket.initialize(env, helloPort, port, broadcastAddrList_);

    //
    // Sound
    //

    m_soundPlayer = std::make_shared<SoundPlayer>();
    if (m_soundPlayer->initialize() != 0) {
        LOGE("Failed on SoundPlayer initialize.");
        m_soundPlayer.reset();
    }
    LOGI("SoundPlayer successfully initialize.");

    //
    // Pipe used for send buffer notification.
    //

    if (pipe2(m_notifyPipe, O_NONBLOCK) < 0) {
        throw FormatException("pipe2 error : %d %s", errno, strerror(errno));
    }

    LOGI("ServerConnectionNative initialized.");
}

void ServerConnectionNative::sendPacketLossReport(ALVR_LOST_FRAME_TYPE frameType, uint32_t fromPacketCounter,
                                      uint32_t toPacketCounter) {
    PacketErrorReport report;
    report.type = ALVR_PACKET_TYPE_PACKET_ERROR_REPORT;
    report.lostFrameType = frameType;
    report.fromPacketCounter = fromPacketCounter;
    report.toPacketCounter = toPacketCounter;
    int ret = m_socket.send(&report, sizeof(report));
    LOGI("Sent packet loss report. ret=%d", ret);
}

void ServerConnectionNative::processVideoSequence(uint32_t sequence) {
    if (m_prevVideoSequence != 0 && m_prevVideoSequence + 1 != sequence) {
        int32_t lost = sequence - (m_prevVideoSequence + 1);
        if (lost < 0) {
            // lost become negative on out-of-order packet.
            // TODO: This is not accurate statistics.
            lost = -lost;
        }
        LatencyCollector::Instance().packetLoss(lost);

        LOGE("VideoPacket loss %d (%d -> %d)", lost, m_prevVideoSequence + 1,
             sequence - 1);
    }
    m_prevVideoSequence = sequence;
}

void ServerConnectionNative::processSoundSequence(uint32_t sequence) {
    if (m_prevSoundSequence != 0 && m_prevSoundSequence + 1 != sequence) {
        int32_t lost = sequence - (m_prevSoundSequence + 1);
        if (lost < 0) {
            // lost become negative on out-of-order packet.
            // TODO: This is not accurate statistics.
            lost = -lost;
        }
        LatencyCollector::Instance().packetLoss(lost);

        sendPacketLossReport(ALVR_LOST_FRAME_TYPE_AUDIO, m_prevSoundSequence + 1, sequence - 1);

        LOGE("SoundPacket loss %d (%d -> %d)", lost, m_prevSoundSequence + 1,
             sequence - 1);
    }
    m_prevSoundSequence = sequence;
}

void ServerConnectionNative::processReadPipe(int pipefd) {
    char buf[2000];
    int len = 1;

    int ret = static_cast<int>(read(pipefd, buf, len));
    if (ret <= 0)
    {
        return;
    }

    SendBuffer sendBuffer;
    while (1) {
        {
            std::lock_guard<std::mutex> lock(pipeMutex);

            if (m_sendQueue.empty())
            {
                break;
            }
            else
            {
                sendBuffer = m_sendQueue.front();
                m_sendQueue.pop_front();
            }
        }
        if (m_stopped)
        {
            return;
        }

        //LOG("Sending tracking packet %d", sendBuffer.len);
        m_socket.send(sendBuffer.buf, sendBuffer.len);
    }

    return;
}

void ServerConnectionNative::sendTimeSyncLocked()
{
    time_t current = time(nullptr);
    if (m_prevSentSync != current && m_socket.isConnected()) {
        LOGI("Sending timesync.");

        TimeSync timeSync = {};
        timeSync.type = ALVR_PACKET_TYPE_TIME_SYNC;
        timeSync.mode = 0;
        timeSync.clientTime = getTimestampUs();
        timeSync.sequence = ++timeSyncSequence;

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

        timeSync.fecFailure = m_nalParser->fecFailure() ? 1 : 0;
        timeSync.fecFailureTotal = LatencyCollector::Instance().getFecFailureTotal();
        timeSync.fecFailureInSecond = LatencyCollector::Instance().getFecFailureInSecond();

        timeSync.fps = LatencyCollector::Instance().getFramesInSecond();

        m_socket.send(&timeSync, sizeof(timeSync));
    }
    m_prevSentSync = current;
}

void ServerConnectionNative::sendBroadcastLocked()
{
    if(m_socket.isConnected()) {
        return;
    }

    time_t current = time(nullptr);
    if (m_prevSentBroadcast != current) {
        LOGI("Sending broadcast hello.");
        m_socket.sendBroadcast(&mHelloMessage, sizeof(mHelloMessage));
    }
    m_prevSentBroadcast = current;
}

void ServerConnectionNative::doPeriodicWork()
{
    sendTimeSyncLocked();
    sendBroadcastLocked();
    checkConnection();
}

void ServerConnectionNative::recoverConnection(std::string serverAddress, int serverPort)
{
    m_socket.recoverConnection(serverAddress, serverPort);
}

void ServerConnectionNative::send(const void *packet, int length)
{
    if (m_stopped) {
        return;
    }
    SendBuffer sendBuffer;

    memcpy(sendBuffer.buf, packet, length);
    sendBuffer.len = length;

    {
        std::lock_guard<decltype(pipeMutex)> lock(pipeMutex);
        m_sendQueue.push_back(sendBuffer);
    }
    // Notify enqueue to loop thread
    write(m_notifyPipe[1], "", 1);
}

void ServerConnectionNative::runLoop(JNIEnv *env, jobject instance, jstring serverAddress, int serverPort)
{
    fd_set fds, fds_org;

    FD_ZERO(&fds_org);
    FD_SET(m_socket.getSocket(), &fds_org);
    FD_SET(m_notifyPipe[0], &fds_org);
    int nfds = std::max(m_socket.getSocket(), m_notifyPipe[0]) + 1;

    m_env = env;
    m_instance = instance;

    if (serverAddress != NULL) {
        recoverConnection(GetStringFromJNIString(env, serverAddress), serverPort);
    }

    while (!m_stopped) {
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

        if (FD_ISSET(m_notifyPipe[0], &fds)) {
            //LOG("select pipe");
            processReadPipe(m_notifyPipe[0]);
        }

        if (FD_ISSET(m_socket.getSocket(), &fds)) {
            m_socket.recv();
        }
        doPeriodicWork();
    }

    LOGI("Exited select loop.");

    if (m_socket.isConnected()) {
        // Stop stream.
        StreamControlMessage message = {};
        message.type = ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE;
        message.mode = 2;
        m_socket.send(&message, sizeof(message));
    }

    m_soundPlayer.reset();

    LOGI("Exiting UdpReceiverThread runLoop");

    return;
}

void ServerConnectionNative::interrupt() {
    m_stopped = true;

    // Notify stop to loop thread.
    write(m_notifyPipe[1], "", 1);
}

void ServerConnectionNative::setSinkPrepared(bool prepared) {
    if (m_stopped) {
        return;
    }
    mSinkPrepared = prepared;
    LOGSOCKETI("setSinkPrepared: Decoder prepared=%d", mSinkPrepared);
    if (prepared && isConnected()) {
        LOGSOCKETI("setSinkPrepared: Send stream start packet.");
        sendStreamStartPacket();
    }
}

bool ServerConnectionNative::isConnected() {
    return m_socket.isConnected();
}

jstring ServerConnectionNative::getServerAddress(JNIEnv *env) {
    return m_socket.getServerAddress(env);
}

int ServerConnectionNative::getServerPort() {
    return m_socket.getServerPort();
}

void ServerConnectionNative::onConnect(const ConnectionMessage &connectionMessage) {
    // Save video width and height
    m_connectionMessage = connectionMessage;

    updateTimeout();
    m_prevVideoSequence = 0;
    m_prevSoundSequence = 0;
    m_timeDiff = 0;
    LatencyCollector::Instance().resetAll();
    m_nalParser->setCodec(m_connectionMessage.codec);

    m_env->CallVoidMethod(m_instance, mOnConnectMethodID, m_connectionMessage.videoWidth
            , m_connectionMessage.videoHeight, m_connectionMessage.codec
            , m_connectionMessage.frameQueueSize, m_connectionMessage.refreshRate, m_connectionMessage.streamMic,
            m_connectionMessage.foveationMode,
            m_connectionMessage.foveationStrength,
            m_connectionMessage.foveationShape,
            m_connectionMessage.foveationVerticalOffset);

    jstring jstr = m_env->NewStringUTF(m_connectionMessage.webGuiUrl);
    m_env->CallVoidMethod(m_instance, mSetWebGuiUrlID, jstr);

    if (mSinkPrepared) {
        LOGSOCKETI("onConnect: Send stream start packet.");
        sendStreamStartPacket();
    }
}

void ServerConnectionNative::onBroadcastRequest() {
    // Respond with hello message.
    m_socket.send(&mHelloMessage, sizeof(mHelloMessage));
}

void ServerConnectionNative::onPacketRecv(const char *packet, size_t packetSize) {
    updateTimeout();

    uint32_t type = *(uint32_t *) packet;
    if (type == ALVR_PACKET_TYPE_VIDEO_FRAME) {
        VideoFrame *header = (VideoFrame *) packet;

        if (m_lastFrameIndex != header->trackingFrameIndex) {
            LatencyCollector::Instance().receivedFirst(header->trackingFrameIndex);
            if ((int64_t) header->sentTime - m_timeDiff > getTimestampUs()) {
                LatencyCollector::Instance().estimatedSent(header->trackingFrameIndex, 0);
            } else {
                LatencyCollector::Instance().estimatedSent(header->trackingFrameIndex,
                                                           (int64_t) header->sentTime -
                                                           m_timeDiff - getTimestampUs());
            }
            m_lastFrameIndex = header->trackingFrameIndex;
        }

        processVideoSequence(header->packetCounter);

        // Following packets of a video frame
        bool fecFailure = false;
        bool ret2 = m_nalParser->processPacket(header, packetSize, fecFailure);
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
            m_timeDiff = ((int64_t) timeSync->serverTime + (int64_t) RTT / 2) - (int64_t) Current;
            LOGI("TimeSync: server - client = %ld us RTT = %lu us", m_timeDiff, RTT);

            TimeSync sendBuf = *timeSync;
            sendBuf.mode = 2;
            sendBuf.clientTime = Current;
            m_socket.send(&sendBuf, sizeof(sendBuf));
        }
    } else if (type == ALVR_PACKET_TYPE_CHANGE_SETTINGS) {
        // Change settings
        if (packetSize < sizeof(ChangeSettings)) {
            return;
        }
        ChangeSettings *settings = (ChangeSettings *) packet;

        m_env->CallVoidMethod(m_instance, mOnChangeSettingsMethodID, settings->debugFlags, settings->suspend, settings->frameQueueSize);
    } else if (type == ALVR_PACKET_TYPE_AUDIO_FRAME_START) {
        // Change settings
        if (packetSize < sizeof(AudioFrameStart)) {
            return;
        }
        auto header = (AudioFrameStart *) packet;

        processSoundSequence(header->packetCounter);

        if (m_soundPlayer) {
            m_soundPlayer->putData((uint8_t *) packet + sizeof(*header),
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

        if (m_soundPlayer) {
            m_soundPlayer->putData((uint8_t *) packet + sizeof(*header),
                                   packetSize - sizeof(*header));
        }

        //LOG("Received audio frame: Counter=%d", header->packetCounter);
    } else if (type == ALVR_PACKET_TYPE_HAPTICS) {
        if(packetSize < sizeof(HapticsFeedback)) {
            return;
        }
        auto header = (HapticsFeedback *) packet;

        m_env->CallVoidMethod(m_instance, mOnHapticsFeedbackID, static_cast<jlong>(header->startTime),
                header->amplitude, header->duration, header->frequency, static_cast<jboolean>(header->hand));

    } else if (type == ALVR_PACKET_TYPE_GUARDIAN_SYNC_ACK) {
        if (packetSize < sizeof(GuardianSyncStartAck)) {
            return;
        }

        auto ack = (GuardianSyncStartAck *) packet;

        m_env->CallVoidMethod(m_instance, mOnGuardianSyncAckID, static_cast<jlong>(ack->timestamp));
    } else if (type == ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_ACK) {
        if (packetSize < sizeof(GuardianSegmentAck)) {
            return;
        }

        auto ack = (GuardianSegmentAck *) packet;

        m_env->CallVoidMethod(m_instance, mOnGuardianSegmentAckID,
                              static_cast<jlong>(ack->timestamp),
                              static_cast<jint>(ack->segmentIndex));
    }
}

void ServerConnectionNative::checkConnection() {
    if (m_socket.isConnected()) {
        if (m_lastReceived + CONNECTION_TIMEOUT < getTimestampUs()) {
            // Timeout
            LOGE("Connection timeout.");
            m_socket.disconnect();

            m_env->CallVoidMethod(m_instance, mOnDisconnectedMethodID);

            if (m_soundPlayer) {
                m_soundPlayer->Stop();
            }
        }
    }
}

void ServerConnectionNative::updateTimeout() {
    m_lastReceived = getTimestampUs();
}

void ServerConnectionNative::loadFov(JNIEnv *env, jfloatArray fov_) {
    jfloat *fov = env->GetFloatArrayElements(fov_, nullptr);
    for(int eye = 0; eye < 2; eye++) {
        mHelloMessage.eyeFov[eye].left = fov[eye * 4 + 0];
        mHelloMessage.eyeFov[eye].right = fov[eye * 4 + 1];
        mHelloMessage.eyeFov[eye].top = fov[eye * 4 + 2];
        mHelloMessage.eyeFov[eye].bottom = fov[eye * 4 + 3];
    }
    env->ReleaseFloatArrayElements(fov_, fov, 0);
}

void ServerConnectionNative::sendStreamStartPacket() {
    LOGSOCKETI("Sending stream start packet.");
    // Start stream.
    StreamControlMessage message = {};
    message.type = ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE;
    message.mode = 1;
    m_socket.send(&message, sizeof(message));
}

void ServerConnectionNative::initializeJNICallbacks(JNIEnv *env, jobject instance) {
    jclass clazz = env->GetObjectClass(instance);

    mOnConnectMethodID = env->GetMethodID(clazz, "onConnected", "(IIIIIZIFFF)V");
    mOnChangeSettingsMethodID = env->GetMethodID(clazz, "onChangeSettings", "(JII)V");
    mOnDisconnectedMethodID = env->GetMethodID(clazz, "onDisconnected", "()V");
    mOnHapticsFeedbackID = env->GetMethodID(clazz, "onHapticsFeedback", "(JFFFZ)V");
    mSetWebGuiUrlID = env->GetMethodID(clazz, "setWebViewURL", "(Ljava/lang/String;)V");
    mOnGuardianSyncAckID = env->GetMethodID(clazz, "onGuardianSyncAck", "(J)V");
    mOnGuardianSegmentAckID = env->GetMethodID(clazz, "onGuardianSegmentAck", "(JI)V");

    env->DeleteLocalRef(clazz);
}

extern "C"
JNIEXPORT jlong JNICALL
Java_com_polygraphene_alvr_ServerConnection_initializeSocket(
        JNIEnv *env, jobject instance,
        jint helloPort, jint port, jstring deviceName_, jobjectArray broadcastAddrList_,
        jintArray refreshRates_, jint renderWidth, jint renderHeight, jfloatArray fov,
        jint deviceType, jint deviceSubType, jint deviceCapabilityFlags, jint controllerCapabilityFlags) {
    auto udpManager = new ServerConnectionNative();
    try {
        udpManager->initialize(env, instance, helloPort, port, deviceName_,
                               broadcastAddrList_, refreshRates_, renderWidth, renderHeight, fov,
                               deviceType, deviceSubType, deviceCapabilityFlags,
                               controllerCapabilityFlags);
    } catch (Exception &e) {
        LOGE("Exception on initializing ServerConnectionNative. e=%ls", e.what());
        delete udpManager;
        return 0;
    }
    return reinterpret_cast<jlong>(udpManager);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_ServerConnection_closeSocket(JNIEnv *env, jobject instance, jlong nativeHandle) {
    delete reinterpret_cast<ServerConnectionNative *>(nativeHandle);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_ServerConnection_runLoop(JNIEnv *env, jobject instance, jlong nativeHandle,
                                                     jstring serverAddress, jint serverPort) {
    reinterpret_cast<ServerConnectionNative *>(nativeHandle)->runLoop(env, instance, serverAddress, serverPort);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_ServerConnection_interruptNative(JNIEnv *env, jobject instance, jlong nativeHandle) {
    reinterpret_cast<ServerConnectionNative *>(nativeHandle)->interrupt();
}

extern "C"
JNIEXPORT jboolean JNICALL
Java_com_polygraphene_alvr_ServerConnection_isConnectedNative(JNIEnv *env, jobject instance, jlong nativeHandle) {
    return nativeHandle != 0 && reinterpret_cast<ServerConnectionNative *>(nativeHandle)->isConnected();
}

extern "C"
JNIEXPORT jstring JNICALL
Java_com_polygraphene_alvr_ServerConnection_getServerAddress(JNIEnv *env, jobject instance, jlong nativeHandle) {
    return reinterpret_cast<ServerConnectionNative *>(nativeHandle)->getServerAddress(env);
}

extern "C"
JNIEXPORT jint JNICALL
Java_com_polygraphene_alvr_ServerConnection_getServerPort(JNIEnv *env, jobject instance, jlong nativeHandle) {
    return reinterpret_cast<ServerConnectionNative *>(nativeHandle)->getServerPort();
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_ServerConnection_sendNative(JNIEnv *env, jobject instance,
                                                        jlong nativeHandle, jlong nativeBuffer,
                                                        jint bufferLength) {
    return reinterpret_cast<ServerConnectionNative *>(nativeHandle)->send(reinterpret_cast<char*>(nativeBuffer), bufferLength);
}

extern "C"
JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_ServerConnection_setSinkPreparedNative(JNIEnv *env, jobject instance, jlong nativeHandle, jboolean prepared) {
    reinterpret_cast<ServerConnectionNative *>(nativeHandle)->setSinkPrepared(static_cast<bool>(prepared));
}
