#include "ThrottlingBuffer.h"
#include "Utils.h"
#include "Logger.h"

ThrottlingBuffer::ThrottlingBuffer(const Bitrate &bitrate) : mBitrate(bitrate)
{
	mBytesPerSlot = mBitrate.toBytes() / (1000 * 1000 / TIME_SLOT_US);
	Log(L"ThrottlingBuffer::ThrottlingBuffer(). Limit=%llu Mbps %llu bytes/slot", mBitrate.toMiBits(), mBytesPerSlot);
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
	uint64_t current = GetTimestampUs();
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

	if (current - mCurrentTimeSlotUs > TIME_SLOT_US) {
		// New time slot.
		mByteCount = 0;
		mCurrentTimeSlotUs = current;
		Log(L"ThrottlingBuffer::CanSend(). New time slot.");
	}

	uint64_t BytesPerSlot = mBitrate.toBytes() / (1000 * 1000 / TIME_SLOT_US);
	int len = mQueue.front().len;
	Log(L"ThrottlingBuffer::CanSend(). Check %llu <= %llu: %d Buffered=%llu", len + mByteCount, BytesPerSlot, len + mByteCount <= BytesPerSlot, mBuffered);
	if (len + mByteCount <= BytesPerSlot) {
		return true;
	}
	return false;
}
