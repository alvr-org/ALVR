#include <vector>
#include <algorithm>
#include <stdlib.h>
#include <inttypes.h>
#include "fec.h"
#include "packet_types.h"
#include "utils.h"

bool FECQueue::reed_solomon_initialized = false;

FECQueue::FECQueue() {
    m_currentFrame.videoFrameIndex = UINT64_MAX;
    m_recovered = true;
    m_fecFailure = false;

    if (!reed_solomon_initialized) {
        reed_solomon_init();
        reed_solomon_initialized = true;
    }
}

FECQueue::~FECQueue() {
    if (m_rs != NULL) {
        reed_solomon_release(m_rs);
    }
}

// Add packet to queue. packet must point to buffer whose size=ALVR_MAX_PACKET_SIZE.
void FECQueue::addVideoPacket(const VideoFrame *packet, int packetSize, bool &fecFailure) {
    if (m_recovered && m_currentFrame.videoFrameIndex == packet->videoFrameIndex) {
        return;
    }
    if (m_currentFrame.videoFrameIndex != packet->videoFrameIndex) {
        // New frame
        if (!m_recovered) {
            FrameLog(m_currentFrame.trackingFrameIndex,
                     "Previous frame cannot be recovered. videoFrame=%llu shards=%u:%u frameByteSize=%d"
                     " fecPercentage=%d m_totalShards=%u m_shardPackets=%u m_blockSize=%u",
                     m_currentFrame.videoFrameIndex,
                     m_totalDataShards,
                     m_totalParityShards,
                     m_currentFrame.frameByteSize, m_currentFrame.fecPercentage, m_totalShards,
                     m_shardPackets, m_blockSize);
            for (size_t packet = 0; packet < m_shardPackets; packet++) {
                FrameLog(m_currentFrame.trackingFrameIndex,
                         "packetIndex=%d, shards=%u:%u",
                         packet, m_receivedDataShards[packet], m_receivedParityShards[packet]);
            }
            fecFailure = m_fecFailure = true;
        }
        m_currentFrame = *packet;
        m_recovered = false;
        if (m_rs != NULL) {
            reed_solomon_release(m_rs);
        }

        uint32_t fecDataPackets = (packet->frameByteSize + ALVR_MAX_VIDEO_BUFFER_SIZE - 1) /
                                  ALVR_MAX_VIDEO_BUFFER_SIZE;
        m_shardPackets = CalculateFECShardPackets(m_currentFrame.frameByteSize,
                                                  m_currentFrame.fecPercentage);
        m_blockSize = m_shardPackets * ALVR_MAX_VIDEO_BUFFER_SIZE;

        m_totalDataShards = (m_currentFrame.frameByteSize + m_blockSize - 1) / m_blockSize;
        m_totalParityShards = CalculateParityShards(m_totalDataShards,
                                                    m_currentFrame.fecPercentage);
        m_totalShards = m_totalDataShards + m_totalParityShards;

        m_recoveredPacket.clear();
        m_recoveredPacket.resize(m_shardPackets);

        m_receivedDataShards.clear();
        m_receivedDataShards.resize(m_shardPackets);
        m_receivedParityShards.clear();
        m_receivedParityShards.resize(m_shardPackets);

        m_shards.resize(m_totalShards);

        m_rs = reed_solomon_new(m_totalDataShards, m_totalParityShards);
        if (m_rs == NULL) {
            return;
        }

        m_marks.resize(m_shardPackets);
        for (size_t i = 0; i < m_shardPackets; i++) {
            m_marks[i].resize(m_totalShards);
            memset(&m_marks[i][0], 1, m_totalShards);
        }

        if (m_frameBuffer.size() < m_totalShards * m_blockSize) {
            // Only expand buffer for performance reason.
            m_frameBuffer.resize(m_totalShards * m_blockSize);
        }
        memset(&m_frameBuffer[0], 0, m_totalShards * m_blockSize);

        // Padding packets are not sent, so we can fill bitmap by default.
        size_t padding = (m_shardPackets - fecDataPackets % m_shardPackets) % m_shardPackets;
        for (size_t i = 0; i < padding; i++) {
            m_marks[m_shardPackets - i - 1][m_totalDataShards - 1] = 0;
            m_receivedDataShards[m_shardPackets - i - 1]++;
        }

        // Calculate last packet counter of current frame to detect whole frame packet loss.
        uint32_t startPacket;
        uint32_t nextStartPacket;
        if(m_currentFrame.fecIndex / m_shardPackets < m_totalDataShards) {
            // First seen packet was data packet
            startPacket = m_currentFrame.packetCounter - m_currentFrame.fecIndex;
            nextStartPacket = m_currentFrame.packetCounter - m_currentFrame.fecIndex + m_totalShards * m_shardPackets - padding;
        }else{
            // was parity packet
            startPacket = m_currentFrame.packetCounter - (m_currentFrame.fecIndex - padding);
            uint64_t m_startOfParityPacket = m_currentFrame.packetCounter - (m_currentFrame.fecIndex - m_totalDataShards * m_shardPackets);
            nextStartPacket = m_startOfParityPacket + m_totalParityShards * m_shardPackets;
        }
        if(m_firstPacketOfNextFrame != 0 && m_firstPacketOfNextFrame != startPacket) {
            // Whole frame packet loss
            FrameLog(m_currentFrame.trackingFrameIndex,
                     "Previous frame was completely lost. videoFrame=%llu shards=%u:%u frameByteSize=%d fecPercentage=%d m_totalShards=%u "
                     "m_shardPackets=%u m_blockSize=%u m_firstPacketOfNextFrame=%u startPacket=%u currentPacket=%u",
                     m_currentFrame.videoFrameIndex, m_totalDataShards, m_totalParityShards,
                     m_currentFrame.frameByteSize, m_currentFrame.fecPercentage, m_totalShards,
                     m_shardPackets, m_blockSize, m_firstPacketOfNextFrame, startPacket, m_currentFrame.packetCounter);
            for (size_t packet = 0; packet < m_shardPackets; packet++) {
                FrameLog(m_currentFrame.trackingFrameIndex,
                         "packetIndex=%d, shards=%u:%u",
                         packet, m_receivedDataShards[packet], m_receivedParityShards[packet]);
            }
            fecFailure = m_fecFailure = true;
        }
        m_firstPacketOfNextFrame = nextStartPacket;

        FrameLog(m_currentFrame.trackingFrameIndex,
                 "Start new frame. videoFrame=%llu frameByteSize=%d fecPercentage=%d m_totalDataShards=%u m_totalParityShards=%u"
                 " m_totalShards=%u m_shardPackets=%u m_blockSize=%u",
                 m_currentFrame.videoFrameIndex, m_currentFrame.frameByteSize, m_currentFrame.fecPercentage, m_totalDataShards,
                 m_totalParityShards, m_totalShards, m_shardPackets, m_blockSize);
    }
    size_t shardIndex = packet->fecIndex / m_shardPackets;
    size_t packetIndex = packet->fecIndex % m_shardPackets;
    if (m_marks[packetIndex][shardIndex] == 0) {
        // Duplicate packet.
        LOGI("Packet duplication. packetCounter=%d fecIndex=%d", packet->packetCounter,
             packet->fecIndex);
        return;
    }
    m_marks[packetIndex][shardIndex] = 0;
    if (shardIndex < m_totalDataShards) {
        m_receivedDataShards[packetIndex]++;
    } else {
        m_receivedParityShards[packetIndex]++;
    }

    std::byte *p = &m_frameBuffer[packet->fecIndex * ALVR_MAX_VIDEO_BUFFER_SIZE];
    char *payload = ((char *) packet) + sizeof(VideoFrame);
    int payloadSize = packetSize - sizeof(VideoFrame);
    memcpy(p, payload, payloadSize);
    if (payloadSize != ALVR_MAX_VIDEO_BUFFER_SIZE) {
        // Fill padding
        memset(p + payloadSize, 0, ALVR_MAX_VIDEO_BUFFER_SIZE - payloadSize);
    }
}

