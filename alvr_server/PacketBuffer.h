#pragma once

#include <list>
#include <memory>
#include <functional>

#include "openvr-utils\ipctools.h"

class PacketBuffer
{
public:
	PacketBuffer();
	~PacketBuffer();

	void Push(char *buf, int len);
	bool Send(std::function<bool(char *, int)> sendFunc);

	bool IsEmpty();
	void Clear();
private:
	struct SendBuffer {
		std::shared_ptr<char> buf;
		int len;

		SendBuffer() : buf(NULL, [](char *p) { delete[] p; }) {
		}
	};

	std::list<SendBuffer> mQueue;
	IPCCriticalSection mCS;
};

