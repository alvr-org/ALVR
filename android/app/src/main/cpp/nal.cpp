/// H.264 NAL Parser functions
// Extract NAL Units from packet by UDP/SRT socket.
////////////////////////////////////////////////////////////////////

#include <string>
#include <stdlib.h>
#include <android/log.h>
#include <pthread.h>
#include "nal.h"
#include "packet_types.h"
#include "bindings.h"

static const std::byte NAL_TYPE_SPS = static_cast<const std::byte>(7);

static const std::byte H265_NAL_TYPE_VPS = static_cast<const std::byte>(32);

void (*pushNal)(const char *buffer, int length, unsigned long long frameIndex);

NALParser::NALParser(bool enableFEC) : m_enableFEC(enableFEC)
{
}

void NALParser::setCodec(int codec)
{
    m_codec = codec;
}

bool NALParser::processPacket(VideoFrame *packet, int packetSize, bool &fecFailure)
{
    if (m_enableFEC) {
        m_queue.addVideoPacket(packet, packetSize, fecFailure);
    }

    bool result = m_queue.reconstruct() || !m_enableFEC;
    if (result)
    {
        const std::byte *frameBuffer;
        int frameByteSize;
        if (m_enableFEC) {
            // Reconstructed
            frameBuffer = m_queue.getFrameBuffer();
            frameByteSize = m_queue.getFrameByteSize();
        } else {
            frameBuffer = reinterpret_cast<const std::byte *>(packet) + sizeof(VideoFrame);
            frameByteSize = packetSize - sizeof(VideoFrame);
        }

        std::byte NALType;
        if (m_codec == ALVR_CODEC_H264)
            NALType = frameBuffer[4] & std::byte(0x1F);
        else
            NALType = (frameBuffer[4] >> 1) & std::byte(0x3F);

        if ((m_codec == ALVR_CODEC_H264 && NALType == NAL_TYPE_SPS) ||
            (m_codec == ALVR_CODEC_H265 && NALType == H265_NAL_TYPE_VPS))
        {
            // This frame contains (VPS + )SPS + PPS + IDR on NVENC H.264 (H.265) stream.
            // (VPS + )SPS + PPS has short size (8bytes + 28bytes in some environment), so we can assume SPS + PPS is contained in first fragment.

            int end = findVPSSPS(frameBuffer, frameByteSize);
            if (end == -1)
            {
                // Invalid frame.
                LOG("Got invalid frame. Too large SPS or PPS?");
                return false;
            }
            LOGI("Got frame=%d %d, Codec=%d", (std::int32_t) NALType, end, m_codec);
            pushNal((const char *)&frameBuffer[0], end, packet->trackingFrameIndex);
            pushNal((const char *)&frameBuffer[end], frameByteSize - end, packet->trackingFrameIndex);

            m_queue.clearFecFailure();
        } else
        {
            pushNal((const char *)&frameBuffer[0], frameByteSize, packet->trackingFrameIndex);
        }
        return true;
    }
    return false;
}

bool NALParser::fecFailure()
{
    return m_queue.fecFailure();
}

int NALParser::findVPSSPS(const std::byte *frameBuffer, int frameByteSize)
{
    int zeroes = 0;
    int foundNals = 0;
    for (int i = 0; i < frameByteSize; i++)
    {
        if (frameBuffer[i] == std::byte(0))
        {
            zeroes++;
        }
        else if (frameBuffer[i] == std::byte(1))
        {
            if (zeroes >= 2)
            {
                foundNals++;
                if (m_codec == ALVR_CODEC_H264 && foundNals >= 3)
                {
                    // Find end of SPS+PPS on H.264.
                    return i - 3;
                } else if (m_codec == ALVR_CODEC_H265 && foundNals >= 4)
                {
                    // Find end of VPS+SPS+PPS on H.264.
                    return i - 3;
                }
            }
            zeroes = 0;
        } else
        {
            zeroes = 0;
        }
    }
    return -1;
}
