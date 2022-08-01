#pragma once

#include <algorithm>
#include <stdint.h>
#include <time.h>

#include "Utils.h"
#include "Settings.h"

#define BITS_IN_MBIT 1000000
#define US_IN_S 1000000

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
		m_bitrateSent = 0;

		m_framesInSecond = 0;
		m_framesPrevious = 0;

		m_totalLatency = 0;

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

	void NetworkTotal(uint64_t latencyUs) {
		if (latencyUs > 5e5) // limit to 0.5s
			latencyUs = 5e5;
		if (m_totalLatency == 0) {
			m_totalLatency = latencyUs;
		} else {
			m_totalLatency = latencyUs * 0.05 + m_totalLatency * 0.95;
		}
	}

	void NetworkSend(uint64_t latencyUs) {
		if (latencyUs > 5e5 || latencyUs == 0) // remove invalid latency, limit to 0.5s
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
	uint64_t GetTotalLatencyAverage() {
		return m_totalLatency;
	}
	uint64_t GetEncodeLatencyAverage() {
		return m_encodeLatencyAveragePrev;
	}
	uint64_t GetSendLatencyAverage() {
		return m_sendLatency;
	}

	bool CheckBitrateUpdated() {
		if (m_enableAdaptiveBitrate) {
			uint64_t latencyUs = m_sendLatency; // using video stream transport latency
			if (latencyUs != 0) { // check valid latency
				if (latencyUs > m_adaptiveBitrateTarget + m_adaptiveBitrateThreshold) {
					if (m_bitrate <= 5 + m_adaptiveBitrateDownRate)
						m_bitrate = 5; // minimum bitrate 5mbps
					else
						m_bitrate -= m_adaptiveBitrateDownRate;
				} else if (latencyUs < m_adaptiveBitrateTarget - m_adaptiveBitrateThreshold) {
					if (m_bitrate >= m_adaptiveBitrateMaximum - m_adaptiveBitrateUpRate)
						m_bitrate = m_adaptiveBitrateMaximum; // maximum bitrate
					else if (m_bitrateSent > m_bitrate * m_adaptiveBitrateLightLoadThreshold * (m_framesPrevious == 0 ? m_refreshRate : m_framesPrevious) / m_refreshRate)
						m_bitrate += m_adaptiveBitrateUpRate; // increase bitrate if sent mbps is higher than set bitrate threshold (set bitrate * load threshold * valid framerate)
				}
			}
			if (m_bitrateUpdated != m_bitrate) { // bitrate changed
				m_bitrateUpdated = m_bitrate;
				return true;
			}
		}
		return false;
	}

	void Reset() {
		for(int i = 0; i < 6; i++) {
			m_statistics[i] = 0;
		}
		m_statisticsCount = 0;
	}
	void Add(float total, float encode, float send, float decode, float fps, float ping) {
		m_statistics[0] += total;
		m_statistics[1] += encode;
		m_statistics[2] += send;
		m_statistics[3] += decode;
		m_statistics[4] += fps;
		m_statistics[5] += ping;
		m_statisticsCount++;
	}
	float Get(uint32_t i) {
		return (m_statistics[i] / m_statisticsCount);
	}

	float m_hmdBattery;
	bool m_hmdPlugged;
	float m_leftControllerBattery;
	float m_rightControllerBattery;

private:
	void ResetSecond() {
		m_packetsSentInSecondPrev = m_packetsSentInSecond;
		m_bitsSentInSecondPrev = m_bitsSentInSecond;
		m_bitrateSent = m_bitsSentInSecondPrev / BITS_IN_MBIT;
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

		if (m_adaptiveBitrateUseFrametime) {
			if (m_framesPrevious > 0) {
				m_adaptiveBitrateTarget = US_IN_S / m_framesPrevious + m_adaptiveBitrateTargetOffset; // fps to frametime (us) + offset
			}
			if (m_adaptiveBitrateTarget > m_adaptiveBitrateTargetMaximum) {
				m_adaptiveBitrateTarget = m_adaptiveBitrateTargetMaximum;
			}
		}
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

	// bit/s
	uint64_t m_bitsSentTotal;
	uint64_t m_bitsSentInSecond;
	uint64_t m_bitsSentInSecondPrev;
	// mbit/s
	uint64_t m_bitrateSent;

	uint32_t m_framesInSecond;
	uint32_t m_framesPrevious;

	uint64_t m_totalLatency = 0;

	uint64_t m_encodeLatencyTotalUs;
	uint64_t m_encodeLatencyMin;
	uint64_t m_encodeLatencyMax;
	uint64_t m_encodeSampleCount;
	uint64_t m_encodeLatencyAveragePrev = 0;
	uint64_t m_encodeLatencyMinPrev;
	uint64_t m_encodeLatencyMaxPrev;

	uint64_t m_sendLatency = 0;

	// mbit/s
	uint64_t m_bitrate = Settings::Instance().mEncodeBitrateMBs;
	uint64_t m_bitrateUpdated = Settings::Instance().mEncodeBitrateMBs;

	int64_t m_refreshRate = Settings::Instance().m_refreshRate;

	// mbit/s
	bool m_enableAdaptiveBitrate = Settings::Instance().m_enableAdaptiveBitrate;
	uint64_t m_adaptiveBitrateMaximum = Settings::Instance().m_adaptiveBitrateMaximum;
	uint64_t m_adaptiveBitrateUpRate = Settings::Instance().m_adaptiveBitrateUpRate;
	uint64_t m_adaptiveBitrateDownRate = Settings::Instance().m_adaptiveBitrateDownRate;
	// us
	bool m_adaptiveBitrateUseFrametime = Settings::Instance().m_adaptiveBitrateUseFrametime;
	uint64_t m_adaptiveBitrateTarget = Settings::Instance().m_adaptiveBitrateTarget;
	uint64_t m_adaptiveBitrateTargetMaximum = Settings::Instance().m_adaptiveBitrateTargetMaximum;
	int32_t m_adaptiveBitrateTargetOffset = Settings::Instance().m_adaptiveBitrateTargetOffset;
	uint64_t m_adaptiveBitrateThreshold = Settings::Instance().m_adaptiveBitrateThreshold;
	
	float m_adaptiveBitrateLightLoadThreshold = Settings::Instance().m_adaptiveBitrateLightLoadThreshold;

	time_t m_current;

	// Total/Encode/Send/Decode/ClientFPS/Ping
	float m_statistics[6];
	uint64_t m_statisticsCount = 0;
};
