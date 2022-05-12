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

	void FECSend(uint8_t *buf, int len, uint64_t targetTimestampNs, uint64_t videoFrameIndex);
	void SendVideo(uint8_t *buf, int len, uint64_t targetTimestampNs);
 	void ReportNetworkLatency(uint64_t latencyUs);
	void OnFecFailure();
	std::shared_ptr<Statistics> GetStatistics();

	std::shared_ptr<Statistics> m_Statistics;

	uint32_t videoPacketCounter = 0;

	uint64_t m_lastFecFailure = 0;
	static const uint64_t CONTINUOUS_FEC_FAILURE = 60 * 1000 * 1000;
	static const int INITIAL_FEC_PERCENTAGE = 5;
	static const int MAX_FEC_PERCENTAGE = 10;
	int m_fecPercentage = INITIAL_FEC_PERCENTAGE;

	uint64_t mVideoFrameIndex = 1;
};