bool FECQueue::reconstruct() {
    if (m_recovered) {
        return false;
    }

    bool ret = true;
    // On server side, we encoded all buffer in one call of reed_solomon_encode.
    // But client side, we should split shards for more resilient recovery.
    for (size_t packet = 0; packet < m_shardPackets; packet++) {
        if (m_recoveredPacket[packet]) {
            continue;
        }
        if (m_receivedDataShards[packet] == m_totalDataShards) {
            // We've received a full packet with no need for FEC.
            //FrameLog(m_currentFrame.frameIndex, "No need for FEC. packetIndex=%d", packet);
            m_recoveredPacket[packet] = true;
            continue;
        }
        m_rs->shards = m_receivedDataShards[packet] +
                       m_receivedParityShards[packet]; //Don't let RS complain about missing parity packets

        if (m_rs->shards < (int) m_totalDataShards) {
            // Not enough parity data
            ret = false;
            continue;
        }

        FrameLog(m_currentFrame.trackingFrameIndex,
                 "Recovering. packetIndex=%d receivedDataShards=%d/%d receivedParityShards=%d/%d",
                 packet, m_receivedDataShards[packet], m_totalDataShards,
                 m_receivedParityShards[packet], m_totalParityShards);

        for (size_t i = 0; i < m_totalShards; i++) {
            m_shards[i] = &m_frameBuffer[(i * m_shardPackets + packet) * ALVR_MAX_VIDEO_BUFFER_SIZE];
        }

        int result = reed_solomon_reconstruct(m_rs, (unsigned char **) &m_shards[0],
                                              &m_marks[packet][0],
                                              m_totalShards, ALVR_MAX_VIDEO_BUFFER_SIZE);
        m_recoveredPacket[packet] = true;
        // We should always provide enough parity to recover the missing data successfully.
        // If this fails, something is probably wrong with our FEC state.
        if (result != 0) {
            LOGE("reed_solomon_reconstruct failed.");
            return false;
        }
        /*
        for(int i = 0; i < m_totalShards * m_shardPackets; i++) {
            char *p = &frameBuffer[ALVR_MAX_VIDEO_BUFFER_SIZE * i];
            LOGI("Reconstructed packets. i=%d shardIndex=%d buffer=[%02X %02X %02X %02X %02X ...]", i, i / m_shardPackets, p[0], p[1], p[2], p[3], p[4]);
        }*/
    }
    if (ret) {
        m_recovered = true;
        FrameLog(m_currentFrame.trackingFrameIndex, "Frame was successfully recovered by FEC.");
    }
    return ret;
}

const std::byte *FECQueue::getFrameBuffer() {
    return &m_frameBuffer[0];
}

int FECQueue::getFrameByteSize() {
    return m_currentFrame.frameByteSize;
}

bool FECQueue::fecFailure() {
    return m_fecFailure;
}

void FECQueue::clearFecFailure() {
    m_fecFailure = false;
}
