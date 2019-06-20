#pragma once

#include <list>
#include <memory>
#include <functional>

#include "Bitrate.h"
#include "openvr-utils\ipctools.h"
#include <packet_types.h>

class ThrottlingBuffer
{
public:
	ThrottlingBuffer(const Bitrate &bitrate);
	~ThrottlingBuffer();

	void Push(VideoFrame *buf, int len, uint64_t frameIndex);
	bool Send(std::function<bool(char *, int)> sendFunc);

	bool GetFirstBufferedFrame(uint64_t *videoFrameIndex);

	bool IsEmpty();

	void Clear();
private:
	struct SendBuffer {
		std::shared_ptr<char> buf;
		int len;
		uint64_t frameIndex;

		SendBuffer() : buf(NULL, [](char *p) { delete[] p; }) {
		}
	};

	Bitrate mBitrate;
	uint64_t mBuffered = 0;
	std::list<SendBuffer> mQueue;
	IPCCriticalSection mCS;

	uint64_t mWindow;
	int64_t mByteCount = 0;
	uint64_t mLastSent = 0;

	// Permit burst sending for performance (or implementation) reason.
	// Maximum size we can send at a time is mBitrate * BurstTime.
	static const uint64_t BURST_US = 1000;

	bool CanSend(uint64_t current);
};

