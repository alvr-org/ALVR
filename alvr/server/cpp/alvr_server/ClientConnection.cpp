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

	VideoSend(targetTimestampNs, buf, len);
	m_Statistics->CountPacket(len);
}

void ClientConnection::ReportNetworkLatency(uint64_t latencyUs) {
	m_Statistics->NetworkSend(latencyUs);
}

std::shared_ptr<Statistics> ClientConnection::GetStatistics() {
	return m_Statistics;
}
