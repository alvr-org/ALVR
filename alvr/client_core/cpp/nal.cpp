/// H.264 NAL Parser functions
// Extract NAL Units from packet by UDP/SRT socket.
////////////////////////////////////////////////////////////////////

#include "bindings.h"
#include "fec.h"

static const std::byte NAL_TYPE_SPS = static_cast<const std::byte>(7);
static const std::byte H265_NAL_TYPE_VPS = static_cast<const std::byte>(32);

enum ALVR_CODEC {
    ALVR_CODEC_H264 = 0,
    ALVR_CODEC_H265 = 1,
};

void (*createDecoder)(const char *csd_0, int length);
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

int findVPSSPS(const std::byte *frameBuffer, int frameByteSize) {
    int zeroes = 0;
    int foundNals = 0;
    for (int i = 0; i < frameByteSize; i++) {
        if (frameBuffer[i] == std::byte(0)) {
            zeroes++;
        } else if (frameBuffer[i] == std::byte(1)) {
            if (zeroes >= 2) {
                foundNals++;
                if (m_codec == ALVR_CODEC_H264 && foundNals >= 3) {
                    // Find end of SPS+PPS on H.264.
                    return i - 3;
                } else if (m_codec == ALVR_CODEC_H265 && foundNals >= 4) {
                    // Find end of VPS+SPS+PPS on H.264.
                    return i - 3;
                }
            }
            zeroes = 0;
        } else {
            zeroes = 0;
        }
    }
    return -1;
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

        std::byte NALType;
        if (m_codec == ALVR_CODEC_H264)
            NALType = frameBuffer[4] & std::byte(0x1F);
        else
            NALType = (frameBuffer[4] >> 1) & std::byte(0x3F);

        if ((m_codec == ALVR_CODEC_H264 && NALType == NAL_TYPE_SPS) ||
            (m_codec == ALVR_CODEC_H265 && NALType == H265_NAL_TYPE_VPS)) {
            // This frame contains (VPS + )SPS + PPS + IDR on NVENC H.264 (H.265) stream.
            // (VPS + )SPS + PPS has short size (8bytes + 28bytes in some environment), so we can
            // assume SPS + PPS is contained in first fragment.

            int end = findVPSSPS(frameBuffer, frameByteSize);
            if (end == -1) {
                // Invalid frame.
                return false;
            }
            createDecoder((const char *)&frameBuffer[0], end);
            pushNal(
                (const char *)&frameBuffer[end], frameByteSize - end, header.trackingFrameIndex);

            m_queue.clearFecFailure();
        } else {
            pushNal((const char *)&frameBuffer[0], frameByteSize, header.trackingFrameIndex);
        }
        return true;
    }
    return false;
}
