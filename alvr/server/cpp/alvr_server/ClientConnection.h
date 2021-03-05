#pragma once

#include <iostream>
#include <string>
#include <sstream>
#include <vector>
#include <algorithm>
#include "threadtools.h"
#include "Logger.h"
#include "Utils.h"
#include "packet_types.h"
#include "Settings.h"
#include "Statistics.h"

extern "C" {
#include "reedsolomon/rs.h"
};

class ClientConnection {
public:

	ClientConnection(std::function<void()> poseUpdatedCallback, std::function<void()> packetLossCallback);
	~ClientConnection();

	void FECSend(uint8_t *buf, int len, uint64_t frameIndex, uint64_t videoFrameIndex);
	void SendVideo(uint8_t *buf, int len, uint64_t frameIndex);
	void SendAudio(uint8_t *buf, int len, uint64_t presentationTime);
	void SendHapticsFeedback(uint64_t startTime, float amplitude, float duration, float frequency, uint8_t hand);
	void ProcessRecv(unsigned char *buf, int len);
	bool HasValidTrackingInfo() const;
	void GetTrackingInfo(TrackingInfo &info);
	uint64_t clientToServerTime(uint64_t clientTime) const;
	uint64_t serverToClientTime(uint64_t serverTime) const;
	void OnFecFailure();
	std::shared_ptr<Statistics> GetStatistics();
private:
	bool m_bExiting;
	std::shared_ptr<Statistics> m_Statistics;

	std::ofstream outfile;

	// Maximum UDP payload
	static const int PACKET_SIZE = 1400;
	static const int64_t REQUEST_TIMEOUT = 5 * 1000 * 1000;
	static const int64_t CONNECTION_TIMEOUT = 5 * 1000 * 1000;
	static const int64_t STATISTICS_TIMEOUT_US = 1000 * 1000;

	uint32_t videoPacketCounter = 0;
	uint32_t soundPacketCounter = 0;

	std::function<void()> m_PoseUpdatedCallback;
	std::function<void()> m_PacketLossCallback;
	TrackingInfo m_TrackingInfo;

	uint64_t m_TimeDiff = 0;
	CRITICAL_SECTION m_CS;

	TimeSync m_reportedStatistics;
	uint64_t m_lastFecFailure = 0;
	static const uint64_t CONTINUOUS_FEC_FAILURE = 60 * 1000 * 1000;
	static const int INITIAL_FEC_PERCENTAGE = 5;
	static const int MAX_FEC_PERCENTAGE = 10;
	int m_fecPercentage = INITIAL_FEC_PERCENTAGE;

	uint64_t mVideoFrameIndex = 1;

	uint64_t m_LastStatisticsUpdate;
};
