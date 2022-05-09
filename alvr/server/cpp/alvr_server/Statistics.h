#pragma once

#include <algorithm>
#include <stdint.h>
#include <time.h>
#include <chrono>

#include "Utils.h"
#include "Settings.h"
#include "Logger.h"

class Statistics {
public:
	Statistics() {
		ResetAll();
		m_current = time(NULL);
	}

	void ResetAll() {
		m_bitsSentInSecond = 0;
		m_bitsSentInSecondPrev = 0;

		m_framesInSecond = 0;
		m_framesPrevious = 0;

		m_sendLatency = 0;
	}

	void CountPacket(int bytes) {
		CheckAndResetSecond();

		m_bitsSentInSecond += bytes * 8;
	}

	void EncodeOutput(uint64_t latencyUs) {
		CheckAndResetSecond();

		m_framesInSecond++;
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
	uint64_t GetBitrate() {
		return m_bitrate;
	}

	bool CheckBitrateUpdated() {
		if (m_enableAdaptiveBitrate) {
			uint64_t latencyUs = m_sendLatency;
			if (latencyUs != 0) {
				if (latencyUs > m_adaptiveBitrateTarget + m_adaptiveBitrateThreshold) {
					if (m_bitrate < 5 + m_adaptiveBitrateDownRate)
						m_bitrate = 5;
					else
						m_bitrate -= m_adaptiveBitrateDownRate;
				} else if (latencyUs < m_adaptiveBitrateTarget - m_adaptiveBitrateThreshold) {
					if (m_bitrate > m_adaptiveBitrateMaximum - m_adaptiveBitrateDownRate)
						m_bitrate = m_adaptiveBitrateMaximum;
					else if (m_bitsSentInSecondPrev * 1e-6 > m_bitrate * m_adaptiveBitrateLightLoadThreshold * (m_framesPrevious == 0 ? m_refreshRate : m_framesPrevious) / m_refreshRate)
						m_bitrate += m_adaptiveBitrateUpRate;
				}
			}
			if (m_bitrateUpdated != m_bitrate) {
				m_bitrateUpdated = m_bitrate;
				return true;
			}
		}
		return false;
	}

	float m_hmdBattery;
	bool m_hmdPlugged;
	float m_leftControllerBattery;
	float m_rightControllerBattery;

	void ResetSecond() {
		m_bitsSentInSecondPrev = m_bitsSentInSecond;
		m_bitsSentInSecond = 0;

		m_framesPrevious = m_framesInSecond;
		m_framesInSecond = 0;

		if (m_adaptiveBitrateUseFrametime) {
			if (m_framesPrevious > 0) {
				m_adaptiveBitrateTarget = 1e6 / m_framesPrevious + m_adaptiveBitrateTargetOffset;
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

	uint64_t m_bitsSentInSecond;
	uint64_t m_bitsSentInSecondPrev;

	uint32_t m_framesInSecond;
	uint32_t m_framesPrevious;
	
	uint64_t m_sendLatency = 0;

	uint64_t m_bitrate = Settings::Instance().mEncodeBitrateMBs;
	uint64_t m_bitrateUpdated = Settings::Instance().mEncodeBitrateMBs;

	int64_t m_refreshRate = Settings::Instance().m_refreshRate;

	bool m_enableAdaptiveBitrate = Settings::Instance().m_enableAdaptiveBitrate;
	uint64_t m_adaptiveBitrateMaximum = Settings::Instance().m_adaptiveBitrateMaximum;
	uint64_t m_adaptiveBitrateTarget = Settings::Instance().m_adaptiveBitrateTarget;
	bool m_adaptiveBitrateUseFrametime = Settings::Instance().m_adaptiveBitrateUseFrametime;
	uint64_t m_adaptiveBitrateTargetMaximum = Settings::Instance().m_adaptiveBitrateTargetMaximum;
	int32_t m_adaptiveBitrateTargetOffset = Settings::Instance().m_adaptiveBitrateTargetOffset;
	uint64_t m_adaptiveBitrateThreshold = Settings::Instance().m_adaptiveBitrateThreshold;

	uint64_t m_adaptiveBitrateUpRate = Settings::Instance().m_adaptiveBitrateUpRate;
	uint64_t m_adaptiveBitrateDownRate = Settings::Instance().m_adaptiveBitrateDownRate;

	float m_adaptiveBitrateLightLoadThreshold = Settings::Instance().m_adaptiveBitrateLightLoadThreshold;

	time_t m_current;

};
