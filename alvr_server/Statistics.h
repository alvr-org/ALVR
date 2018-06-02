#pragma once

#include <stdint.h>
#include <time.h>

class Statistics {
public:
	Statistics() {
		ResetAll();
		m_current = time(NULL);
	}

	void ResetAll() {
		m_packetsSentTotal = 0;
		m_packetsSentInSecond = 0;
		m_bitsSentTotal = 0;
		m_bitsSentInSecond = 0;
	}

	void CountPacket(int bytes) {
		time_t current = time(NULL);
		if (m_current != current) {
			m_current = current;
			m_packetsSentInSecondPrev = m_packetsSentInSecond;
			m_bitsSentInSecondPrev = m_bitsSentInSecond;
			m_packetsSentInSecond = 0;
			m_bitsSentInSecond = 0;
		}
		m_packetsSentTotal++;
		m_packetsSentInSecond++;
		m_bitsSentTotal += bytes * 8;
		m_bitsSentInSecond += bytes * 8;
	}

	uint64_t GetPacketsSentTotal() {
		return m_packetsSentTotal;
	}
	uint64_t GetPacketsSentInSecond() {
		return m_packetsSentInSecondPrev;
	}
	uint64_t GetBitsSentTotal() {
		return m_bitsSentTotal;
	}
	uint64_t GetBitsSentInSecond() {
		return m_bitsSentInSecondPrev;
	}
private:
	uint64_t m_packetsSentTotal;
	uint64_t m_packetsSentInSecond;
	uint64_t m_packetsSentInSecondPrev;
	uint64_t m_bitsSentTotal;
	uint64_t m_bitsSentInSecond;
	uint64_t m_bitsSentInSecondPrev;

	time_t m_current;
};