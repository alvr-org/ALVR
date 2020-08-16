#include "ThrottlingBuffer.h"
#include "Utils.h"
#include "Logger.h"

ThrottlingBuffer::ThrottlingBuffer(const Bitrate &bitrate) : mBitrate(bitrate)
{
	// mWindow bytes can be sent at a time.
	mWindow = mBitrate.toBytes() / (1000 * 1000 / BURST_US);
	if (mWindow < 2000) {
		// Ensure single packet can be sent
		mWindow = 2000;
	}
	LogDriver("ThrottlingBuffer::ThrottlingBuffer(). Limit=%llu Mbps %llu bytes/slot Current=%llu", mBitrate.toMiBits(), mWindow, GetCounterUs());
}

ThrottlingBuffer::~ThrottlingBuffer()
{
}

void ThrottlingBuffer::Push(char *buf, int len, uint64_t frameIndex)
{
	IPCCriticalSectionLock lock(mCS);
	SendBuffer buffer;
	buffer.buf.reset(new char[len]);
	buffer.len = len;
	buffer.frameIndex = frameIndex;
	memcpy(buffer.buf.get(), buf, len);
	mQueue.push_back(buffer);
	mBuffered += len;
}

bool ThrottlingBuffer::Send(std::function<bool(char*, int)> sendFunc)
{
	IPCCriticalSectionLock lock(mCS);
	uint64_t current = GetCounterUs();
	if (CanSend(current)) {
		SendBuffer &buffer = mQueue.front();
		if (sendFunc(buffer.buf.get(), buffer.len)) {
			mByteCount += buffer.len;
			mBuffered -= buffer.len;
			mQueue.pop_front();
			return true;
		}
	}
	return false;
}

bool ThrottlingBuffer::IsEmpty()
{
	IPCCriticalSectionLock lock(mCS);
	return mQueue.empty();
}

bool ThrottlingBuffer::CanSend(uint64_t current)
{
	if (mQueue.empty()) {
		return false;
	}

	if (mBitrate.toBits() == 0) {
		// No limit.
		return true;
	}

	int64_t fullup = static_cast<int64_t>(mBitrate.toBytes() * static_cast<double>(current - mLastSent) / 1000000.0);
	mByteCount -= fullup;
	if (mByteCount < 0) {
		mByteCount = 0;
	}

	mLastSent = current;

	Log("ThrottlingBuffer::CanSend(). %03llu.%03llu Check %llu <= %llu: %d Buffered=%llu Fillup=%llu", (current / 1000) % 1000, current % 1000
		, mByteCount, mWindow, mByteCount <= mWindow, mBuffered, fullup);
	if (mByteCount <= mWindow) {
		return true;
	}
	return false;
}
