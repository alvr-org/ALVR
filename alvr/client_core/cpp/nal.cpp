/// H.264 NAL Parser functions
// Extract NAL Units from packet by UDP/SRT socket.
////////////////////////////////////////////////////////////////////

#include "bindings.h"
#include "fec.h"

enum ALVR_CODEC {
    ALVR_CODEC_H264 = 0,
    ALVR_CODEC_H265 = 1,
};

void (*pushNal)(const char *buffer, int length, unsigned long long frameIndex);

namespace {
bool m_enableFEC;
int m_codec = 1;
FECQueue m_queue;
} // namespace

void initializeNalParser(int codec, bool enableFec) {
    m_enableFEC = enableFec;
    m_codec = codec;
    m_queue = FECQueue();
}

void notifyNewDecoder() {
	m_queue.clearFecFailure();
}

bool processNalPacket(VideoFrame header,
                      const unsigned char *payload,
                      int payloadSize,
                      bool &outHadFecFailure) {
    if (m_enableFEC) {
        m_queue.addVideoPacket(header, payload, payloadSize, outHadFecFailure);
    }

    if (m_queue.reconstruct() || !m_enableFEC) {
        const std::byte *frameBuffer;
        int frameByteSize;
        if (m_enableFEC) {
            // Reconstructed
            frameBuffer = m_queue.getFrameBuffer();
            frameByteSize = m_queue.getFrameByteSize();
        } else {
            frameBuffer = (const std::byte *)payload;
            frameByteSize = payloadSize;
        }

        pushNal((const char *)&frameBuffer[0], frameByteSize, header.trackingFrameIndex);

        return true;
    }
    return false;
}
