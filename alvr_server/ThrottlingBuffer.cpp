#include "ThrottlingBuffer.h"
#include "Utils.h"
#include "Logger.h"

ThrottlingBuffer::ThrottlingBuffer(const Bitrate &bitrate) : mBitrate(bitrate)
{
	// mWindow bytes can be sent at a time.
	mWindow = mBitrate.toBytes() * BURST_US / (1000 * 1000);
	if (mWindow < 2000) {
		// Ensure single packet can be sent
		mWindow = 2000;
	}
	Log(L"ThrottlingBuffer::ThrottlingBuffer(). Limit=%llu Mbps %llu bytes/slot Current=%llu", mBitrate.toMiBits(), mWindow, GetCounterUs());
}

ThrottlingBuffer::~ThrottlingBuffer()
{
}

void ThrottlingBuffer::Push(VideoFrame *buf, int len, uint64_t frameIndex)
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

bool ThrottlingBuffer::GetFirstBufferedFrame(uint64_t *videoFrameIndex)
{
	IPCCriticalSectionLock lock(mCS);
	if (mQueue.empty()) {
		return false;
	}

	auto videoFrame = (VideoFrame *)mQueue.front().buf.get();
	*videoFrameIndex = videoFrame->videoFrameIndex;
	return true;
}

bool ThrottlingBuffer::IsEmpty()
{
	IPCCriticalSectionLock lock(mCS);
	return mQueue.empty();
}

void ThrottlingBuffer::Clear()
{
	IPCCriticalSectionLock lock(mCS);
	mQueue.clear();
	mByteCount = 0;
	mBuffered = 0;
	mLastSent = 0;
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

	auto videoFrame = (VideoFrame *)mQueue.front().buf.get();
	Log(L"ThrottlingBuffer::CanSend(). %03llu.%03llu Check %llu <= %llu: %d Packet=%u(%u) VideoFrame=%llu Buffered=%llu Fillup=%llu", (current / 1000) % 1000, current % 1000
		, mByteCount, mWindow, mByteCount <= static_cast<int64_t>(mWindow), videoFrame->packetCounter, videoFrame->fecIndex, videoFrame->videoFrameIndex, mBuffered, fullup);
	if (mByteCount <= static_cast<int64_t>(mWindow)) {
		return true;
	}
	return false;
}
