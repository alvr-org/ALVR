#pragma once

#include <algorithm>
#include <stdint.h>
#include <time.h>

class Statistics {
public:
	Statistics() {
		ResetAll();
		mCurrent = time(NULL);
	}

	void ResetAll() {
		mPacketsSentTotal = 0;
		mPacketsSentInSecond = 0;
		mPacketsSentInSecondPrev = 0;
		mBitsSentTotal = 0;
		mBitsSentInSecond = 0;
		mBitsSentInSecondPrev = 0;

		mFramesInSecond = 0;
		mFramesPrevious = 0;

		mEncodeLatencyTotalUs = 0;
		mEncodeLatencyMin = 0;
		mEncodeLatencyMax = 0;
		mEncodeSampleCount = 0;
		mEncodeLatencyAveragePrev = 0;
		mEncodeLatencyMinPrev = 0;
		mEncodeLatencyMaxPrev = 0;
	}

	void CountPacket(int bytes) {
		CheckAndResetSecond();

		mPacketsSentTotal++;
		mPacketsSentInSecond++;
		mBitsSentTotal += bytes * 8;
		mBitsSentInSecond += bytes * 8;
	}

	void EncodeOutput(uint64_t latencyUs) {
		CheckAndResetSecond();

		mFramesInSecond++;
		mEncodeLatencyTotalUs += latencyUs;
		mEncodeLatencyMin = std::min(latencyUs, mEncodeLatencyMin);
		mEncodeLatencyMax = std::max(latencyUs, mEncodeLatencyMax);
		mEncodeSampleCount++;
	}

	uint64_t GetPacketsSentTotal() {
		return mPacketsSentTotal;
	}
	uint64_t GetPacketsSentInSecond() {
		return mPacketsSentInSecondPrev;
	}
	uint64_t GetBitsSentTotal() {
		return mBitsSentTotal;
	}
	uint64_t GetBitsSentInSecond() {
		return mBitsSentInSecondPrev;
	}
	uint32_t GetFPS() {
		return mFramesPrevious;
	}
	uint64_t GetEncodeLatencyAverage() {
		return mEncodeLatencyAveragePrev;
	}
	uint64_t GetEncodeLatencyMin() {
		return mEncodeLatencyMinPrev;
	}
	uint64_t GetEncodeLatencyMax() {
		return mEncodeLatencyMaxPrev;
	}
private:
	void ResetSecond() {
		mPacketsSentInSecondPrev = mPacketsSentInSecond;
		mBitsSentInSecondPrev = mBitsSentInSecond;
		mPacketsSentInSecond = 0;
		mBitsSentInSecond = 0;

		mFramesPrevious = mFramesInSecond;
		mFramesInSecond = 0;

		mEncodeLatencyMinPrev = mEncodeLatencyMin;
		mEncodeLatencyMaxPrev = mEncodeLatencyMax;
		if (mEncodeSampleCount == 0) {
			mEncodeLatencyAveragePrev = 0;
		}else{
			mEncodeLatencyAveragePrev = mEncodeLatencyTotalUs / mEncodeSampleCount;
		}
		mEncodeLatencyTotalUs = 0;
		mEncodeSampleCount = 0;
		mEncodeLatencyMin = UINT64_MAX;
		mEncodeLatencyMax = 0;
	}

	void CheckAndResetSecond() {
		time_t current = time(NULL);
		if (mCurrent != current) {
			mCurrent = current;
			ResetSecond();
		}
	}

	uint64_t mPacketsSentTotal;
	uint64_t mPacketsSentInSecond;
	uint64_t mPacketsSentInSecondPrev;

	uint64_t mBitsSentTotal;
	uint64_t mBitsSentInSecond;
	uint64_t mBitsSentInSecondPrev;

	uint32_t mFramesInSecond;
	uint32_t mFramesPrevious;

	uint64_t mEncodeLatencyTotalUs;
	uint64_t mEncodeLatencyMin;
	uint64_t mEncodeLatencyMax;
	uint64_t mEncodeSampleCount;
	uint64_t mEncodeLatencyAveragePrev;
	uint64_t mEncodeLatencyMinPrev;
	uint64_t mEncodeLatencyMaxPrev;

	time_t mCurrent;
};