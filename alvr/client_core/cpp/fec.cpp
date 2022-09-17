
#include "fec.h"
#include <assert.h>
#include <cstring>

static const int ALVR_MAX_VIDEO_BUFFER_SIZE = 1400;
static const int ALVR_FEC_SHARDS_MAX = 20;

inline int CalculateParityShards(int dataShards, int fecPercentage) {
    int totalParityShards = (dataShards * fecPercentage + 99) / 100;
    return totalParityShards;
}

// Calculate how many packet is needed for make signal shard.
inline int CalculateFECShardPackets(int len, int fecPercentage) {
    // This reed solomon implementation accept only 255 shards.
    // Normally, we use ALVR_MAX_VIDEO_BUFFER_SIZE as block_size and single packet becomes single
    // shard. If we need more than maxDataShards packets, we need to combine multiple packet to make
    // single shrad. NOTE: Moonlight seems to use only 255 shards for video frame.
    int maxDataShards =
        ((ALVR_FEC_SHARDS_MAX - 2) * 100 + 99 + fecPercentage) / (100 + fecPercentage);
    int minBlockSize = (len + maxDataShards - 1) / maxDataShards;
    int shardPackets = (minBlockSize + ALVR_MAX_VIDEO_BUFFER_SIZE - 1) / ALVR_MAX_VIDEO_BUFFER_SIZE;
    assert(maxDataShards + CalculateParityShards(maxDataShards, fecPercentage) <=
           ALVR_FEC_SHARDS_MAX);
    return shardPackets;
}

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
void FECQueue::addVideoPacket(VideoFrame header,
                              const unsigned char *payload,
                              int payloadSize,
                              bool &fecFailure) {
    if (m_recovered && m_currentFrame.videoFrameIndex == header.videoFrameIndex) {
        return;
    }
    if (m_currentFrame.videoFrameIndex != header.videoFrameIndex) {
        // New frame
        if (!m_recovered) {
            fecFailure = m_fecFailure = true;
        }
        m_currentFrame = header;
        m_recovered = false;
        if (m_rs != NULL) {
            reed_solomon_release(m_rs);
        }

        uint32_t fecDataPackets =
            (header.frameByteSize + ALVR_MAX_VIDEO_BUFFER_SIZE - 1) / ALVR_MAX_VIDEO_BUFFER_SIZE;
        m_shardPackets =
            CalculateFECShardPackets(m_currentFrame.frameByteSize, m_currentFrame.fecPercentage);
        m_blockSize = m_shardPackets * ALVR_MAX_VIDEO_BUFFER_SIZE;

        m_totalDataShards = (m_currentFrame.frameByteSize + m_blockSize - 1) / m_blockSize;
        m_totalParityShards =
            CalculateParityShards(m_totalDataShards, m_currentFrame.fecPercentage);
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
        if (m_currentFrame.fecIndex / m_shardPackets < m_totalDataShards) {
            // First seen packet was data packet
            startPacket = m_currentFrame.packetCounter - m_currentFrame.fecIndex;
            nextStartPacket = m_currentFrame.packetCounter - m_currentFrame.fecIndex +
                              m_totalShards * m_shardPackets - padding;
        } else {
            // was parity packet
            startPacket = m_currentFrame.packetCounter - (m_currentFrame.fecIndex - padding);
            uint64_t m_startOfParityPacket =
                m_currentFrame.packetCounter -
                (m_currentFrame.fecIndex - m_totalDataShards * m_shardPackets);
            nextStartPacket = m_startOfParityPacket + m_totalParityShards * m_shardPackets;
        }
        if (m_firstPacketOfNextFrame != 0 && m_firstPacketOfNextFrame != startPacket) {
            // Whole frame packet loss
            fecFailure = m_fecFailure = true;
        }
        m_firstPacketOfNextFrame = nextStartPacket;
    }
    size_t shardIndex = header.fecIndex / m_shardPackets;
    size_t packetIndex = header.fecIndex % m_shardPackets;
    if (m_marks[packetIndex][shardIndex] == 0) {
        // Duplicate packet.
        return;
    }
    m_marks[packetIndex][shardIndex] = 0;
    if (shardIndex < m_totalDataShards) {
        m_receivedDataShards[packetIndex]++;
    } else {
        m_receivedParityShards[packetIndex]++;
    }

    std::byte *p = &m_frameBuffer[header.fecIndex * ALVR_MAX_VIDEO_BUFFER_SIZE];
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
            m_recoveredPacket[packet] = true;
            continue;
        }
        m_rs->shards =
            m_receivedDataShards[packet] +
            m_receivedParityShards[packet]; // Don't let RS complain about missing parity packets

        if (m_rs->shards < (int)m_totalDataShards) {
            // Not enough parity data
            ret = false;
            continue;
        }

        for (size_t i = 0; i < m_totalShards; i++) {
            m_shards[i] =
                &m_frameBuffer[(i * m_shardPackets + packet) * ALVR_MAX_VIDEO_BUFFER_SIZE];
        }

        int result = reed_solomon_reconstruct(m_rs,
                                              (unsigned char **)&m_shards[0],
                                              &m_marks[packet][0],
                                              m_totalShards,
                                              ALVR_MAX_VIDEO_BUFFER_SIZE);
        m_recoveredPacket[packet] = true;
        // We should always provide enough parity to recover the missing data successfully.
        // If this fails, something is probably wrong with our FEC state.
        if (result != 0) {
            return false;
        }
    }
    if (ret) {
        m_recovered = true;
    }
    return ret;
}

const std::byte *FECQueue::getFrameBuffer() { return &m_frameBuffer[0]; }

int FECQueue::getFrameByteSize() { return m_currentFrame.frameByteSize; }

void FECQueue::clearFecFailure() { m_fecFailure = false; }
