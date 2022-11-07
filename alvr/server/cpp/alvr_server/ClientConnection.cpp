#include "ClientConnection.h"
#include <mutex>
#include <string.h>

#include "Statistics.h"
#include "Logger.h"
#include "bindings.h"
#include "Utils.h"
#include "Settings.h"

static const uint8_t NAL_TYPE_SPS = 7;
static const uint8_t H265_NAL_TYPE_VPS = 32;

ClientConnection::ClientConnection() {

	m_Statistics = std::make_shared<Statistics>();

	reed_solomon_init();
	
	videoPacketCounter = 0;
	m_fecPercentage = INITIAL_FEC_PERCENTAGE;
}

int findVPSSPS(const uint8_t *frameBuffer, int frameByteSize) {
    int zeroes = 0;
    int foundNals = 0;
    for (int i = 0; i < frameByteSize; i++) {
        if (frameBuffer[i] == 0) {
            zeroes++;
        } else if (frameBuffer[i] == 1) {
            if (zeroes >= 2) {
                foundNals++;
                if (Settings::Instance().m_codec == ALVR_CODEC_H264 && foundNals >= 3) {
                    // Find end of SPS+PPS on H.264.
                    return i - 3;
                } else if (Settings::Instance().m_codec == ALVR_CODEC_H265 && foundNals >= 4) {
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

void ClientConnection::FECSend(uint8_t *buf, int len, uint64_t targetTimestampNs, uint64_t videoFrameIndex) {
	int shardPackets = CalculateFECShardPackets(len, m_fecPercentage);

	int blockSize = shardPackets * ALVR_MAX_VIDEO_BUFFER_SIZE;

	int dataShards = (len + blockSize - 1) / blockSize;
	int totalParityShards = CalculateParityShards(dataShards, m_fecPercentage);
	int totalShards = dataShards + totalParityShards;

	assert(totalShards <= DATA_SHARDS_MAX);

	reed_solomon *rs = reed_solomon_new(dataShards, totalParityShards);

	std::vector<uint8_t *> shards(totalShards);

	for (int i = 0; i < dataShards; i++) {
		shards[i] = buf + i * blockSize;
	}
	if (len % blockSize != 0) {
		// Padding
		shards[dataShards - 1] = new uint8_t[blockSize];
		memset(shards[dataShards - 1], 0, blockSize);
		memcpy(shards[dataShards - 1], buf + (dataShards - 1) * blockSize, len % blockSize);
	}
	for (int i = 0; i < totalParityShards; i++) {
		shards[dataShards + i] = new uint8_t[blockSize];
	}

	int ret = reed_solomon_encode(rs, &shards[0], totalShards, blockSize);
	assert(ret == 0);

	reed_solomon_release(rs);

	uint8_t packetBuffer[2000];
	VideoFrame *header = (VideoFrame *)packetBuffer;
	uint8_t *payload = packetBuffer + sizeof(VideoFrame);
	int dataRemain = len;

	header->trackingFrameIndex = targetTimestampNs;
	header->videoFrameIndex = videoFrameIndex;
	header->sentTime = GetTimestampUs();
	header->frameByteSize = len;
	header->fecIndex = 0;
	header->fecPercentage = (uint16_t)m_fecPercentage;
	for (int i = 0; i < dataShards; i++) {
		for (int j = 0; j < shardPackets; j++) {
			int copyLength = std::min(ALVR_MAX_VIDEO_BUFFER_SIZE, dataRemain);
			if (copyLength <= 0) {
				break;
			}
			memcpy(payload, shards[i] + j * ALVR_MAX_VIDEO_BUFFER_SIZE, copyLength);
			dataRemain -= ALVR_MAX_VIDEO_BUFFER_SIZE;

			header->packetCounter = videoPacketCounter;
			videoPacketCounter++;
			VideoSend(*header, (unsigned char *)packetBuffer + sizeof(VideoFrame), copyLength);
			m_Statistics->CountPacket(sizeof(VideoFrame) + copyLength);
			header->fecIndex++;
		}
	}
	header->fecIndex = dataShards * shardPackets;
	for (int i = 0; i < totalParityShards; i++) {
		for (int j = 0; j < shardPackets; j++) {
			int copyLength = ALVR_MAX_VIDEO_BUFFER_SIZE;
			memcpy(payload, shards[dataShards + i] + j * ALVR_MAX_VIDEO_BUFFER_SIZE, copyLength);

			header->packetCounter = videoPacketCounter;
			videoPacketCounter++;
			
			VideoSend(*header, (unsigned char *)packetBuffer + sizeof(VideoFrame), copyLength);
			m_Statistics->CountPacket(sizeof(VideoFrame) + copyLength);
			header->fecIndex++;
		}
	}

	if (len % blockSize != 0) {
		delete[] shards[dataShards - 1];
	}
	for (int i = 0; i < totalParityShards; i++) {
		delete[] shards[dataShards + i];
	}
}

void ClientConnection::SendVideo(uint8_t *buf, int len, uint64_t targetTimestampNs) {
	// Report before the frame is packetized
	ReportEncoded(targetTimestampNs);

	uint8_t NALType;
	if (Settings::Instance().m_codec == ALVR_CODEC_H264)
		NALType = buf[4] & 0x1F;
	else
		NALType = (buf[4] >> 1) & 0x3F;

	if ((Settings::Instance().m_codec == ALVR_CODEC_H264 && NALType == NAL_TYPE_SPS) ||
		(Settings::Instance().m_codec == ALVR_CODEC_H265 && NALType == H265_NAL_TYPE_VPS)) {
		// This frame contains (VPS + )SPS + PPS + IDR on NVENC H.264 (H.265) stream.
		// (VPS + )SPS + PPS has short size (8bytes + 28bytes in some environment), so we can
		// assume SPS + PPS is contained in first fragment.

		int end = findVPSSPS(buf, len);
		if (end == -1) {
			// Invalid frame.
			return;
		}

		InitializeDecoder((const unsigned char *)buf, end);

		// move the cursor forward excluding config NALs
		buf = &buf[end];
		len = len - end;
	}

	if (Settings::Instance().m_enableFec) {
		FECSend(buf, len, targetTimestampNs, mVideoFrameIndex);
	} else {
		VideoFrame header = {};
		header.packetCounter = this->videoPacketCounter;
		header.trackingFrameIndex = targetTimestampNs;
		header.videoFrameIndex = mVideoFrameIndex;
		header.sentTime = GetTimestampUs();
		header.frameByteSize = len;

		VideoSend(header, buf, len);

		m_Statistics->CountPacket(sizeof(VideoFrame) + len);

		this->videoPacketCounter++;
	}

	mVideoFrameIndex++;
}

void ClientConnection::ReportNetworkLatency(uint64_t latencyUs) {
	m_Statistics->NetworkSend(latencyUs);
}

void ClientConnection::OnFecFailure() {
	Debug("Listener::OnFecFailure()\n");
	if (GetTimestampUs() - m_lastFecFailure < CONTINUOUS_FEC_FAILURE) {
		if (m_fecPercentage < MAX_FEC_PERCENTAGE) {
			m_fecPercentage += 5;
		}
	}
	m_lastFecFailure = GetTimestampUs();

	ReportFecFailure(m_fecPercentage);
}

std::shared_ptr<Statistics> ClientConnection::GetStatistics() {
	return m_Statistics;
}
