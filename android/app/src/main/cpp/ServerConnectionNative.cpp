/// UdpReceiverThread jni functions using UDP socket
// Send tracking information and lost packet feedback to server.
// And receive screen video stream.
////////////////////////////////////////////////////////////////////

#include "bindings.h"
#include <jni.h>
#include "packet_types.h"
#include "nal.h"

class ServerConnectionNative {
public:
    bool m_connected = false;

    uint32_t m_prevVideoSequence = 0;
    std::shared_ptr<NALParser> m_nalParser;
};

namespace {
    ServerConnectionNative g_socket;
}

void initializeSocket(unsigned int codec, bool enableFEC) {
    g_socket.m_prevVideoSequence = 0;

    g_socket.m_nalParser = std::make_shared<NALParser>(enableFEC);
    g_socket.m_nalParser->setCodec(codec);
}

void processVideoSequence(uint32_t sequence) {
    if (g_socket.m_prevVideoSequence != 0 && g_socket.m_prevVideoSequence + 1 != sequence) {
        int32_t lost = sequence - (g_socket.m_prevVideoSequence + 1);
        if (lost < 0) {
            // lost become negative on out-of-order packet.
            // TODO: This is not accurate statistics.
            lost = -lost;
        }

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

        processVideoSequence(header->packetCounter);

        // Following packets of a video frame
        bool fecFailure = false;
        bool ret2 = g_socket.m_nalParser->processPacket(header, packetSize, fecFailure);
        if (fecFailure) {
            videoErrorReportSend();
        }
    }
}

unsigned char isConnectedNative() {
    return g_socket.m_connected;
}

void closeSocket() {
    g_socket.m_connected = false;

    g_socket.m_nalParser.reset();
}