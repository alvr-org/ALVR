/// H.264 NAL Parser functions
// Extract NAL Units from packet by UDP/SRT socket.
////////////////////////////////////////////////////////////////////

#include <string>
#include <stdlib.h>
#include <android/log.h>
#include <pthread.h>
#include "nal.h"
#include "packet_types.h"

static const std::byte NAL_TYPE_SPS = static_cast<const std::byte>(7);

static const std::byte H265_NAL_TYPE_VPS = static_cast<const std::byte>(32);


NALParser::NALParser(JNIEnv *env, jobject udpManager, jclass nalClass, bool enableFEC)
    : m_enableFEC(enableFEC)
{
    LOGE("NALParser initialized %p", this);

    m_env = env;
    mUdpManager = env->NewGlobalRef(udpManager);

    NAL_length = env->GetFieldID(nalClass, "length", "I");
    NAL_frameIndex = env->GetFieldID(nalClass, "frameIndex", "J");
    NAL_buf = env->GetFieldID(nalClass, "buf", "[B");

    jclass activityClass = env->GetObjectClass(udpManager);
    mObtainNALMethodID = env->GetMethodID(activityClass, "obtainNAL",
                                          "(I)Lcom/polygraphene/alvr/NAL;");
    mPushNALMethodID = env->GetMethodID(activityClass, "pushNAL",
                                        "(Lcom/polygraphene/alvr/NAL;)V");
}

NALParser::~NALParser()
{
    m_env->DeleteGlobalRef(mUdpManager);
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
            push(&frameBuffer[0], end, packet->trackingFrameIndex);
            push(&frameBuffer[end], frameByteSize - end, packet->trackingFrameIndex);

            m_queue.clearFecFailure();
        } else
        {
            push(&frameBuffer[0], frameByteSize, packet->trackingFrameIndex);
        }
        return true;
    }
    return false;
}

void NALParser::push(const std::byte *buffer, int length, uint64_t frameIndex)
{
    jobject nal;
    jbyteArray buf;

    nal = m_env->CallObjectMethod(mUdpManager, mObtainNALMethodID, static_cast<jint>(length));
    if (nal == nullptr)
    {
        LOGE("NAL Queue is full.");
        return;
    }

    m_env->SetIntField(nal, NAL_length, length);
    m_env->SetLongField(nal, NAL_frameIndex, frameIndex);

    buf = (jbyteArray) m_env->GetObjectField(nal, NAL_buf);
    std::byte *cbuf = (std::byte *) m_env->GetByteArrayElements(buf, NULL);

    memcpy(cbuf, buffer, length);
    m_env->ReleaseByteArrayElements(buf, (jbyte *) cbuf, 0);
    m_env->DeleteLocalRef(buf);

    m_env->CallVoidMethod(mUdpManager, mPushNALMethodID, nal);

    m_env->DeleteLocalRef(nal);
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
