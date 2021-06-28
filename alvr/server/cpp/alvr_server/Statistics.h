#pragma once

#include <algorithm>
#include <stdint.h>
#include <time.h>

#include "Utils.h"
#include "Settings.h"

class Statistics {
public:
	Statistics() {
		ResetAll();
		m_current = time(NULL);
	}

	void ResetAll() {
		m_packetsSentTotal = 0;
		m_packetsSentInSecond = 0;
		m_packetsSentInSecondPrev = 0;
		m_bitsSentTotal = 0;
		m_bitsSentInSecond = 0;
		m_bitsSentInSecondPrev = 0;

		m_framesInSecond = 0;
		m_framesPrevious = 0;

		m_encodeLatencyTotalUs = 0;
		m_encodeLatencyMin = 0;
		m_encodeLatencyMax = 0;
		m_encodeSampleCount = 0;
		m_encodeLatencyAveragePrev = 0;
		m_encodeLatencyMinPrev = 0;
		m_encodeLatencyMaxPrev = 0;

		m_sendLatency = 0;
	}

	void CountPacket(int bytes) {
		CheckAndResetSecond();

		m_packetsSentTotal++;
		m_packetsSentInSecond++;
		m_bitsSentTotal += bytes * 8;
		m_bitsSentInSecond += bytes * 8;
	}

	void EncodeOutput(uint64_t latencyUs) {
		CheckAndResetSecond();

		m_framesInSecond++;
		m_encodeLatencyAveragePrev = latencyUs;
		m_encodeLatencyTotalUs += latencyUs;
		m_encodeLatencyMin = std::min(latencyUs, m_encodeLatencyMin);
		m_encodeLatencyMax = std::max(latencyUs, m_encodeLatencyMax);
		m_encodeSampleCount++;
	}

	void NetworkSend(uint64_t latencyUs) {
		if (latencyUs > 5e5)
			latencyUs = 5e5;
		if (m_sendLatency == 0) {
			m_sendLatency = latencyUs;
		} else {
			m_sendLatency = latencyUs * 0.1 + m_sendLatency * 0.9;
		}
	}

	uint64_t GetPacketsSentTotal() {
		return m_packetsSentTotal;
	}
	uint64_t GetPacketsSentInSecond() {
		return m_packetsSentInSecondPrev;
	}
	uint64_t GetBitrate() {
		return m_bitrate;
	}
	uint64_t GetBitsSentTotal() {
		return m_bitsSentTotal;
	}
	uint64_t GetBitsSentInSecond() {
		return m_bitsSentInSecondPrev;
	}
	float GetFPS() {
		return m_framesPrevious;
	}
	uint64_t GetEncodeLatencyAverage() {
		return m_encodeLatencyAveragePrev;
	}
	uint64_t GetSendLatencyAverage() {
		return m_sendLatency;
	}

	bool CheckBitrateUpdated() {
		if (m_enableAdaptiveBitrate) {
			uint64_t latencyUs = m_sendLatency;
			if (latencyUs != 0) {
				if (latencyUs > m_adaptiveBitrateTarget + m_adaptiveBitrateThreshold) {
					m_bitrate -= 3;
				} else if (latencyUs < m_adaptiveBitrateTarget - m_adaptiveBitrateThreshold) {
					m_bitrate += 1;
				}
				if (m_bitrate > m_adaptiveBitrateMaximum) {
					m_bitrate = m_adaptiveBitrateMaximum;
				} else if (m_bitrate < 5) {
					m_bitrate = 5;
				}
			}
			if (m_bitrateUpdated != m_bitrate) {
				m_bitrateUpdated = m_bitrate;
				return true;
			}
		}
		return false;
	}
private:
	void ResetSecond() {
		m_packetsSentInSecondPrev = m_packetsSentInSecond;
		m_bitsSentInSecondPrev = m_bitsSentInSecond;
		m_packetsSentInSecond = 0;
		m_bitsSentInSecond = 0;

		m_framesPrevious = m_framesInSecond;
		m_framesInSecond = 0;

		m_encodeLatencyMinPrev = m_encodeLatencyMin;
		m_encodeLatencyMaxPrev = m_encodeLatencyMax;
		m_encodeLatencyTotalUs = 0;
		m_encodeSampleCount = 0;
		m_encodeLatencyMin = UINT64_MAX;
		m_encodeLatencyMax = 0;
	}

	void CheckAndResetSecond() {
		time_t current = time(NULL);
		if (m_current != current) {
			m_current = current;
			ResetSecond();
		}
	}

	uint64_t m_packetsSentTotal;
	uint64_t m_packetsSentInSecond;
	uint64_t m_packetsSentInSecondPrev;

	uint64_t m_bitsSentTotal;
	uint64_t m_bitsSentInSecond;
	uint64_t m_bitsSentInSecondPrev;

	uint32_t m_framesInSecond;
	uint32_t m_framesPrevious;

	uint64_t m_encodeLatencyTotalUs;
	uint64_t m_encodeLatencyMin;
	uint64_t m_encodeLatencyMax;
	uint64_t m_encodeSampleCount;
	uint64_t m_encodeLatencyAveragePrev = 0;
	uint64_t m_encodeLatencyMinPrev;
	uint64_t m_encodeLatencyMaxPrev;
	
	uint64_t m_sendLatency = 0;

	uint64_t m_bitrate = Settings::Instance().mEncodeBitrateMBs;
	uint64_t m_bitrateUpdated = Settings::Instance().mEncodeBitrateMBs;

	bool m_enableAdaptiveBitrate = Settings::Instance().m_enableAdaptiveBitrate;
	uint64_t m_adaptiveBitrateMaximum = Settings::Instance().m_adaptiveBitrateMaximum;
	uint64_t m_adaptiveBitrateTarget = Settings::Instance().m_adaptiveBitrateTarget;
	uint64_t m_adaptiveBitrateThreshold = Settings::Instance().m_adaptiveBitrateThreshold;

	time_t m_current;
};