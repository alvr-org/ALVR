#include "PacketBuffer.h"
#include "Utils.h"
#include "Logger.h"

PacketBuffer::PacketBuffer()
{
}

PacketBuffer::~PacketBuffer()
{
}

void PacketBuffer::Push(char *buf, int len)
{
	IPCCriticalSectionLock lock(mCS);
	SendBuffer buffer;
	buffer.buf.reset(new char[len]);
	buffer.len = len;
	memcpy(buffer.buf.get(), buf, len);
	mQueue.push_back(buffer);
}

bool PacketBuffer::Send(std::function<bool(char*, int)> sendFunc)
{
	IPCCriticalSectionLock lock(mCS);
	if (mQueue.empty()) {
		return false;
	}
	SendBuffer &buffer = mQueue.front();
	if (sendFunc(buffer.buf.get(), buffer.len)) {
		mQueue.pop_front();
		return true;
	}
	return false;
}

bool PacketBuffer::IsEmpty()
{
	IPCCriticalSectionLock lock(mCS);
	return mQueue.empty();
}

void PacketBuffer::Clear()
{
	IPCCriticalSectionLock lock(mCS);
	mQueue.clear();
}
