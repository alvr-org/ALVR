#pragma once

#include <list>
#include <memory>
#include <functional>

#include "Bitrate.h"
#include "ipctools.h"

struct SendBuffer {
	std::shared_ptr<char> buf;
	int len;
	uint64_t frameIndex;

	SendBuffer() : buf(NULL, [](char *p) { delete[] p; }) {
	}
};

class ThrottlingBuffer
{
public:
	ThrottlingBuffer(const Bitrate &bitrate);
	~ThrottlingBuffer();

	void Push(char *buf, int len, uint64_t frameIndex);
	bool Send(std::function<bool(char *, int)> sendFunc);

	bool IsEmpty();
private:
	Bitrate mBitrate;
	uint64_t mBuffered = 0;
	std::list<SendBuffer> mQueue;
	IPCCriticalSection mCS;

	uint64_t mBytesPerSlot;
	uint64_t mByteCount = 0;
	uint64_t mCurrentTimeSlotUs = 0;
	// Windows has 990us ~ 1000us resolution of timer in my environment.
	static const uint64_t TIME_SLOT_US = 900;

	bool CanSend(uint64_t current);
};

