#pragma once

#include <stdint.h>
#include "ipctools.h"

class IDRScheduler
{
public:
	IDRScheduler();
	~IDRScheduler();

	void OnPacketLoss();

	void OnStreamStart();

	bool CheckIDRInsertion();
private:
	static const int MIN_IDR_FRAME_INTERVAL = 100 * 1000; // 100-milliseconds
	uint64_t m_insertIDRTime = 0;
	bool m_scheduled = false;
	IPCCriticalSection m_IDRCS;
};

