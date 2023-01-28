#include "ClientConnection.h"
#include <mutex>
#include <string.h>

#include "Statistics.h"
#include "Logger.h"
#include "bindings.h"
#include "Utils.h"
#include "Settings.h"

static const char NAL_HEADER[] = {0x00, 0x00, 0x00, 0x01};

static const uint8_t H264_NAL_TYPE_SPS = 7;
static const uint8_t H265_NAL_TYPE_VPS = 32;

static const uint8_t H264_NAL_TYPE_AUD = 9;
static const uint8_t H265_NAL_TYPE_AUD = 35;

ClientConnection::ClientConnection() { 
	m_Statistics = std::make_shared<Statistics>(); 
}

/*
	Sends the (VPS + )SPS + PPS video configuration headers from H.264 or H.265 stream as a sequence of NALs.
	(VPS + )SPS + PPS have short size (8bytes + 28bytes in some environment), so we can
	assume SPS + PPS is contained in first fragment.
*/
void sendHeaders(uint8_t **buf, int *len, int nalNum) {
	uint8_t *b = *buf;
	uint8_t *end = b + *len;

	int headersLen = 0;
	int foundHeaders = -1; // Offset by 1 header to find the length until the next header
	while (b != end) {
		if (b + sizeof(NAL_HEADER) <= end && memcmp(b, NAL_HEADER, sizeof(NAL_HEADER)) == 0) {
			foundHeaders++;
			if (foundHeaders == nalNum) {
				break;
			}
			b += sizeof(NAL_HEADER);
			headersLen += sizeof(NAL_HEADER);
		}

		b++;
		headersLen++;
	}
	if (foundHeaders != nalNum) {
		return;
	}
	InitializeDecoder((const unsigned char *)*buf, headersLen);

	// move the cursor forward excluding config NALs
	*buf = b;
	*len -= headersLen;
}

void processH264Nals(uint8_t **buf, int *len) {
	uint8_t *b = *buf;
	int l = *len;
	uint8_t nalType = b[4] & 0x1F;

	if (nalType == H264_NAL_TYPE_AUD && l > sizeof(NAL_HEADER) * 2 + 2) {
		b += sizeof(NAL_HEADER) + 2;
		l -= sizeof(NAL_HEADER) + 2;
		nalType = b[4] & 0x1F;
	}
	if (nalType == H264_NAL_TYPE_SPS) {
		sendHeaders(&b, &l, 2); // 2 headers SPS and PPS
	}
	*buf = b;
	*len = l;
}

void processH265Nals(uint8_t **buf, int *len) {
	uint8_t *b = *buf;
	int l = *len;
	uint8_t nalType = (b[4] >> 1) & 0x3F;

	if (nalType == H265_NAL_TYPE_AUD && l > sizeof(NAL_HEADER) * 2 + 3) {
		b += sizeof(NAL_HEADER) + 3;
		l -= sizeof(NAL_HEADER) + 3;
		nalType = (b[4] >> 1) & 0x3F;
	}
	if (nalType == H265_NAL_TYPE_VPS) {
		sendHeaders(&b, &l, 3); // 3 headers VPS, SPS and PPS
	}
	*buf = b;
	*len = l;
}

void ClientConnection::SendVideo(uint8_t *buf, int len, uint64_t targetTimestampNs) {
	// Report before the frame is packetized
	ReportEncoded(targetTimestampNs);

	if (len < sizeof(NAL_HEADER)) {
		return;
	}

	int codec = Settings::Instance().m_codec;
	if (codec == ALVR_CODEC_H264) {
		processH264Nals(&buf, &len);
	} else if (codec == ALVR_CODEC_H265) {
		processH265Nals(&buf, &len);
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
