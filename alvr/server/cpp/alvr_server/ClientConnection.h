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

	ClientConnection(std::function<void()> poseUpdatedCallback, std::function<void()> packetLossCallback);

	void FECSend(uint8_t *buf, int len, uint64_t frameIndex, uint64_t videoFrameIndex);
	void SendVideo(uint8_t *buf, int len, uint64_t frameIndex);
	void ProcessTrackingInfo(TrackingInfo data);
 	void ProcessTimeSync(TimeSync data);
	bool HasValidTrackingInfo() const;
	void GetTrackingInfo(TrackingInfo &info);
	float GetPoseTimeOffset();
	void OnFecFailure();
	std::shared_ptr<Statistics> GetStatistics();
private:
	std::shared_ptr<Statistics> m_Statistics;

	uint32_t videoPacketCounter = 0;

	std::function<void()> m_PoseUpdatedCallback;
	std::function<void()> m_PacketLossCallback;
	TrackingInfo m_TrackingInfo;

	uint64_t m_RTT = 0;
	int64_t m_TimeDiff = 0;
	std::mutex m_CS;

	TimeSync m_reportedStatistics;
	uint64_t m_lastFecFailure = 0;
	static const uint64_t CONTINUOUS_FEC_FAILURE = 60 * 1000 * 1000;
	static const int INITIAL_FEC_PERCENTAGE = 5;
	static const int MAX_FEC_PERCENTAGE = 10;
	int m_fecPercentage = INITIAL_FEC_PERCENTAGE;

	uint64_t mVideoFrameIndex = 1;

	uint64_t m_LastStatisticsUpdate;
};
