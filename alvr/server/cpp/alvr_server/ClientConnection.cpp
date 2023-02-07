#include "ClientConnection.h"
#include <mutex>
#include <string.h>

#include "Statistics.h"
#include "Logger.h"
#include "bindings.h"
#include "Utils.h"
#include "Settings.h"

static const char NAL_PREFIX_3B[] = {0x00, 0x00, 0x01};
static const char NAL_PREFIX_4B[] = {0x00, 0x00, 0x00, 0x01};

static const uint8_t H264_NAL_TYPE_SPS = 7;
static const uint8_t H265_NAL_TYPE_VPS = 32;

static const uint8_t H264_NAL_TYPE_AUD = 9;
static const uint8_t H265_NAL_TYPE_AUD = 35;

ClientConnection::ClientConnection() { 
	m_Statistics = std::make_shared<Statistics>(); 
}

int8_t getNalPrefixSize(uint8_t *buf) {
	if (memcmp(buf, NAL_PREFIX_3B, sizeof(NAL_PREFIX_3B)) == 0) {
		return sizeof(NAL_PREFIX_3B);
	} else if (memcmp(buf, NAL_PREFIX_4B, sizeof(NAL_PREFIX_4B)) == 0) {
		return sizeof(NAL_PREFIX_4B);
	} else {
		return -1;
	}
}

/*
	Sends the (VPS + )SPS + PPS video configuration headers from H.264 or H.265 stream as a sequence of NALs.
	(VPS + )SPS + PPS have short size (8bytes + 28bytes in some environment), so we can
	assume SPS + PPS is contained in first fragment.
*/
void sendHeaders(uint8_t *&buf, int &len, int nalNum) {
	uint8_t *cursor = buf;
	int headersLen = 0;
	int foundHeaders = -1; // Offset by 1 header to find the length until the next header

	while (headersLen <= len) {
		if (headersLen + sizeof(NAL_PREFIX_4B) > len) {
			cursor++;
			headersLen++;
			continue;
		}
		int8_t prefixSize = getNalPrefixSize(cursor);
		if (prefixSize == -1) {
			cursor++;
			headersLen++;
			continue;
		}
		foundHeaders++;
		if (foundHeaders == nalNum) {
			break;
		}
		headersLen += prefixSize;
		cursor += prefixSize;
	}

	if (foundHeaders != nalNum) {
		return;
	}
	InitializeDecoder((const unsigned char *)buf, headersLen);

	// move the cursor forward excluding config NALs
	buf = cursor;
	len -= headersLen;
}

void processH264Nals(uint8_t *&buf, int &len) {
	uint8_t prefixSize = getNalPrefixSize(buf);
	uint8_t nalType = buf[prefixSize] & 0x1F;

	if (nalType == H264_NAL_TYPE_AUD && len > prefixSize * 2 + 2) {
		buf += prefixSize + 2;
		len -= prefixSize + 2;
		prefixSize = getNalPrefixSize(buf);
		nalType = buf[prefixSize] & 0x1F;
	}
	if (nalType == H264_NAL_TYPE_SPS) {
		sendHeaders(buf, len, 2); // 2 headers SPS and PPS
	}
}

void processH265Nals(uint8_t *&buf, int &len) {
	uint8_t prefixSize = getNalPrefixSize(buf);
	uint8_t nalType = (buf[prefixSize] >> 1) & 0x3F;

	if (nalType == H265_NAL_TYPE_AUD && len > prefixSize * 2 + 3) {
		buf += prefixSize + 3;
		len -= prefixSize + 3;
		prefixSize = getNalPrefixSize(buf);
		nalType = (buf[prefixSize] >> 1) & 0x3F;
	}
	if (nalType == H265_NAL_TYPE_VPS) {
		sendHeaders(buf, len, 3); // 3 headers VPS, SPS and PPS
	}
}

void ClientConnection::SendVideo(uint8_t *buf, int len, uint64_t targetTimestampNs) {
	// Report before the frame is packetized
	ReportEncoded(targetTimestampNs);

	if (len < sizeof(NAL_PREFIX_4B)) {
		return;
	}

	int codec = Settings::Instance().m_codec;
	if (codec == ALVR_CODEC_H264) {
		processH264Nals(buf, len);
	} else if (codec == ALVR_CODEC_H265) {
		processH265Nals(buf, len);
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
