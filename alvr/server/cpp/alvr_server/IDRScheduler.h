#pragma once

#include <stdint.h>
#ifdef _WIN32
	#include "ipctools.h"
#else
	#include <mutex>
#endif
#include "Settings.h"

class IDRScheduler
{
public:
	IDRScheduler();
	~IDRScheduler();

	void OnPacketLoss();

	void OnStreamStart();
	void InsertIDR();

	bool CheckIDRInsertion();
private:
	static const int MIN_IDR_FRAME_INTERVAL = 100 * 1000; // 100-milliseconds
	static const int MIN_IDR_FRAME_INTERVAL_AGGRESSIVE = 5 * 1000; // 5-milliseconds (less than screen refresh interval)
	uint64_t m_insertIDRTime = 0;
	bool m_scheduled = false;
#ifdef _WIN32
	//FIXME: does it need to be IPC ?
	IPCCriticalSection m_IDRCS;
#else
	std::mutex m_mutex;
#endif
	int m_minIDRFrameInterval = MIN_IDR_FRAME_INTERVAL;
};

