#pragma once

#include <functional>
#include <memory>
#include <fstream>
#include <mutex>

#include "ALVR-common/packet_types.h"
#include "Settings.h"

#include "openvr_driver.h"

class Statistics;

class ClientConnection {
public:

	ClientConnection();

	void Send(uint8_t *buf, int len, uint64_t targetTimestampNs, uint64_t videoFrameIndex);
	void SendVideo(uint8_t *buf, int len, uint64_t targetTimestampNs);
 	void ReportNetworkLatency(uint64_t latencyUs);
	void OnPacketLoss();
	std::shared_ptr<Statistics> GetStatistics();

	std::shared_ptr<Statistics> m_Statistics;

	int m_maxPayloadSize;

	uint64_t mVideoFrameIndex = 1;
};
